use crate::buffer::{Buffer, BufferId, CharIdx, Transaction};
use crate::command::Command;
use crate::search;
use crate::view::{View, char_idx_at_display_col, display_col, gutter_width};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Editor,
    Find,
    Replace,
    GoToLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptField {
    Find,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptInfo<'a> {
    pub focus: Focus,
    pub query: &'a str,
    pub replacement: &'a str,
    pub goto: &'a str,
    pub field: PromptField,
}

#[derive(Debug)]
struct Prompt {
    query: String,
    replacement: String,
    goto: String,
    field: PromptField,
    origin: CharIdx,
    matches: Vec<(CharIdx, CharIdx)>,
    current: Option<usize>,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            query: String::new(),
            replacement: String::new(),
            goto: String::new(),
            field: PromptField::Find,
            origin: CharIdx::new(0),
            matches: Vec::new(),
            current: None,
        }
    }
}

#[derive(Debug)]
pub struct Editor {
    buffers: Vec<Buffer>,
    view: View,
    viewport: Viewport,
    quit: bool,
    pending_quit: bool,
    message: Option<String>,
    clipboard: String,
    pending_clipboard: Option<String>,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    focus: Focus,
    prompt: Prompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug)]
struct Edit {
    transaction: Transaction,
    before: CharIdx,
    after: CharIdx,
}

impl Editor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffers: vec![buffer],
            view: View::new(BufferId::new(0)),
            viewport: Viewport {
                width: 80,
                height: 24,
            },
            quit: false,
            pending_quit: false,
            message: None,
            clipboard: String::new(),
            pending_clipboard: None,
            undo: Vec::new(),
            redo: Vec::new(),
            focus: Focus::Editor,
            prompt: Prompt::default(),
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffers[self.view.buffer_id().index()]
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    pub fn selection(&self) -> Option<(CharIdx, CharIdx)> {
        self.view.selection()
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn matches(&self) -> &[(CharIdx, CharIdx)] {
        &self.prompt.matches
    }

    pub fn prompt_info(&self) -> Option<PromptInfo<'_>> {
        (self.focus != Focus::Editor).then_some(PromptInfo {
            focus: self.focus,
            query: &self.prompt.query,
            replacement: &self.prompt.replacement,
            goto: &self.prompt.goto,
            field: self.prompt.field,
        })
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn pending_quit(&self) -> bool {
        self.pending_quit
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    pub fn set_clipboard_text(&mut self, text: String) {
        self.clipboard = text;
    }

    pub fn take_clipboard_request(&mut self) -> Option<String> {
        self.pending_clipboard.take()
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.viewport = Viewport { width, height };
        self.ensure_visible();
    }

    pub fn set_cursor(&mut self, at: CharIdx) {
        self.view.set_cursor(at);
        self.ensure_visible();
    }

    pub fn apply(&mut self, command: Command) -> bool {
        let before = self.snapshot();
        let revision = self.buffer().revision();
        if !matches!(command, Command::Quit) {
            self.pending_quit = false;
        }
        match command {
            Command::MoveLeft { select } => self.move_left(select),
            Command::MoveRight { select } => self.move_right(select),
            Command::MoveUp { select } => self.move_up(select),
            Command::MoveDown { select } => self.move_down(select),
            Command::Home { select } => self.home(select),
            Command::End { select } => self.end(select),
            Command::PageUp { select } => self.page_up(select),
            Command::PageDown { select } => self.page_down(select),
            Command::Insert(character) => self.insert(character),
            Command::Backspace => self.backspace(),
            Command::Delete => self.delete(),
            Command::SelectAll => self.select_all(),
            Command::Cut => self.cut(),
            Command::Copy => self.copy(),
            Command::Paste => self.paste(),
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
            Command::MoveLineUp => self.move_line(-1),
            Command::MoveLineDown => self.move_line(1),
            Command::Save => self.save(),
            Command::Quit => self.request_quit(),
            Command::Cancel => self.cancel(),
            Command::OpenFind => self.open_prompt(Focus::Find),
            Command::OpenReplace => self.open_prompt(Focus::Replace),
            Command::OpenGoToLine => self.open_prompt(Focus::GoToLine),
            Command::FindNext => self.find_step(1),
            Command::FindPrevious => self.find_step(-1),
            Command::PromptInsert(character) => self.prompt_insert(character),
            Command::PromptBackspace => self.prompt_backspace(),
            Command::PromptNext => self.prompt_enter(),
            Command::PromptPrevious => self.prompt_previous(),
            Command::PromptToggleField => self.prompt_toggle_field(),
            Command::ReplaceAll => self.replace_all(),
            Command::Click { column, row } => self.click(column, row),
            Command::Drag { column, row } => self.drag(column, row),
            Command::ScrollLines(delta) => self.scroll_lines(delta),
        }
        if self.buffer().revision() != revision {
            self.prompt.matches.clear();
            self.prompt.current = None;
        }
        if !matches!(command, Command::ScrollLines(_)) {
            self.ensure_visible();
        }
        before != self.snapshot()
    }

    fn move_left(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.move_left(&self.buffers[index], select);
    }

    fn move_right(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.move_right(&self.buffers[index], select);
    }

    fn move_up(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.move_up(&self.buffers[index], select);
    }

    fn move_down(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.move_down(&self.buffers[index], select);
    }

    fn home(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.home(&self.buffers[index], select);
    }

    fn end(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.end(&self.buffers[index], select);
    }

    fn page_up(&mut self, select: bool) {
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        self.view.page_up(&self.buffers[index], height, select);
    }

    fn page_down(&mut self, select: bool) {
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        self.view.page_down(&self.buffers[index], height, select);
    }

    fn insert(&mut self, character: char) {
        let index = self.view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.view;
        let mut encoded = [0; 4];
        let text = character.encode_utf8(&mut encoded);
        let cursor = view.cursor();
        let (transaction, after) = if let Some((from, to)) = view.selection() {
            let transaction = buffer.replace(from, to, text);
            (transaction, CharIdx::new(from.get() + 1))
        } else {
            let transaction = buffer.insert(cursor, text);
            (transaction, buffer.next_grapheme(cursor))
        };
        view.set_cursor(after);
        record(&mut self.undo, &mut self.redo, transaction, cursor, after);
    }

    fn backspace(&mut self) {
        let index = self.view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.view;
        let cursor = view.cursor();
        let range = if let Some(range) = view.selection() {
            Some(range)
        } else {
            let previous = buffer.prev_grapheme(cursor);
            (previous != cursor).then_some((previous, cursor))
        };
        if let Some((from, to)) = range {
            let transaction = buffer.replace(from, to, "");
            view.set_cursor(from);
            record(&mut self.undo, &mut self.redo, transaction, cursor, from);
        }
    }

    fn delete(&mut self) {
        let index = self.view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.view;
        let cursor = view.cursor();
        let range = if let Some(range) = view.selection() {
            Some(range)
        } else {
            let next = buffer.next_grapheme(cursor);
            (next != cursor).then_some((cursor, next))
        };
        if let Some((from, to)) = range {
            let transaction = buffer.replace(from, to, "");
            view.set_cursor(from);
            record(&mut self.undo, &mut self.redo, transaction, cursor, from);
        }
    }

    fn select_all(&mut self) {
        let index = self.view.buffer_id().index();
        self.view.select_all(&self.buffers[index]);
    }

    fn cut(&mut self) {
        let index = self.view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.view;
        let Some((from, to)) = view.selection() else {
            return;
        };
        self.clipboard = buffer.text_range(from, to);
        self.pending_clipboard = Some(self.clipboard.clone());
        let cursor = view.cursor();
        let transaction = buffer.replace(from, to, "");
        view.set_cursor(from);
        record(&mut self.undo, &mut self.redo, transaction, cursor, from);
    }

    fn copy(&mut self) {
        let index = self.view.buffer_id().index();
        let buffer = &self.buffers[index];
        let Some((from, to)) = self.view.selection() else {
            return;
        };
        self.clipboard = buffer.text_range(from, to);
        self.pending_clipboard = Some(self.clipboard.clone());
    }

    fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        let text = self.clipboard.clone();
        let index = self.view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.view;
        let cursor = view.cursor();
        let (transaction, after) = if let Some((from, to)) = view.selection() {
            let transaction = buffer.replace(from, to, &text);
            (transaction, CharIdx::new(from.get() + text.chars().count()))
        } else {
            let transaction = buffer.insert(cursor, &text);
            (
                transaction,
                CharIdx::new(cursor.get() + text.chars().count()),
            )
        };
        view.set_cursor(after);
        record(&mut self.undo, &mut self.redo, transaction, cursor, after);
    }

    fn undo(&mut self) {
        let Some(edit) = self.undo.pop() else {
            return;
        };
        let index = self.view.buffer_id().index();
        self.buffers[index].revert(&edit.transaction);
        self.view.set_cursor(edit.before);
        self.redo.push(edit);
    }

    fn redo(&mut self) {
        let Some(edit) = self.redo.pop() else {
            return;
        };
        let index = self.view.buffer_id().index();
        self.buffers[index].apply(&edit.transaction);
        self.view.set_cursor(edit.after);
        self.undo.push(edit);
    }

    fn move_line(&mut self, delta: isize) {
        let index = self.view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.view;
        if let Some((transaction, cursor, after)) = move_line(buffer, view, delta) {
            record(&mut self.undo, &mut self.redo, transaction, cursor, after);
        }
    }

    fn save(&mut self) {
        let index = self.view.buffer_id().index();
        match self.buffers[index].save() {
            Ok(()) => self.message = None,
            Err(error) => self.message = Some(error.to_string()),
        }
    }

    fn request_quit(&mut self) {
        let index = self.view.buffer_id().index();
        if self.buffers[index].is_dirty() && !self.pending_quit {
            self.pending_quit = true;
        } else {
            self.quit = true;
        }
    }

    fn cancel(&mut self) {
        if self.focus != Focus::Editor {
            self.focus = Focus::Editor;
        }
    }

    fn open_prompt(&mut self, focus: Focus) {
        self.focus = focus;
        self.prompt.field = PromptField::Find;
        match focus {
            Focus::Find | Focus::Replace => {
                self.prompt.query.clear();
                self.prompt.replacement.clear();
                self.prompt.matches.clear();
                self.prompt.current = None;
                self.prompt.origin = self.view.cursor();
            }
            Focus::GoToLine => self.prompt.goto.clear(),
            Focus::Editor => {}
        }
    }

    fn refresh_matches(&mut self) {
        let index = self.view.buffer_id().index();
        let text = self.buffers[index].text();
        let starts = search::find_all(&text, &self.prompt.query);
        let len = self.prompt.query.chars().count();
        self.prompt.matches = starts
            .iter()
            .map(|&start| (CharIdx::new(start), CharIdx::new(start + len)))
            .collect();
        self.prompt.current = if self.prompt.matches.is_empty() {
            None
        } else {
            Some(
                self.prompt
                    .matches
                    .iter()
                    .position(|(from, _)| *from >= self.prompt.origin)
                    .unwrap_or(0),
            )
        };
    }

    fn reveal_match(&mut self) {
        if let Some(current) = self.prompt.current {
            let at = self.prompt.matches[current].0;
            self.view.set_cursor(at);
        }
    }

    fn find_step(&mut self, delta: isize) {
        if self.prompt.query.is_empty() {
            return;
        }
        if self.prompt.matches.is_empty() {
            self.prompt.origin = self.view.cursor();
            self.refresh_matches();
        }
        let len = self.prompt.matches.len();
        if len == 0 {
            self.message = Some(format!("no matches for \"{}\"", self.prompt.query));
            return;
        }
        let current = match self.prompt.current {
            Some(current) => {
                let step = delta.unsigned_abs() % len;
                if delta < 0 {
                    (current + len - step) % len
                } else {
                    (current + step) % len
                }
            }
            None if delta < 0 => len - 1,
            None => 0,
        };
        self.prompt.current = Some(current);
        let at = self.prompt.matches[current].0;
        self.view.set_cursor(at);
        self.message = None;
    }

    fn prompt_insert(&mut self, character: char) {
        match self.focus {
            Focus::Find => {
                self.prompt.query.push(character);
                self.refresh_matches();
                self.reveal_match();
            }
            Focus::Replace => match self.prompt.field {
                PromptField::Find => {
                    self.prompt.query.push(character);
                    self.refresh_matches();
                    self.reveal_match();
                }
                PromptField::Replace => self.prompt.replacement.push(character),
            },
            Focus::GoToLine => self.prompt.goto.push(character),
            Focus::Editor => {}
        }
    }

    fn prompt_backspace(&mut self) {
        match self.focus {
            Focus::Find => {
                self.prompt.query.pop();
                self.refresh_matches();
                self.reveal_match();
            }
            Focus::Replace => match self.prompt.field {
                PromptField::Find => {
                    self.prompt.query.pop();
                    self.refresh_matches();
                    self.reveal_match();
                }
                PromptField::Replace => {
                    self.prompt.replacement.pop();
                }
            },
            Focus::GoToLine => {
                self.prompt.goto.pop();
            }
            Focus::Editor => {}
        }
    }

    fn prompt_enter(&mut self) {
        match self.focus {
            Focus::Find | Focus::Replace => self.find_step(1),
            Focus::GoToLine => self.goto_line(),
            Focus::Editor => {}
        }
    }

    fn prompt_previous(&mut self) {
        if matches!(self.focus, Focus::Find | Focus::Replace) {
            self.find_step(-1);
        }
    }

    fn prompt_toggle_field(&mut self) {
        if self.focus == Focus::Replace {
            self.prompt.field = match self.prompt.field {
                PromptField::Find => PromptField::Replace,
                PromptField::Replace => PromptField::Find,
            };
        }
    }

    fn replace_all(&mut self) {
        if !matches!(self.focus, Focus::Find | Focus::Replace) || self.prompt.query.is_empty() {
            return;
        }
        if self.prompt.matches.is_empty() {
            self.prompt.origin = self.view.cursor();
            self.refresh_matches();
        }
        let matches: Vec<(CharIdx, CharIdx)> = self.prompt.matches.iter().copied().fold(
            Vec::new(),
            |mut kept: Vec<(CharIdx, CharIdx)>, candidate| {
                if kept.last().is_none_or(|previous| candidate.0 >= previous.1) {
                    kept.push(candidate);
                }
                kept
            },
        );
        let count = matches.len();
        if count == 0 {
            self.message = Some(format!("no matches for \"{}\"", self.prompt.query));
            return;
        }
        let replacement = self.prompt.replacement.clone();
        let index = self.view.buffer_id().index();
        let before = self.view.cursor();
        let transaction = self.buffers[index].replace_matches(&matches, &replacement);
        let after = matches[0].0;
        self.view.set_cursor(after);
        record(&mut self.undo, &mut self.redo, transaction, before, after);
        self.focus = Focus::Editor;
        self.prompt.matches.clear();
        self.prompt.current = None;
        self.message = Some(format!("replaced {count} matches"));
    }

    fn goto_line(&mut self) {
        let Ok(number) = self.prompt.goto.trim().parse::<usize>() else {
            self.message = Some("invalid line number".to_owned());
            return;
        };
        if number == 0 {
            self.message = Some("invalid line number".to_owned());
            return;
        }
        let index = self.view.buffer_id().index();
        let line = (number - 1).min(self.buffers[index].len_lines().saturating_sub(1));
        let at = self.buffers[index].line_start(line);
        self.view.set_cursor(at);
        self.focus = Focus::Editor;
        self.message = None;
    }

    fn click(&mut self, column: u16, row: u16) {
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        let at = char_at_screen(&self.buffers[index], &self.view, height, column, row);
        if let Some(at) = at {
            self.view.click(at);
        }
    }

    fn drag(&mut self, column: u16, row: u16) {
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        let at = char_at_screen(&self.buffers[index], &self.view, height, column, row);
        if let Some(at) = at {
            self.view.extend_to(at);
        }
    }

    fn scroll_lines(&mut self, delta: isize) {
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        let max = self.buffers[index].len_lines().saturating_sub(height);
        let next = self
            .view
            .scroll_line()
            .saturating_add_signed(delta)
            .min(max);
        self.view.set_scroll_line(next);
    }

    fn snapshot(&self) -> State {
        State {
            cursor: self.view.cursor(),
            scroll_line: self.view.scroll_line(),
            scroll_col: self.view.scroll_col(),
            revision: self.buffer().revision(),
            dirty: self.buffer().is_dirty(),
            pending_quit: self.pending_quit,
            message: self.message.clone(),
            focus: self.focus,
            query: self.prompt.query.clone(),
            replacement: self.prompt.replacement.clone(),
            goto: self.prompt.goto.clone(),
            field: self.prompt.field,
            current: self.prompt.current,
        }
    }

    fn text_width(&self) -> usize {
        usize::from(self.viewport.width).saturating_sub(usize::from(gutter_width(self.buffer())))
    }

    fn text_height(&self) -> usize {
        usize::from(self.viewport.height).saturating_sub(1)
    }

    fn ensure_visible(&mut self) {
        let width = self.text_width();
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        let buffer = &self.buffers[index];
        self.view.ensure_visible(buffer, width, height);
    }
}

#[derive(Debug, PartialEq, Eq)]
struct State {
    cursor: CharIdx,
    scroll_line: usize,
    scroll_col: usize,
    revision: u64,
    dirty: bool,
    pending_quit: bool,
    message: Option<String>,
    focus: Focus,
    query: String,
    replacement: String,
    goto: String,
    field: PromptField,
    current: Option<usize>,
}

fn record(
    undo: &mut Vec<Edit>,
    redo: &mut Vec<Edit>,
    transaction: Transaction,
    before: CharIdx,
    after: CharIdx,
) {
    if transaction.is_empty() {
        return;
    }
    undo.push(Edit {
        transaction,
        before,
        after,
    });
    redo.clear();
}

fn char_at_screen(
    buffer: &Buffer,
    view: &View,
    height: usize,
    column: u16,
    row: u16,
) -> Option<CharIdx> {
    let gutter = gutter_width(buffer);
    if column < gutter || usize::from(row) >= height {
        return None;
    }
    let line = view.scroll_line().saturating_add(usize::from(row));
    if line >= buffer.len_lines() {
        return None;
    }
    let display = usize::from(column - gutter).saturating_add(view.scroll_col());
    Some(char_idx_at_display_col(
        buffer,
        line,
        display,
        view.tab_width(),
    ))
}

fn move_line(
    buffer: &mut Buffer,
    view: &mut View,
    delta: isize,
) -> Option<(Transaction, CharIdx, CharIdx)> {
    let line = buffer.line_col(view.cursor()).line;
    let target = line.checked_add_signed(delta)?;
    if target >= buffer.len_lines() {
        return None;
    }
    let (first, second) = if delta < 0 {
        (target, line)
    } else {
        (line, target)
    };
    let start = buffer.line_start(first);
    let end = buffer.line_end(second);
    let text = buffer.text_range(start, end);
    let (head, tail) = text.split_once('\n')?;
    let swapped = format!("{tail}\n{head}");
    let cursor = view.cursor();
    let display = display_col(buffer, cursor, view.tab_width());
    let transaction = buffer.replace(start, end, &swapped);
    let after = char_idx_at_display_col(buffer, target, display, view.tab_width());
    view.set_cursor(after);
    Some((transaction, cursor, after))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Editor, Focus, PromptField};
    use crate::buffer::{Buffer, CharIdx, LineCol};
    use crate::command::Command;
    use crate::test_support::TempDir;

    fn editor(text: &str) -> Editor {
        let mut editor = Editor::new(Buffer::from_text(text));
        editor.resize(40, 10);
        editor
    }

    fn at(editor: &Editor) -> LineCol {
        editor.buffer().line_col(editor.view().cursor())
    }

    fn numbered(count: usize) -> String {
        (0..count)
            .map(|index| index.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn select(editor: &mut Editor, count: usize) {
        for _ in 0..count {
            editor.apply(Command::MoveRight { select: true });
        }
    }

    #[test]
    fn vertical_movement_keeps_goal_column() {
        let mut editor = editor("abcdef\nxy\nabcdef");
        editor.set_cursor(CharIdx::new(4));
        assert!(editor.apply(Command::MoveDown { select: false }));
        assert_eq!(at(&editor), LineCol { line: 1, col: 2 });
        assert!(editor.apply(Command::MoveDown { select: false }));
        assert_eq!(at(&editor), LineCol { line: 2, col: 4 });
        assert!(editor.apply(Command::MoveUp { select: false }));
        assert_eq!(at(&editor), LineCol { line: 1, col: 2 });
    }

    #[test]
    fn home_and_end_move_within_line() {
        let mut editor = editor("abc\ndef");
        editor.set_cursor(CharIdx::new(5));
        assert!(editor.apply(Command::Home { select: false }));
        assert_eq!(at(&editor), LineCol { line: 1, col: 0 });
        assert!(editor.apply(Command::End { select: false }));
        assert_eq!(at(&editor), LineCol { line: 1, col: 3 });
    }

    #[test]
    fn page_down_moves_by_view_height() {
        let mut editor = editor("0\n1\n2\n3\n4\n5\n6\n7\n8\n9");
        assert!(editor.apply(Command::PageDown { select: false }));
        assert_eq!(at(&editor).line, 9);
        assert!(editor.apply(Command::PageUp { select: false }));
        assert_eq!(at(&editor).line, 0);
    }

    #[test]
    fn vertical_movement_at_first_line_is_not_a_change() {
        let mut editor = editor("abc");
        assert!(!editor.apply(Command::MoveUp { select: false }));
    }

    #[test]
    fn scrolling_follows_cursor() {
        let mut editor = editor("0\n1\n2\n3\n4\n5\n6\n7\n8\n9");
        for _ in 0..9 {
            editor.apply(Command::MoveDown { select: false });
        }
        assert_eq!(editor.view().scroll_line(), 1);
    }

    #[test]
    fn horizontal_scroll_follows_cursor() {
        let mut editor = editor("abcdefghijklmnopqrstuvwxyz");
        editor.resize(10, 5);
        for _ in 0..25 {
            editor.apply(Command::MoveRight { select: false });
        }
        assert_eq!(editor.view().scroll_col(), 18);
    }

    #[test]
    fn typing_inserts_at_cursor_and_advances() {
        let mut editor = editor("ab");
        editor.set_cursor(CharIdx::new(1));
        assert!(editor.apply(Command::Insert('x')));
        assert_eq!(editor.buffer().text(), "axb");
        assert_eq!(at(&editor), LineCol { line: 0, col: 2 });
        assert!(editor.buffer().is_dirty());
    }

    #[test]
    fn enter_splits_line() {
        let mut editor = editor("ab");
        editor.set_cursor(CharIdx::new(1));
        assert!(editor.apply(Command::Insert('\n')));
        assert_eq!(editor.buffer().text(), "a\nb");
        assert_eq!(at(&editor), LineCol { line: 1, col: 0 });
    }

    #[test]
    fn tab_inserts_tab() {
        let mut editor = editor("");
        assert!(editor.apply(Command::Insert('\t')));
        assert_eq!(editor.buffer().text(), "\t");
    }

    #[test]
    fn backspace_deletes_previous_grapheme() {
        let mut editor = editor("a\u{301}b");
        editor.set_cursor(CharIdx::new(2));
        assert!(editor.apply(Command::Backspace));
        assert_eq!(editor.buffer().text(), "b");
        assert_eq!(editor.view().cursor(), CharIdx::new(0));
    }

    #[test]
    fn backspace_at_start_is_not_a_change() {
        let mut editor = editor("ab");
        assert!(!editor.apply(Command::Backspace));
    }

    #[test]
    fn backspace_crosses_line_boundary() {
        let mut editor = editor("a\nb");
        editor.set_cursor(CharIdx::new(2));
        assert!(editor.apply(Command::Backspace));
        assert_eq!(editor.buffer().text(), "ab");
        assert_eq!(editor.view().cursor(), CharIdx::new(1));
    }

    #[test]
    fn delete_removes_next_grapheme() {
        let mut editor = editor("abc");
        assert!(editor.apply(Command::Delete));
        assert_eq!(editor.buffer().text(), "bc");
        assert_eq!(editor.view().cursor(), CharIdx::new(0));
    }

    #[test]
    fn delete_at_end_is_not_a_change() {
        let mut editor = editor("abc");
        editor.set_cursor(CharIdx::new(3));
        assert!(!editor.apply(Command::Delete));
    }

    #[test]
    fn save_writes_file_and_clears_dirty() {
        let dir = TempDir::new();
        let path = dir.join("save.txt");
        fs::write(&path, "ab\n").unwrap();
        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        editor.resize(40, 10);
        editor.apply(Command::Insert('x'));
        assert!(editor.apply(Command::Save));
        assert_eq!(fs::read(&path).unwrap(), b"xab\n");
        assert!(!editor.buffer().is_dirty());
        assert_eq!(editor.message(), None);
    }

    #[test]
    fn save_failure_sets_message() {
        let dir = TempDir::new();
        let path = dir.join("readonly.txt");
        fs::write(&path, "ab\n").unwrap();
        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        editor.resize(40, 10);
        editor.apply(Command::Insert('x'));
        let original = fs::metadata(&path).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        editor.apply(Command::Save);
        assert!(
            editor
                .message()
                .is_some_and(|text| text.contains("read-only"))
        );
        assert!(editor.buffer().is_dirty());
        fs::set_permissions(&path, original).unwrap();
    }

    #[test]
    fn quit_on_clean_buffer_exits() {
        let mut editor = editor("abc");
        editor.apply(Command::Quit);
        assert!(editor.should_quit());
        assert!(!editor.pending_quit());
    }

    #[test]
    fn quit_with_unsaved_changes_needs_confirmation() {
        let mut editor = editor("abc");
        editor.apply(Command::Insert('x'));
        assert!(editor.apply(Command::Quit));
        assert!(!editor.should_quit());
        assert!(editor.pending_quit());
        editor.apply(Command::Quit);
        assert!(editor.should_quit());
    }

    #[test]
    fn cancel_clears_pending_quit() {
        let mut editor = editor("abc");
        editor.apply(Command::Insert('x'));
        editor.apply(Command::Quit);
        assert!(editor.pending_quit());
        assert!(editor.apply(Command::Cancel));
        assert!(!editor.pending_quit());
        assert!(!editor.should_quit());
    }

    #[test]
    fn other_commands_clear_pending_quit() {
        let mut editor = editor("abc");
        editor.apply(Command::Insert('x'));
        editor.apply(Command::Quit);
        editor.apply(Command::MoveLeft { select: false });
        assert!(!editor.pending_quit());
    }

    #[test]
    fn click_places_cursor() {
        let mut editor = editor("abc\ndef");
        editor.resize(40, 10);
        assert!(editor.apply(Command::Click { column: 2, row: 1 }));
        assert_eq!(at(&editor), LineCol { line: 1, col: 0 });
        assert!(editor.apply(Command::Click { column: 4, row: 0 }));
        assert_eq!(at(&editor), LineCol { line: 0, col: 2 });
    }

    #[test]
    fn click_past_last_line_does_not_move() {
        let mut editor = editor("abc");
        editor.set_cursor(CharIdx::new(1));
        assert!(!editor.apply(Command::Click { column: 5, row: 5 }));
        assert_eq!(editor.view().cursor(), CharIdx::new(1));
    }

    #[test]
    fn click_in_gutter_does_not_move() {
        let mut editor = editor("abc");
        editor.set_cursor(CharIdx::new(1));
        assert!(!editor.apply(Command::Click { column: 0, row: 0 }));
        assert_eq!(editor.view().cursor(), CharIdx::new(1));
    }

    #[test]
    fn wheel_scrolls_view_without_moving_cursor() {
        let mut editor = editor(&numbered(20));
        let cursor = editor.view().cursor();
        assert!(editor.apply(Command::ScrollLines(3)));
        assert_eq!(editor.view().scroll_line(), 3);
        assert_eq!(editor.view().cursor(), cursor);
    }

    #[test]
    fn wheel_scroll_clamps_to_last_page() {
        let mut editor = editor("0\n1\n2\n3\n4\n5\n6\n7\n8\n9");
        editor.apply(Command::ScrollLines(100));
        assert_eq!(editor.view().scroll_line(), 1);
        assert!(!editor.apply(Command::ScrollLines(100)));
    }

    #[test]
    fn wheel_scroll_up_stops_at_top() {
        let mut editor = editor("0\n1\n2\n3\n4\n5\n6\n7\n8\n9");
        assert!(!editor.apply(Command::ScrollLines(-3)));
        assert_eq!(editor.view().scroll_line(), 0);
    }

    #[test]
    fn shift_right_selects_text() {
        let mut editor = editor("abcdef");
        select(&mut editor, 2);
        assert_eq!(editor.selection(), Some((CharIdx::new(0), CharIdx::new(2))));
    }

    #[test]
    fn shift_left_shrinks_selection() {
        let mut editor = editor("abcdef");
        select(&mut editor, 3);
        editor.apply(Command::MoveLeft { select: true });
        assert_eq!(editor.selection(), Some((CharIdx::new(0), CharIdx::new(2))));
    }

    #[test]
    fn plain_movement_clears_selection() {
        let mut editor = editor("abcdef");
        select(&mut editor, 2);
        editor.apply(Command::MoveRight { select: false });
        assert_eq!(editor.selection(), None);
    }

    #[test]
    fn select_all_covers_buffer() {
        let mut editor = editor("abc\ndef");
        editor.apply(Command::SelectAll);
        assert_eq!(editor.selection(), Some((CharIdx::new(0), CharIdx::new(7))));
    }

    #[test]
    fn drag_extends_selection_from_click() {
        let mut editor = editor("abc\ndef");
        editor.apply(Command::Click { column: 2, row: 0 });
        editor.apply(Command::Drag { column: 4, row: 0 });
        assert_eq!(editor.selection(), Some((CharIdx::new(0), CharIdx::new(2))));
    }

    #[test]
    fn click_clears_selection() {
        let mut editor = editor("abc\ndef");
        select(&mut editor, 2);
        editor.apply(Command::Click { column: 5, row: 1 });
        assert_eq!(editor.selection(), None);
    }

    #[test]
    fn typing_replaces_selection() {
        let mut editor = editor("abc");
        select(&mut editor, 2);
        assert!(editor.apply(Command::Insert('X')));
        assert_eq!(editor.buffer().text(), "Xc");
        assert_eq!(editor.selection(), None);
        assert_eq!(editor.view().cursor(), CharIdx::new(1));
    }

    #[test]
    fn backspace_deletes_selection() {
        let mut editor = editor("abc");
        select(&mut editor, 2);
        assert!(editor.apply(Command::Backspace));
        assert_eq!(editor.buffer().text(), "c");
        assert_eq!(editor.view().cursor(), CharIdx::new(0));
    }

    #[test]
    fn delete_deletes_selection() {
        let mut editor = editor("abc");
        select(&mut editor, 2);
        assert!(editor.apply(Command::Delete));
        assert_eq!(editor.buffer().text(), "c");
    }

    #[test]
    fn cut_removes_selection_and_requests_clipboard() {
        let mut editor = editor("abc");
        select(&mut editor, 2);
        assert!(editor.apply(Command::Cut));
        assert_eq!(editor.buffer().text(), "c");
        assert_eq!(editor.view().cursor(), CharIdx::new(0));
        assert_eq!(editor.take_clipboard_request().as_deref(), Some("ab"));
        assert_eq!(editor.take_clipboard_request(), None);
    }

    #[test]
    fn copy_keeps_text_and_requests_clipboard() {
        let mut editor = editor("abc");
        select(&mut editor, 2);
        assert!(!editor.apply(Command::Copy));
        assert_eq!(editor.buffer().text(), "abc");
        assert_eq!(editor.take_clipboard_request().as_deref(), Some("ab"));
    }

    #[test]
    fn copy_without_selection_does_nothing() {
        let mut editor = editor("abc");
        assert!(!editor.apply(Command::Copy));
        assert_eq!(editor.take_clipboard_request(), None);
    }

    #[test]
    fn paste_inserts_register() {
        let mut editor = editor("abc");
        editor.set_clipboard_text("XY".to_owned());
        assert!(editor.apply(Command::Paste));
        assert_eq!(editor.buffer().text(), "XYabc");
        assert_eq!(editor.view().cursor(), CharIdx::new(2));
    }

    #[test]
    fn paste_replaces_selection() {
        let mut editor = editor("abc");
        editor.set_clipboard_text("XY".to_owned());
        select(&mut editor, 1);
        assert!(editor.apply(Command::Paste));
        assert_eq!(editor.buffer().text(), "XYbc");
    }

    #[test]
    fn undo_restores_text_and_cursor() {
        let mut editor = editor("abc");
        editor.set_cursor(CharIdx::new(3));
        editor.apply(Command::Insert('d'));
        assert_eq!(editor.buffer().text(), "abcd");
        assert!(editor.apply(Command::Undo));
        assert_eq!(editor.buffer().text(), "abc");
        assert_eq!(editor.view().cursor(), CharIdx::new(3));
    }

    #[test]
    fn redo_after_undo() {
        let mut editor = editor("abc");
        editor.set_cursor(CharIdx::new(3));
        editor.apply(Command::Insert('d'));
        editor.apply(Command::Undo);
        assert!(editor.apply(Command::Redo));
        assert_eq!(editor.buffer().text(), "abcd");
        assert_eq!(editor.view().cursor(), CharIdx::new(4));
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut editor = editor("abc");
        editor.set_cursor(CharIdx::new(3));
        editor.apply(Command::Insert('d'));
        editor.apply(Command::Undo);
        editor.apply(Command::Insert('x'));
        assert!(!editor.apply(Command::Redo));
        assert_eq!(editor.buffer().text(), "abcx");
    }

    #[test]
    fn undo_across_paste() {
        let mut editor = editor("abc");
        editor.set_clipboard_text("XY".to_owned());
        editor.apply(Command::Paste);
        assert_eq!(editor.buffer().text(), "XYabc");
        assert!(editor.apply(Command::Undo));
        assert_eq!(editor.buffer().text(), "abc");
        assert!(editor.apply(Command::Redo));
        assert_eq!(editor.buffer().text(), "XYabc");
    }

    #[test]
    fn undo_after_backspace() {
        let mut editor = editor("abc");
        editor.set_cursor(CharIdx::new(3));
        editor.apply(Command::Backspace);
        assert_eq!(editor.buffer().text(), "ab");
        editor.apply(Command::Undo);
        assert_eq!(editor.buffer().text(), "abc");
        assert_eq!(editor.view().cursor(), CharIdx::new(3));
    }

    #[test]
    fn undo_after_cut_restores_selection_text() {
        let mut editor = editor("abc");
        select(&mut editor, 2);
        editor.apply(Command::Cut);
        assert!(editor.apply(Command::Undo));
        assert_eq!(editor.buffer().text(), "abc");
        assert_eq!(editor.view().cursor(), CharIdx::new(2));
    }

    #[test]
    fn move_line_up_swaps_lines() {
        let mut editor = editor("a\nb\nc");
        editor.set_cursor(CharIdx::new(2));
        assert!(editor.apply(Command::MoveLineUp));
        assert_eq!(editor.buffer().text(), "b\na\nc");
        assert_eq!(at(&editor), LineCol { line: 0, col: 0 });
    }

    #[test]
    fn move_line_down_swaps_lines() {
        let mut editor = editor("a\nb\nc");
        assert!(editor.apply(Command::MoveLineDown));
        assert_eq!(editor.buffer().text(), "b\na\nc");
        assert_eq!(at(&editor), LineCol { line: 1, col: 0 });
    }

    #[test]
    fn move_line_up_at_first_line_is_not_a_change() {
        let mut editor = editor("a\nb");
        assert!(!editor.apply(Command::MoveLineUp));
        assert_eq!(editor.buffer().text(), "a\nb");
    }

    #[test]
    fn move_line_down_at_last_line_is_not_a_change() {
        let mut editor = editor("a\nb");
        editor.set_cursor(CharIdx::new(2));
        assert!(!editor.apply(Command::MoveLineDown));
        assert_eq!(editor.buffer().text(), "a\nb");
    }

    #[test]
    fn undo_after_move_line_restores_order() {
        let mut editor = editor("a\nb");
        editor.set_cursor(CharIdx::new(2));
        editor.apply(Command::MoveLineUp);
        assert!(editor.apply(Command::Undo));
        assert_eq!(editor.buffer().text(), "a\nb");
        assert_eq!(editor.view().cursor(), CharIdx::new(2));
    }

    #[test]
    fn undo_with_empty_history_is_not_a_change() {
        let mut editor = editor("abc");
        assert!(!editor.apply(Command::Undo));
        assert!(!editor.apply(Command::Redo));
    }

    fn type_prompt(editor: &mut Editor, text: &str) {
        for character in text.chars() {
            editor.apply(Command::PromptInsert(character));
        }
    }

    #[test]
    fn find_prompt_previews_first_match() {
        let mut editor = editor("one two one");
        assert!(editor.apply(Command::OpenFind));
        assert_eq!(editor.focus(), Focus::Find);
        type_prompt(&mut editor, "one");
        assert_eq!(editor.matches().len(), 2);
        assert_eq!(editor.view().cursor(), CharIdx::new(0));
        assert!(editor.apply(Command::PromptNext));
        assert_eq!(editor.view().cursor(), CharIdx::new(8));
        assert!(editor.apply(Command::PromptNext));
        assert_eq!(editor.view().cursor(), CharIdx::new(0));
        assert!(editor.apply(Command::PromptPrevious));
        assert_eq!(editor.view().cursor(), CharIdx::new(8));
    }

    #[test]
    fn find_prompt_without_matches_is_empty() {
        let mut editor = editor("abc");
        editor.apply(Command::OpenFind);
        type_prompt(&mut editor, "zzz");
        assert!(editor.matches().is_empty());
        assert_eq!(editor.focus(), Focus::Find);
        editor.apply(Command::PromptNext);
        assert_eq!(editor.view().cursor(), CharIdx::new(0));
    }

    #[test]
    fn find_prompt_reports_overlapping_matches() {
        let mut editor = editor("aaaa");
        editor.apply(Command::OpenFind);
        type_prompt(&mut editor, "aa");
        assert_eq!(editor.matches().len(), 3);
    }

    #[test]
    fn find_prompt_matches_across_lines() {
        let mut editor = editor("ab\ncd");
        editor.apply(Command::OpenFind);
        type_prompt(&mut editor, "b\nc");
        assert_eq!(editor.matches(), &[(CharIdx::new(1), CharIdx::new(4))]);
    }

    #[test]
    fn escape_closes_prompt_and_keeps_query() {
        let mut editor = editor("abc");
        editor.apply(Command::OpenFind);
        type_prompt(&mut editor, "b");
        assert!(editor.apply(Command::Cancel));
        assert_eq!(editor.focus(), Focus::Editor);
        assert!(editor.prompt_info().is_none());
        assert_eq!(editor.matches().len(), 1);
    }

    #[test]
    fn edits_clear_matches() {
        let mut editor = editor("abc");
        editor.apply(Command::OpenFind);
        type_prompt(&mut editor, "b");
        editor.apply(Command::Cancel);
        assert_eq!(editor.matches().len(), 1);
        editor.apply(Command::Insert('x'));
        assert!(editor.matches().is_empty());
    }

    #[test]
    fn replace_all_is_one_undo_step() {
        let mut editor = editor("one two one");
        editor.apply(Command::OpenReplace);
        type_prompt(&mut editor, "one");
        assert!(editor.apply(Command::PromptToggleField));
        type_prompt(&mut editor, "X");
        assert!(editor.apply(Command::ReplaceAll));
        assert_eq!(editor.buffer().text(), "X two X");
        assert_eq!(editor.focus(), Focus::Editor);
        assert!(
            editor
                .message()
                .is_some_and(|text| text.contains("2 matches"))
        );
        assert!(editor.apply(Command::Undo));
        assert_eq!(editor.buffer().text(), "one two one");
    }

    #[test]
    fn replace_all_skips_overlapping_matches() {
        let mut editor = editor("aaaa");
        editor.apply(Command::OpenReplace);
        type_prompt(&mut editor, "aa");
        assert_eq!(editor.matches().len(), 3);
        editor.apply(Command::PromptToggleField);
        type_prompt(&mut editor, "X");
        assert!(editor.apply(Command::ReplaceAll));
        assert_eq!(editor.buffer().text(), "XX");
        assert!(editor.apply(Command::Undo));
        assert_eq!(editor.buffer().text(), "aaaa");
    }

    #[test]
    fn replace_all_without_matches_keeps_prompt_open() {
        let mut editor = editor("abc");
        editor.apply(Command::OpenReplace);
        type_prompt(&mut editor, "zzz");
        editor.apply(Command::PromptToggleField);
        type_prompt(&mut editor, "X");
        assert!(editor.apply(Command::ReplaceAll));
        assert_eq!(editor.focus(), Focus::Replace);
        assert_eq!(editor.buffer().text(), "abc");
        assert!(
            editor
                .message()
                .is_some_and(|text| text.contains("no matches"))
        );
    }

    #[test]
    fn replace_toggle_field_routes_typing() {
        let mut editor = editor("abc");
        editor.apply(Command::OpenReplace);
        type_prompt(&mut editor, "a");
        editor.apply(Command::PromptToggleField);
        type_prompt(&mut editor, "Z");
        let info = editor.prompt_info().expect("prompt open");
        assert_eq!(info.query, "a");
        assert_eq!(info.replacement, "Z");
        assert_eq!(info.field, PromptField::Replace);
    }

    #[test]
    fn backspace_edits_active_prompt_field() {
        let mut editor = editor("abc");
        editor.apply(Command::OpenFind);
        type_prompt(&mut editor, "ab");
        assert!(editor.apply(Command::PromptBackspace));
        let info = editor.prompt_info().expect("prompt open");
        assert_eq!(info.query, "a");
    }

    #[test]
    fn go_to_line_moves_cursor_and_closes_prompt() {
        let mut editor = editor(&numbered(10));
        editor.apply(Command::OpenGoToLine);
        type_prompt(&mut editor, "5");
        assert!(editor.apply(Command::PromptNext));
        assert_eq!(at(&editor), LineCol { line: 4, col: 0 });
        assert_eq!(editor.focus(), Focus::Editor);
    }

    #[test]
    fn go_to_line_clamps_past_last_line() {
        let mut editor = editor(&numbered(3));
        editor.apply(Command::OpenGoToLine);
        type_prompt(&mut editor, "99");
        editor.apply(Command::PromptNext);
        assert_eq!(at(&editor), LineCol { line: 2, col: 0 });
    }

    #[test]
    fn go_to_line_rejects_invalid_number() {
        let mut editor = editor("abc");
        editor.set_cursor(CharIdx::new(1));
        editor.apply(Command::OpenGoToLine);
        type_prompt(&mut editor, "x");
        editor.apply(Command::PromptNext);
        assert_eq!(editor.focus(), Focus::GoToLine);
        assert_eq!(editor.view().cursor(), CharIdx::new(1));
        assert!(
            editor
                .message()
                .is_some_and(|text| text.contains("invalid line"))
        );
    }

    #[test]
    fn find_next_after_close_uses_retained_query() {
        let mut editor = editor("one two one");
        editor.apply(Command::OpenFind);
        type_prompt(&mut editor, "one");
        editor.apply(Command::Cancel);
        assert!(editor.apply(Command::FindNext));
        assert_eq!(editor.view().cursor(), CharIdx::new(8));
    }
}
