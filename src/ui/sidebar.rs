use ratatui::buffer::Buffer as TuiBuffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::editor::Editor;
use crate::tree::TreeEntry;

const INDENT: usize = 2;

pub(super) fn draw_tree(target: &mut TuiBuffer, editor: &Editor, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let theme = editor.theme();
    target.set_style(area, theme.background_style());
    let Some(tree) = editor.file_tree() else {
        return;
    };
    let height = usize::from(area.height);
    let width = usize::from(area.width);
    for (row, entry) in tree
        .entries()
        .iter()
        .skip(tree.scroll())
        .take(height)
        .enumerate()
    {
        let style = if tree.scroll() + row == tree.selected() {
            theme.selection()
        } else if entry.is_dir {
            theme.line_number_style()
        } else {
            theme.text()
        };
        draw_entry(target, area, width, row, entry, style);
    }
}

fn draw_entry(
    target: &mut TuiBuffer,
    area: Rect,
    width: usize,
    row: usize,
    entry: &TreeEntry,
    style: Style,
) {
    let indent = entry.depth.saturating_mul(INDENT).min(width);
    let name = entry
        .path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let suffix = if entry.is_dir { "/" } else { "" };
    let text = format!("{}{}{}", " ".repeat(indent), name, suffix);
    let text = truncate(&text, width);
    let y = area
        .y
        .saturating_add(u16::try_from(row).unwrap_or(u16::MAX));
    target.set_string(area.x, y, &text, style);
}

pub(super) fn draw_tabs(target: &mut TuiBuffer, editor: &Editor, area: Rect, sidebar: u16) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let theme = editor.theme();
    let row = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    target.set_style(row, theme.background_style());
    let mut x = area.x.saturating_add(sidebar);
    let limit = area.x.saturating_add(area.width);
    for (index, label) in editor.tab_labels().iter().enumerate() {
        if x >= limit {
            break;
        }
        let style = if index == editor.active_tab() {
            theme.status_bar()
        } else {
            theme.line_number_style()
        };
        let text = truncate(label, usize::from(limit - x));
        target.set_string(x, area.y, &text, style);
        let width = u16::try_from(UnicodeWidthStr::width(text.as_str())).unwrap_or(u16::MAX);
        x = x.saturating_add(width);
    }
}

fn truncate(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_owned();
    }
    let mut output = String::new();
    let mut used = 0;
    for character in text.chars() {
        let size = UnicodeWidthStr::width(character.to_string().as_str());
        if used + size > width {
            break;
        }
        used += size;
        output.push(character);
    }
    output
}
