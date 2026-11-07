use ratatui::Frame;
use ratatui::buffer::Buffer as TuiBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::editor::Editor;
use crate::view::{display_col, gutter_width, layout_line};

pub fn draw(frame: &mut Frame, editor: &Editor) {
    let area = frame.area();
    let gutter = gutter_width(editor.buffer());
    let text_height = area.height.saturating_sub(1);
    let text_width = area.width.saturating_sub(gutter);
    let buffer = frame.buffer_mut();
    draw_text(buffer, editor, area, gutter, text_width, text_height);
    draw_status(buffer, editor, area);
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
    let text_x = area.x.saturating_add(gutter);
    let text_right = text_x.saturating_add(text_width);
    let gutter_style = Style::default().add_modifier(Modifier::DIM);
    for row in 0..text_height {
        let line = first + usize::from(row);
        if line >= buffer.len_lines() {
            break;
        }
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
            target.set_string(x, y, &cell.text, Style::default());
            col += cell.width;
        }
    }
    draw_cursor(target, editor, area, gutter, text_width, text_height);
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
        cell.set_style(Style::default().add_modifier(Modifier::REVERSED));
    }
}

fn draw_status(target: &mut TuiBuffer, editor: &Editor, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let y = area.y.saturating_add(area.height - 1);
    let text = if editor.pending_quit() {
        " Unsaved changes. Press Ctrl+Q again to quit, Esc to cancel ".to_owned()
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
    target.set_string(
        area.x,
        y,
        &padded,
        Style::default().add_modifier(Modifier::REVERSED),
    );
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Modifier;

    use super::draw;
    use crate::buffer::{Buffer, CharIdx};
    use crate::command::Command;
    use crate::editor::Editor;
    use crate::test_support::TempDir;

    fn render(editor: &Editor, width: u16, height: u16) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, editor)).unwrap();
        terminal
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
            editor.apply(Command::MoveDown);
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
            editor.apply(Command::MoveRight);
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

    #[test]
    fn tiny_terminal_does_not_panic() {
        for (width, height) in [(1, 1), (2, 1), (1, 2)] {
            let mut editor = Editor::new(Buffer::from_text("abc"));
            editor.resize(width, height);
            let _terminal = render(&editor, width, height);
        }
    }
}
