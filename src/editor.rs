use crate::buffer::{Buffer, BufferId, CharIdx};
use crate::command::Command;
use crate::view::{View, char_idx_at_display_col, gutter_width};

#[derive(Debug)]
pub struct Editor {
    buffers: Vec<Buffer>,
    view: View,
    viewport: Viewport,
    quit: bool,
    pending_quit: bool,
    message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub width: u16,
    pub height: u16,
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

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn pending_quit(&self) -> bool {
        self.pending_quit
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
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
        if !matches!(command, Command::Quit) {
            self.pending_quit = false;
        }
        let index = self.view.buffer_id().index();
        let height = self.text_height();
        let buffer = &mut self.buffers[index];
        let view = &mut self.view;
        match command {
            Command::MoveLeft => view.move_left(buffer),
            Command::MoveRight => view.move_right(buffer),
            Command::MoveUp => view.move_up(buffer),
            Command::MoveDown => view.move_down(buffer),
            Command::Home => view.home(buffer),
            Command::End => view.end(buffer),
            Command::PageUp => view.page_up(buffer, height),
            Command::PageDown => view.page_down(buffer, height),
            Command::Insert(character) => {
                let mut encoded = [0; 4];
                let text = character.encode_utf8(&mut encoded);
                let at = view.cursor();
                buffer.insert(at, text);
                view.set_cursor(buffer.next_grapheme(at));
            }
            Command::Backspace => {
                let at = view.cursor();
                let previous = buffer.prev_grapheme(at);
                if previous != at {
                    buffer.delete(previous, at.get() - previous.get());
                    view.set_cursor(previous);
                }
            }
            Command::Delete => {
                let at = view.cursor();
                let next = buffer.next_grapheme(at);
                if next != at {
                    buffer.delete(at, next.get() - at.get());
                    view.set_cursor(at);
                }
            }
            Command::Save => match buffer.save() {
                Ok(()) => self.message = None,
                Err(error) => self.message = Some(error.to_string()),
            },
            Command::Quit => {
                if buffer.is_dirty() && !self.pending_quit {
                    self.pending_quit = true;
                } else {
                    self.quit = true;
                }
            }
            Command::Cancel => {}
            Command::Click { column, row } => {
                let gutter = gutter_width(buffer);
                if column >= gutter && usize::from(row) < height {
                    let line = view.scroll_line().saturating_add(usize::from(row));
                    if line < buffer.len_lines() {
                        let display =
                            usize::from(column - gutter).saturating_add(view.scroll_col());
                        let at = char_idx_at_display_col(buffer, line, display, view.tab_width());
                        view.set_cursor(at);
                    }
                }
            }
            Command::ScrollLines(delta) => {
                let max = buffer.len_lines().saturating_sub(height);
                let next = view.scroll_line().saturating_add_signed(delta).min(max);
                view.set_scroll_line(next);
            }
        }
        if !matches!(command, Command::ScrollLines(_)) {
            self.ensure_visible();
        }
        before != self.snapshot()
    }

    fn snapshot(&self) -> State {
        State {
            cursor: self.view.cursor(),
            scroll_line: self.view.scroll_line(),
            scroll_col: self.view.scroll_col(),
            dirty: self.buffer().is_dirty(),
            pending_quit: self.pending_quit,
            message: self.message.clone(),
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
    dirty: bool,
    pending_quit: bool,
    message: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Editor;
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

    #[test]
    fn vertical_movement_keeps_goal_column() {
        let mut editor = editor("abcdef\nxy\nabcdef");
        editor.set_cursor(CharIdx::new(4));
        assert!(editor.apply(Command::MoveDown));
        assert_eq!(at(&editor), LineCol { line: 1, col: 2 });
        assert!(editor.apply(Command::MoveDown));
        assert_eq!(at(&editor), LineCol { line: 2, col: 4 });
        assert!(editor.apply(Command::MoveUp));
        assert_eq!(at(&editor), LineCol { line: 1, col: 2 });
    }

    #[test]
    fn home_and_end_move_within_line() {
        let mut editor = editor("abc\ndef");
        editor.set_cursor(CharIdx::new(5));
        assert!(editor.apply(Command::Home));
        assert_eq!(at(&editor), LineCol { line: 1, col: 0 });
        assert!(editor.apply(Command::End));
        assert_eq!(at(&editor), LineCol { line: 1, col: 3 });
    }

    #[test]
    fn page_down_moves_by_view_height() {
        let mut editor = editor("0\n1\n2\n3\n4\n5\n6\n7\n8\n9");
        assert!(editor.apply(Command::PageDown));
        assert_eq!(at(&editor).line, 9);
        assert!(editor.apply(Command::PageUp));
        assert_eq!(at(&editor).line, 0);
    }

    #[test]
    fn vertical_movement_at_first_line_is_not_a_change() {
        let mut editor = editor("abc");
        assert!(!editor.apply(Command::MoveUp));
    }

    #[test]
    fn scrolling_follows_cursor() {
        let mut editor = editor("0\n1\n2\n3\n4\n5\n6\n7\n8\n9");
        for _ in 0..9 {
            editor.apply(Command::MoveDown);
        }
        assert_eq!(editor.view().scroll_line(), 1);
    }

    #[test]
    fn horizontal_scroll_follows_cursor() {
        let mut editor = editor("abcdefghijklmnopqrstuvwxyz");
        editor.resize(10, 5);
        for _ in 0..25 {
            editor.apply(Command::MoveRight);
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
        editor.apply(Command::MoveLeft);
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
}
