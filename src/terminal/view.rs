use ratatui::buffer::Buffer as TuiBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color as TuiColor, Modifier, Style};

/// Draws the visible emulator screen into `target`.
///
/// Returns the cursor position in target coordinates when the emulator has a
/// visible cursor inside `area`.
pub fn draw(screen: &vt100::Screen, target: &mut TuiBuffer, area: Rect) -> Option<(u16, u16)> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    let (screen_rows, screen_cols) = screen.size();
    let rows = area.height.min(screen_rows.get());
    let cols = area.width.min(screen_cols.get());
    target.set_style(
        area,
        Style::default()
            .fg(tui_color(screen.fgcolor()))
            .bg(tui_color(screen.bgcolor())),
    );
    for row in 0..rows {
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if cell.is_wide_continuation() {
                continue;
            }
            let symbol = if cell.contents().is_empty() {
                " "
            } else {
                cell.contents()
            };
            target.set_string(
                area.x.saturating_add(col),
                area.y.saturating_add(row),
                symbol,
                cell_style(cell),
            );
        }
    }
    let (row, col) = screen.cursor_position();
    if screen.hide_cursor() || row >= rows || col >= cols {
        return None;
    }
    Some((area.x.saturating_add(col), area.y.saturating_add(row)))
}

fn cell_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default()
        .fg(tui_color(cell.fgcolor()))
        .bg(tui_color(cell.bgcolor()));
    if cell.bold() {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.dim() {
        style = style.add_modifier(Modifier::DIM);
    }
    if cell.italic() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.inverse() {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

fn tui_color(color: vt100::Color) -> TuiColor {
    match color {
        vt100::Color::Default => TuiColor::Reset,
        vt100::Color::Idx(index) => TuiColor::Indexed(index),
        vt100::Color::Rgb(red, green, blue) => TuiColor::Rgb(red, green, blue),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer as TuiBuffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Modifier};

    use super::draw;

    fn buffer(width: u16, height: u16) -> TuiBuffer {
        TuiBuffer::empty(Rect::new(0, 0, width, height))
    }

    #[test]
    fn draws_text_and_colors() {
        let mut parser = vt100::Parser::new(
            std::num::NonZeroU16::new(5).unwrap(),
            std::num::NonZeroU16::new(20).unwrap(),
            0,
        );
        parser.process(b"hello \x1b[31mred\x1b[0m");
        let mut target = buffer(20, 5);
        let cursor = draw(parser.screen(), &mut target, Rect::new(0, 0, 20, 5));
        assert_eq!(target[(0, 0)].symbol(), "h");
        assert_eq!(target[(6, 0)].symbol(), "r");
        assert_eq!(target[(6, 0)].fg, Color::Indexed(1));
        assert_eq!(cursor, Some((9, 0)));
    }

    #[test]
    fn draws_wide_cells() {
        let mut parser = vt100::Parser::new(
            std::num::NonZeroU16::new(2).unwrap(),
            std::num::NonZeroU16::new(10).unwrap(),
            0,
        );
        parser.process("日本".as_bytes());
        let mut target = buffer(10, 2);
        draw(parser.screen(), &mut target, Rect::new(0, 0, 10, 2));
        assert_eq!(target[(0, 0)].symbol(), "日");
        assert_eq!(target[(2, 0)].symbol(), "本");
    }

    #[test]
    fn maps_attributes_and_offsets_the_cursor() {
        let mut parser = vt100::Parser::new(
            std::num::NonZeroU16::new(3).unwrap(),
            std::num::NonZeroU16::new(12).unwrap(),
            0,
        );
        parser.process(b"\x1b[1;7mwide\x1b[0m");
        let mut target = buffer(16, 6);
        let cursor = draw(parser.screen(), &mut target, Rect::new(2, 1, 12, 3));
        assert!(
            target[(2, 1)]
                .modifier
                .contains(Modifier::BOLD | Modifier::REVERSED)
        );
        assert_eq!(cursor, Some((6, 1)));
    }

    #[test]
    fn draws_the_alternate_screen() {
        let mut parser = vt100::Parser::new(
            std::num::NonZeroU16::new(3).unwrap(),
            std::num::NonZeroU16::new(12).unwrap(),
            0,
        );
        parser.process(b"main\x1b[?1049halt");
        assert!(parser.screen().alternate_screen());
        let mut target = buffer(12, 3);
        draw(parser.screen(), &mut target, Rect::new(0, 0, 12, 3));
        assert_eq!(target[(0, 0)].symbol(), "a");
        parser.process(b"\x1b[?1049l");
        let mut target = buffer(12, 3);
        draw(parser.screen(), &mut target, Rect::new(0, 0, 12, 3));
        assert_eq!(target[(0, 0)].symbol(), "m");
    }

    #[test]
    fn skips_a_hidden_cursor() {
        let mut parser = vt100::Parser::new(
            std::num::NonZeroU16::new(3).unwrap(),
            std::num::NonZeroU16::new(12).unwrap(),
            0,
        );
        parser.process(b"\x1b[?25lhi");
        let mut target = buffer(12, 3);
        let cursor = draw(parser.screen(), &mut target, Rect::new(0, 0, 12, 3));
        assert_eq!(cursor, None);
    }
}
