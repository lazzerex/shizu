use ratatui::buffer::Buffer as TuiBuffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, Widget};
use unicode_width::UnicodeWidthStr;

use super::truncate;
use crate::editor::Editor;

pub(super) fn draw(target: &mut TuiBuffer, editor: &Editor, area: Rect) {
    let Some(palette) = editor.palette() else {
        return;
    };
    let height = editor.palette_height();
    if height < 3 || area.width < 16 || area.height < height {
        return;
    }
    let width = area.width.saturating_sub(4).min(60);
    let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
    let y = area
        .y
        .saturating_add(area.height.saturating_sub(height) / 2);
    let popup = Rect {
        x,
        y,
        width,
        height,
    };
    Clear.render(popup, target);
    let theme = editor.theme();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.line_number_style())
        .style(theme.background_style());
    let inner = block.inner(popup);
    block.render(popup, target);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let prompt = if palette.query().is_empty() {
        " Files  (> commands)".to_owned()
    } else {
        format!(" {}", palette.query())
    };
    target.set_string(
        inner.x,
        inner.y,
        truncate(&prompt, usize::from(inner.width)),
        theme.text(),
    );
    let rows = inner.height.saturating_sub(1);
    if palette.match_count() == 0 {
        if rows > 0 {
            target.set_string(
                inner.x,
                inner.y.saturating_add(1),
                truncate(" (no matches)", usize::from(inner.width)),
                theme.line_number_style(),
            );
        }
        return;
    }
    for (row, entry) in palette
        .visible()
        .skip(palette.scroll())
        .take(usize::from(rows))
        .enumerate()
    {
        let y = inner
            .y
            .saturating_add(1)
            .saturating_add(u16::try_from(row).unwrap_or(u16::MAX));
        let selected = palette.scroll() + row == palette.selected();
        let style = if selected {
            theme.selection()
        } else {
            theme.text()
        };
        target.set_style(
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
            style,
        );
        let shortcut = entry.shortcut.as_deref().unwrap_or_default();
        let shortcut_width = UnicodeWidthStr::width(shortcut);
        let label_width = usize::from(inner.width)
            .saturating_sub(shortcut_width)
            .saturating_sub(1);
        target.set_string(inner.x, y, truncate(&entry.label, label_width), style);
        if !shortcut.is_empty() {
            let x = inner
                .x
                .saturating_add(inner.width)
                .saturating_sub(u16::try_from(shortcut_width).unwrap_or(u16::MAX));
            let shortcut_style = if selected {
                style
            } else {
                theme.line_number_style()
            };
            target.set_string(x, y, shortcut, shortcut_style);
        }
    }
}
