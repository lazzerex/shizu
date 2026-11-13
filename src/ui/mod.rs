use ratatui::Frame;
use ratatui::buffer::Buffer as TuiBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::anim::{MIN_HEIGHT, MIN_WIDTH};
use crate::buffer::{Buffer, CharIdx};
use crate::editor::{Editor, Focus, PromptField, PromptInfo};
use crate::syntax::HighlightSpan;
use crate::theme::Theme;
use crate::view::{display_col, gutter_width, layout_line};

mod sidebar;

const WELCOME_LINES: &[&str] = &[
    "        .-~~~-.",
    "  .-~~'         `~~-.",
    " (        shizu       )",
    "  `-.._          _..-'",
    "        `------'",
    "",
    "Ctrl+S save    Ctrl+Q quit    Ctrl+F find",
    "Ctrl+R replace    Ctrl+G go to line",
    "Ctrl+A select all    Alt+Up/Down move line",
    "Open a file: shizu path/to/file",
];

const WELCOME_BRANCH_LINES: usize = 5;

pub fn draw(frame: &mut Frame, editor: &Editor) {
    let area = frame.area();
    let sidebar = editor.sidebar_width();
    let bars = editor.tab_bar_height();
    let gutter = gutter_width(editor.buffer());
    let content_height = area.height.saturating_sub(1);
    let text_height = content_height.saturating_sub(bars);
    let text_width = area.width.saturating_sub(sidebar).saturating_sub(gutter);
    let content = Rect {
        x: area.x.saturating_add(sidebar),
        y: area.y.saturating_add(bars),
        width: area.width.saturating_sub(sidebar),
        height: text_height,
    };
    let buffer = frame.buffer_mut();
    buffer.set_style(area, editor.theme().background_style());
    if sidebar > 0 {
        let tree_area = Rect {
            x: area.x,
            y: area.y,
            width: sidebar,
            height: content_height,
        };
        sidebar::draw_tree(buffer, editor, tree_area);
    }
    if bars > 0 {
        sidebar::draw_tabs(buffer, editor, area, sidebar);
    }
    if editor.welcome_visible() {
        draw_welcome(buffer, editor, content, gutter, text_width, text_height);
    } else {
        draw_text(buffer, editor, content, gutter, text_width, text_height);
    }
    draw_petals(buffer, editor, content, gutter, text_width, text_height);
    draw_status(buffer, editor, area);
}

fn draw_welcome(
    target: &mut TuiBuffer,
    editor: &Editor,
    area: Rect,
    gutter: u16,
    text_width: u16,
    text_height: u16,
) {
    let rows = usize::from(text_height);
    let columns = usize::from(text_width);
    if rows == 0 || columns == 0 {
        return;
    }
    let start = rows.saturating_sub(WELCOME_LINES.len()) / 2;
    for (index, line) in WELCOME_LINES.iter().enumerate() {
        let row = start + index;
        if row >= rows {
            break;
        }
        let offset = columns.saturating_sub(line.chars().count()) / 2;
        let x = area
            .x
            .saturating_add(gutter)
            .saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
        let y = area
            .y
            .saturating_add(u16::try_from(row).unwrap_or(u16::MAX));
        let style = if index < WELCOME_BRANCH_LINES {
            editor.theme().petal_style()
        } else {
            editor.theme().line_number_style()
        };
        target.set_string(x, y, line, style);
    }
}

fn draw_petals(
    target: &mut TuiBuffer,
    editor: &Editor,
    area: Rect,
    gutter: u16,
    text_width: u16,
    text_height: u16,
) {
    if text_width < MIN_WIDTH || text_height < MIN_HEIGHT {
        return;
    }
    let style = editor.theme().petal_style();
    for petal in editor.petals() {
        if petal.column >= text_width
            || petal.row >= text_height
            || text_cell_occupied(editor, petal.column, petal.row, text_width, text_height)
        {
            continue;
        }
        let x = area.x.saturating_add(gutter).saturating_add(petal.column);
        let y = area.y.saturating_add(petal.row);
        let Some(cell) = target.cell_mut((x, y)) else {
            continue;
        };
        cell.set_symbol(&petal.glyph.to_string());
        cell.set_style(style);
    }
}

fn text_cell_occupied(
    editor: &Editor,
    column: u16,
    row: u16,
    text_width: u16,
    text_height: u16,
) -> bool {
    if editor.welcome_visible() {
        return welcome_cell_occupied(column, row, text_width, text_height);
    }
    let buffer = editor.buffer();
    let view = editor.view();
    let line = view.scroll_line() + usize::from(row);
    if line >= buffer.len_lines() {
        return false;
    }
    let occupied: usize = layout_line(&buffer.line_text(line), view.tab_width())
        .iter()
        .map(|cell| cell.width)
        .sum();
    if usize::from(column) + view.scroll_col() < occupied {
        return true;
    }
    let pos = buffer.line_col(view.cursor());
    if pos.line == line {
        let cursor_column = display_col(buffer, view.cursor(), view.tab_width());
        if cursor_column.checked_sub(view.scroll_col()) == Some(usize::from(column)) {
            return true;
        }
    }
    false
}

fn welcome_cell_occupied(column: u16, row: u16, text_width: u16, text_height: u16) -> bool {
    let rows = usize::from(text_height);
    let columns = usize::from(text_width);
    let start = rows.saturating_sub(WELCOME_LINES.len()) / 2;
    let Some(index) = usize::from(row).checked_sub(start) else {
        return false;
    };
    let Some(line) = WELCOME_LINES.get(index) else {
        return false;
    };
    let offset = columns.saturating_sub(line.chars().count()) / 2;
    let column = usize::from(column);
    column >= offset && column < offset + line.chars().count()
}

fn draw_text(
    target: &mut TuiBuffer,
    editor: &Editor,
    area: Rect,
    gutter: u16,
    text_width: u16,
    text_height: u16,
) {
    let view = editor.view();
    let buffer = editor.buffer();
    let first = view.scroll_line();
    let scroll_col = view.scroll_col();
    let tab_width = view.tab_width();
    let selection = editor.selection();
    let matches = editor.matches();
    let theme = editor.theme();
    let text_x = area.x.saturating_add(gutter);
    let text_right = text_x.saturating_add(text_width);
    let gutter_style = theme.line_number_style();
    for row in 0..text_height {
        let line = first + usize::from(row);
        if line >= buffer.len_lines() {
            break;
        }
        let selected = selection_columns(buffer, selection, line);
        let highlighted = highlight_columns(buffer, matches, line);
        let spans = editor.highlight(line);
        let y = area.y.saturating_add(row);
        let number = format!(
            "{:>width$} ",
            line + 1,
            width = usize::from(gutter.saturating_sub(1))
        );
        target.set_string(area.x, y, &number, gutter_style);
        let mut col = 0;
        for cell in layout_line(&buffer.line_text(line), tab_width) {
            if col < scroll_col {
                col += cell.width;
                continue;
            }
            let offset = u16::try_from(col - scroll_col).unwrap_or(u16::MAX);
            let x = text_x.saturating_add(offset);
            if x >= text_right {
                break;
            }
            let mut style = syntax_style(theme, spans, cell.start);
            if selected.is_some_and(|(start, end)| cell.start >= start && cell.start < end) {
                style = style.add_modifier(Modifier::REVERSED);
            } else if highlighted
                .iter()
                .any(|&(start, end)| cell.start >= start && cell.start < end)
            {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            target.set_string(x, y, &cell.text, style);
            col += cell.width;
        }
    }
    draw_cursor(target, editor, area, gutter, text_width, text_height);
}

fn syntax_style(theme: &Theme, spans: &[HighlightSpan], column: usize) -> Style {
    spans
        .iter()
        .find(|span| span.start <= column && column < span.end)
        .map_or_else(|| theme.text(), |span| theme.style(span.scope))
}

fn highlight_columns(
    buffer: &Buffer,
    matches: &[(CharIdx, CharIdx)],
    line: usize,
) -> Vec<(usize, usize)> {
    let line_start = buffer.line_start(line);
    let line_end = buffer.line_end(line);
    matches
        .iter()
        .filter_map(|&(from, to)| {
            if to <= line_start || from >= line_end {
                return None;
            }
            let start = if from > line_start {
                buffer.line_col(from).col
            } else {
                0
            };
            let end = if to < line_end {
                buffer.line_col(to).col
            } else {
                line_end.get() - line_start.get()
            };
            (start < end).then_some((start, end))
        })
        .collect()
}

fn selection_columns(
    buffer: &Buffer,
    selection: Option<(CharIdx, CharIdx)>,
    line: usize,
) -> Option<(usize, usize)> {
    let (from, to) = selection?;
    let from_pos = buffer.line_col(from);
    let to_pos = buffer.line_col(to);
    if line < from_pos.line || line > to_pos.line {
        return None;
    }
    let start = if line == from_pos.line {
        from_pos.col
    } else {
        0
    };
    let end = if line == to_pos.line {
        to_pos.col
    } else {
        buffer.line_end(line).get() - buffer.line_start(line).get()
    };
    (start < end).then_some((start, end))
}

fn draw_cursor(
    target: &mut TuiBuffer,
    editor: &Editor,
    area: Rect,
    gutter: u16,
    text_width: u16,
    text_height: u16,
) {
    let view = editor.view();
    let buffer = editor.buffer();
    let pos = buffer.line_col(view.cursor());
    let first = view.scroll_line();
    if pos.line < first || pos.line >= first.saturating_add(usize::from(text_height)) {
        return;
    }
    let col = display_col(buffer, view.cursor(), view.tab_width());
    if col < view.scroll_col() {
        return;
    }
    let offset = col - view.scroll_col();
    if offset >= usize::from(text_width) {
        return;
    }
    let x = area
        .x
        .saturating_add(gutter)
        .saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
    let y = area
        .y
        .saturating_add(u16::try_from(pos.line - first).unwrap_or(u16::MAX));
    if let Some(cell) = target.cell_mut((x, y)) {
        let style = cell.style().add_modifier(Modifier::REVERSED);
        cell.set_style(style);
    }
}

fn draw_status(target: &mut TuiBuffer, editor: &Editor, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let y = area.y.saturating_add(area.height - 1);
    let text = if let Some(info) = editor.prompt_info() {
        prompt_text(&info)
    } else if editor.pending_quit() {
        " Unsaved changes. Press Ctrl+Q again to quit, Esc to cancel ".to_owned()
    } else if editor.pending_close() {
        " Unsaved changes. Press Ctrl+W again to close, Esc to cancel ".to_owned()
    } else {
        let buffer = editor.buffer();
        let view = editor.view();
        let name = buffer
            .path()
            .and_then(std::path::Path::file_name)
            .map_or_else(
                || "[No Name]".to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
        let dirty = if buffer.is_dirty() { "*" } else { "" };
        let pos = buffer.line_col(view.cursor());
        let col = display_col(buffer, view.cursor(), view.tab_width()) + 1;
        let message = editor.message().unwrap_or_default();
        format!(" {name} {dirty}  {}:{} {message} ", pos.line + 1, col)
    };
    let padded = format!("{text:<width$}", width = usize::from(area.width));
    target.set_string(area.x, y, &padded, editor.theme().status_bar());
}

fn prompt_text(info: &PromptInfo<'_>) -> String {
    match info.focus {
        Focus::Find => format!(" Find: {} ", info.query),
        Focus::Replace => {
            let find = if info.field == PromptField::Find {
                ">"
            } else {
                " "
            };
            let replace = if info.field == PromptField::Replace {
                ">"
            } else {
                " "
            };
            format!(
                " {find}Find: {}  {replace}Replace: {}  Tab switch  Ctrl+A all ",
                info.query, info.replacement
            )
        }
        Focus::GoToLine => format!(" Go to line: {} ", info.goto),
        Focus::Tree | Focus::Editor => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Instant;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::{Color, Modifier};

    use super::draw;
    use crate::anim::{IDLE_DELAY, Mode};
    use crate::buffer::{Buffer, CharIdx};
    use crate::command::Command;
    use crate::editor::Editor;
    use crate::test_support::TempDir;
    use crate::theme::{ColorDepth, Theme};

    fn render(editor: &Editor, width: u16, height: u16) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, editor)).unwrap();
        terminal
    }

    fn animated(text: &str, seed: u64) -> Editor {
        let mut editor = Editor::new(Buffer::from_text(text));
        editor.resize(40, 10);
        editor.set_animation_seed(seed);
        editor.set_animation_mode(Mode::Idle);
        let now = Instant::now();
        editor.note_input(now);
        assert!(editor.tick(now + IDLE_DELAY));
        editor
    }

    fn symbol(terminal: &Terminal<TestBackend>, x: u16, y: u16) -> String {
        terminal
            .backend()
            .buffer()
            .cell((x, y))
            .map(|cell| cell.symbol().to_owned())
            .unwrap_or_default()
    }

    fn row(terminal: &Terminal<TestBackend>, y: u16, width: u16) -> String {
        (0..width).map(|x| symbol(terminal, x, y)).collect()
    }

    fn numbered_lines(count: usize) -> String {
        (0..count)
            .map(|index| format!("line{index}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn draws_short_file_with_line_numbers_and_status() {
        let mut editor = Editor::new(Buffer::from_text("abc\ndef"));
        editor.resize(20, 5);
        let terminal = render(&editor, 20, 5);

        assert_eq!(symbol(&terminal, 0, 0), "1");
        assert_eq!(symbol(&terminal, 2, 0), "a");
        assert_eq!(symbol(&terminal, 3, 0), "b");
        assert_eq!(symbol(&terminal, 0, 1), "2");
        assert_eq!(symbol(&terminal, 2, 1), "d");
        assert!(
            terminal
                .backend()
                .buffer()
                .cell((2, 0))
                .map(ratatui::buffer::Cell::style)
                .is_some_and(|style| style.add_modifier.contains(Modifier::REVERSED))
        );
        assert!(row(&terminal, 4, 20).contains("[No Name]"));
    }

    #[test]
    fn clips_lines_beyond_viewport() {
        let mut editor = Editor::new(Buffer::from_text(&numbered_lines(10)));
        editor.resize(20, 4);
        let terminal = render(&editor, 20, 4);

        assert_eq!(row(&terminal, 0, 8), " 1 line0");
        assert_eq!(row(&terminal, 2, 8), " 3 line2");
        assert_eq!(symbol(&terminal, 0, 3), " ");
        assert!(row(&terminal, 3, 20).contains("1:1"));
    }

    #[test]
    fn draws_scrolled_view() {
        let mut editor = Editor::new(Buffer::from_text(&numbered_lines(10)));
        editor.resize(20, 4);
        for _ in 0..5 {
            editor.apply(Command::MoveDown { select: false });
        }
        let terminal = render(&editor, 20, 4);

        assert_eq!(editor.view().scroll_line(), 3);
        assert_eq!(row(&terminal, 0, 8), " 4 line3");
        assert_eq!(row(&terminal, 2, 8), " 6 line5");
        assert!(
            terminal
                .backend()
                .buffer()
                .cell((3, 2))
                .map(ratatui::buffer::Cell::style)
                .is_some_and(|style| style.add_modifier.contains(Modifier::REVERSED))
        );
    }

    #[test]
    fn expands_tabs() {
        let mut editor = Editor::new(Buffer::from_text("\tx"));
        editor.resize(20, 3);
        let terminal = render(&editor, 20, 3);

        assert_eq!(row(&terminal, 0, 7), "1     x");
    }

    #[test]
    fn draws_double_width_character() {
        let mut editor = Editor::new(Buffer::from_text("あb"));
        editor.resize(20, 3);
        let terminal = render(&editor, 20, 3);

        assert_eq!(symbol(&terminal, 2, 0), "あ");
        assert_eq!(symbol(&terminal, 3, 0), " ");
        assert_eq!(symbol(&terminal, 4, 0), "b");
    }

    #[test]
    fn draws_horizontally_scrolled_line() {
        let mut editor = Editor::new(Buffer::from_text("abcdefghij"));
        editor.resize(10, 3);
        for _ in 0..9 {
            editor.apply(Command::MoveRight { select: false });
        }
        let terminal = render(&editor, 10, 3);

        assert_eq!(editor.view().scroll_col(), 2);
        assert_eq!(symbol(&terminal, 2, 0), "c");
        assert_eq!(symbol(&terminal, 9, 0), "j");
    }

    #[test]
    fn status_bar_shows_file_name_dirty_marker_and_position() {
        let dir = TempDir::new();
        let path = dir.join("note.txt");
        fs::write(&path, "abc\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(CharIdx::new(1), "x");
        let mut editor = Editor::new(buffer);
        editor.resize(30, 3);
        let terminal = render(&editor, 30, 3);

        let status = row(&terminal, 2, 30);
        assert!(status.contains("note.txt"));
        assert!(status.contains('*'));
        assert!(status.contains("1:1"));
    }

    #[test]
    fn status_bar_shows_quit_confirmation() {
        let mut editor = Editor::new(Buffer::from_text("abc"));
        editor.resize(60, 3);
        editor.apply(Command::Insert('x'));
        editor.apply(Command::Quit);
        let terminal = render(&editor, 60, 3);

        let status = row(&terminal, 2, 60);
        assert!(status.contains("Unsaved changes"));
        assert!(status.contains("Ctrl+Q again"));
        assert!(status.contains("Esc to cancel"));
    }

    #[test]
    fn status_bar_shows_save_error() {
        let dir = TempDir::new();
        let path = dir.join("readonly.txt");
        fs::write(&path, "abc\n").unwrap();
        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        editor.resize(80, 3);
        editor.apply(Command::Insert('x'));
        let original = fs::metadata(&path).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        editor.apply(Command::Save);
        let terminal = render(&editor, 80, 3);

        let status = row(&terminal, 2, 80);
        assert!(status.contains("read-only"));
        fs::set_permissions(&path, original).unwrap();
    }

    #[test]
    fn typed_text_appears() {
        let mut editor = Editor::new(Buffer::from_text(""));
        editor.resize(20, 3);
        for character in "hi".chars() {
            editor.apply(Command::Insert(character));
        }
        let terminal = render(&editor, 20, 3);

        assert_eq!(row(&terminal, 0, 4), "1 hi");
    }

    fn reversed(terminal: &Terminal<TestBackend>, x: u16, y: u16) -> bool {
        terminal
            .backend()
            .buffer()
            .cell((x, y))
            .map(ratatui::buffer::Cell::style)
            .is_some_and(|style| style.add_modifier.contains(Modifier::REVERSED))
    }

    #[test]
    fn draws_selection_reversed() {
        let mut editor = Editor::new(Buffer::from_text("abcdef"));
        editor.resize(20, 4);
        for _ in 0..2 {
            editor.apply(Command::MoveRight { select: true });
        }
        let terminal = render(&editor, 20, 4);

        assert!(reversed(&terminal, 2, 0));
        assert!(reversed(&terminal, 3, 0));
        assert!(!reversed(&terminal, 6, 0));
    }

    #[test]
    fn draws_selection_across_lines() {
        let mut editor = Editor::new(Buffer::from_text("abc\ndef"));
        editor.resize(20, 4);
        editor.apply(Command::SelectAll);
        let terminal = render(&editor, 20, 4);

        assert!(reversed(&terminal, 2, 0));
        assert!(reversed(&terminal, 4, 0));
        assert!(reversed(&terminal, 2, 1));
        assert!(reversed(&terminal, 4, 1));
    }

    #[test]
    fn tiny_terminal_does_not_panic() {
        for (width, height) in [(1, 1), (2, 1), (1, 2)] {
            let mut editor = Editor::new(Buffer::from_text("abc"));
            editor.resize(width, height);
            let _terminal = render(&editor, width, height);
        }
    }

    fn type_prompt(editor: &mut Editor, text: &str) {
        for character in text.chars() {
            editor.apply(Command::PromptInsert(character));
        }
    }

    fn underlined(terminal: &Terminal<TestBackend>, x: u16, y: u16) -> bool {
        terminal
            .backend()
            .buffer()
            .cell((x, y))
            .map(ratatui::buffer::Cell::style)
            .is_some_and(|style| style.add_modifier.contains(Modifier::UNDERLINED))
    }

    #[test]
    fn find_prompt_replaces_status_row() {
        let mut editor = Editor::new(Buffer::from_text("abc"));
        editor.resize(40, 4);
        editor.apply(Command::OpenFind);
        type_prompt(&mut editor, "b");
        let terminal = render(&editor, 40, 4);

        assert!(row(&terminal, 3, 40).contains("Find: b"));
    }

    #[test]
    fn replace_prompt_shows_both_fields() {
        let mut editor = Editor::new(Buffer::from_text("abc"));
        editor.resize(60, 4);
        editor.apply(Command::OpenReplace);
        type_prompt(&mut editor, "a");
        editor.apply(Command::PromptToggleField);
        type_prompt(&mut editor, "Z");
        let terminal = render(&editor, 60, 4);

        let bottom = row(&terminal, 3, 60);
        assert!(bottom.contains("Find: a"));
        assert!(bottom.contains("Replace: Z"));
    }

    #[test]
    fn goto_prompt_shows_line_number() {
        let mut editor = Editor::new(Buffer::from_text("abc"));
        editor.resize(40, 4);
        editor.apply(Command::OpenGoToLine);
        type_prompt(&mut editor, "12");
        let terminal = render(&editor, 40, 4);

        assert!(row(&terminal, 3, 40).contains("Go to line: 12"));
    }

    #[test]
    fn matches_are_underlined() {
        let mut editor = Editor::new(Buffer::from_text("abcabc"));
        editor.resize(40, 4);
        editor.apply(Command::OpenFind);
        type_prompt(&mut editor, "abc");
        let terminal = render(&editor, 40, 4);

        assert!(underlined(&terminal, 3, 0));
        assert!(underlined(&terminal, 4, 0));
        assert!(underlined(&terminal, 6, 0));
        assert!(underlined(&terminal, 7, 0));
        assert!(!underlined(&terminal, 8, 0));
    }

    #[test]
    fn draws_syntax_colors_from_theme() {
        let dir = TempDir::new();
        let path = dir.join("sample.rs");
        fs::write(&path, "fn main() {}\n").expect("write source");
        let mut editor = Editor::new(Buffer::open(&path).expect("open source"));
        editor.resize(40, 4);
        editor.set_theme(Theme::sakura(ColorDepth::TrueColor));
        let terminal = render(&editor, 40, 4);

        let style = terminal
            .backend()
            .buffer()
            .cell((2, 0))
            .map(ratatui::buffer::Cell::style)
            .expect("keyword cell");
        assert_eq!(style.fg, Some(Color::Rgb(0xf4, 0xa6, 0xc0)));
    }

    #[test]
    fn switching_theme_changes_styles() {
        let mut editor = Editor::new(Buffer::from_text("plain"));
        editor.resize(20, 4);
        editor.set_theme(Theme::sakura(ColorDepth::TrueColor));
        let dark = render(&editor, 20, 4);
        editor.set_theme(Theme::sakura_light(ColorDepth::TrueColor));
        let light = render(&editor, 20, 4);

        let color = |terminal: &Terminal<TestBackend>| {
            terminal
                .backend()
                .buffer()
                .cell((2, 0))
                .map(ratatui::buffer::Cell::style)
                .and_then(|style| style.fg)
        };
        assert_ne!(color(&dark), color(&light));
    }

    #[test]
    fn configured_tab_width_changes_layout() {
        let mut editor = Editor::new(Buffer::from_text("\tx"));
        editor.resize(20, 4);
        editor.set_tab_width(2);
        let terminal = render(&editor, 20, 4);

        assert_eq!(symbol(&terminal, 4, 0), "x");
    }

    #[test]
    fn petals_never_overwrite_text_or_chrome() {
        let editor = animated("abc\ndef", 3);
        let terminal = render(&editor, 40, 10);

        assert_eq!(symbol(&terminal, 2, 0), "a");
        assert_eq!(symbol(&terminal, 3, 0), "b");
        assert_eq!(symbol(&terminal, 4, 0), "c");
        assert_eq!(symbol(&terminal, 2, 1), "d");
        assert_eq!(symbol(&terminal, 0, 0), "1");
        assert!(row(&terminal, 9, 40).contains("[No Name]"));
    }

    #[test]
    fn petals_appear_on_empty_cells() {
        let editor = animated("x", 3);
        let terminal = render(&editor, 40, 10);
        let painted = editor
            .petals()
            .filter(|petal| petal.column < 38 && petal.row < 9)
            .filter(|petal| {
                symbol(&terminal, 2 + petal.column, petal.row) == petal.glyph.to_string()
            })
            .count();

        assert!(painted > 0);
    }

    #[test]
    fn petals_keep_selection_and_cursor_styles() {
        let mut editor = animated("abc", 6);
        editor.apply(Command::SelectAll);
        let terminal = render(&editor, 40, 10);

        assert_eq!(symbol(&terminal, 2, 0), "a");
        assert_eq!(symbol(&terminal, 4, 0), "c");
        assert!(reversed(&terminal, 2, 0));
        assert!(reversed(&terminal, 4, 0));
    }

    #[test]
    fn off_mode_renders_no_petals() {
        let mut editor = Editor::new(Buffer::from_text("abc"));
        editor.resize(40, 10);
        editor.set_animation_seed(3);
        editor.set_animation_mode(Mode::Off);
        let now = Instant::now();
        editor.note_input(now);
        assert!(!editor.tick(now + IDLE_DELAY));
        let terminal = render(&editor, 40, 10);

        assert_eq!(editor.petals().count(), 0);
        assert_eq!(symbol(&terminal, 2, 0), "a");
    }

    #[test]
    fn full_screen_of_text_renders_no_petals() {
        let text = (0..9)
            .map(|_| "x".repeat(40))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = Editor::new(Buffer::from_text(&text));
        editor.resize(40, 10);
        editor.set_animation_seed(4);
        editor.set_animation_mode(Mode::Idle);
        let now = Instant::now();
        editor.note_input(now);
        assert!(!editor.tick(now + IDLE_DELAY));
        let terminal = render(&editor, 40, 10);

        for y in 0..9 {
            for x in 2..40 {
                assert_eq!(symbol(&terminal, x, y), "x");
            }
        }
        assert_eq!(editor.petals().count(), 0);
    }

    #[test]
    fn welcome_screen_shows_branch_and_hints() {
        let mut editor = Editor::new(Buffer::new());
        editor.resize(60, 12);
        let terminal = render(&editor, 60, 12);
        let text: String = (0..11).map(|y| row(&terminal, y, 60)).collect();

        assert!(text.contains("shizu"));
        assert!(text.contains("Ctrl+S save"));

        editor.apply(Command::Insert('a'));
        let terminal = render(&editor, 60, 12);
        let text: String = (0..11).map(|y| row(&terminal, y, 60)).collect();
        assert!(!text.contains("Ctrl+S save"));
        assert_eq!(symbol(&terminal, 2, 0), "a");
    }

    fn editor_with_files() -> (TempDir, Editor) {
        let dir = TempDir::new();
        fs::write(dir.join("a.txt"), "alpha\n").expect("write a");
        fs::write(dir.join("b.txt"), "beta\n").expect("write b");
        let mut editor = Editor::new(Buffer::open(dir.join("a.txt")).expect("open a"));
        editor.resize(60, 8);
        (dir, editor)
    }

    #[test]
    fn single_tab_renders_no_tab_bar() {
        let (_dir, mut editor) = editor_with_files();
        editor.resize(60, 8);
        let terminal = render(&editor, 60, 8);

        assert_eq!(row(&terminal, 0, 6), "1 alph");
    }

    #[test]
    fn tab_bar_lists_open_buffers_and_marks_the_active_tab() {
        let (dir, mut editor) = editor_with_files();
        editor.open_path(dir.join("b.txt"));

        let terminal = render(&editor, 60, 8);

        let tabs = row(&terminal, 0, 60);
        assert!(tabs.contains("a.txt"));
        assert!(tabs.contains("b.txt"));
        assert_eq!(row(&terminal, 1, 6), "1 beta");
    }

    #[test]
    fn sidebar_renders_entries_and_shifts_the_text_area() {
        let (_dir, mut editor) = editor_with_files();
        editor.apply(Command::ToggleSidebar);

        let terminal = render(&editor, 60, 8);

        assert!(row(&terminal, 0, 16).contains("a.txt"));
        assert!(row(&terminal, 1, 16).contains("b.txt"));
        assert_eq!(symbol(&terminal, 18, 0), "a");
    }

    #[test]
    fn opened_empty_file_does_not_show_welcome() {
        let dir = TempDir::new();
        let path = dir.join("empty.txt");
        fs::write(&path, "").expect("write empty file");
        let mut editor = Editor::new(Buffer::open(&path).expect("open empty file"));
        editor.resize(60, 12);
        let terminal = render(&editor, 60, 12);
        let text: String = (0..12).map(|y| row(&terminal, y, 60)).collect();

        assert!(!text.contains("Ctrl+S save"));
        assert!(text.contains("empty.txt"));
    }
}
