use std::cmp::Ordering;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::buffer::{Buffer, BufferId, CharIdx, LineCol};

#[derive(Debug)]
pub struct View {
    buffer_id: BufferId,
    cursor: CharIdx,
    anchor: Option<CharIdx>,
    scroll_line: usize,
    scroll_col: usize,
    goal_col: Option<usize>,
    tab_width: usize,
}

impl View {
    pub fn new(buffer_id: BufferId) -> Self {
        Self {
            buffer_id,
            cursor: CharIdx::new(0),
            anchor: None,
            scroll_line: 0,
            scroll_col: 0,
            goal_col: None,
            tab_width: 4,
        }
    }

    pub fn buffer_id(&self) -> BufferId {
        self.buffer_id
    }

    pub fn cursor(&self) -> CharIdx {
        self.cursor
    }

    pub fn scroll_line(&self) -> usize {
        self.scroll_line
    }

    pub fn scroll_col(&self) -> usize {
        self.scroll_col
    }

    pub fn set_scroll_line(&mut self, line: usize) {
        self.scroll_line = line;
    }

    pub fn tab_width(&self) -> usize {
        self.tab_width
    }

    pub fn set_cursor(&mut self, at: CharIdx) {
        self.cursor = at;
        self.anchor = None;
        self.goal_col = None;
    }

    pub fn set_tab_width(&mut self, width: usize) {
        self.tab_width = width.max(1);
    }

    pub fn click(&mut self, at: CharIdx) {
        self.cursor = at;
        self.anchor = Some(at);
        self.goal_col = None;
    }

    pub fn extend_to(&mut self, at: CharIdx) {
        if self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        }
        self.cursor = at;
        self.goal_col = None;
    }

    pub fn select_all(&mut self, buffer: &Buffer) {
        self.anchor = Some(CharIdx::new(0));
        self.cursor = CharIdx::new(buffer.len_chars());
        self.goal_col = None;
    }

    pub fn selection(&self) -> Option<(CharIdx, CharIdx)> {
        let anchor = self.anchor?;
        match anchor.cmp(&self.cursor) {
            Ordering::Less => Some((anchor, self.cursor)),
            Ordering::Greater => Some((self.cursor, anchor)),
            Ordering::Equal => None,
        }
    }

    pub fn move_left(&mut self, buffer: &Buffer, select: bool) {
        self.move_to(buffer.prev_grapheme(self.cursor), select);
    }

    pub fn move_right(&mut self, buffer: &Buffer, select: bool) {
        self.move_to(buffer.next_grapheme(self.cursor), select);
    }

    pub fn move_up(&mut self, buffer: &Buffer, select: bool) {
        self.move_vertical(buffer, -1, select);
    }

    pub fn move_down(&mut self, buffer: &Buffer, select: bool) {
        self.move_vertical(buffer, 1, select);
    }

    pub fn home(&mut self, buffer: &Buffer, select: bool) {
        let line = buffer.line_col(self.cursor).line;
        self.move_to(buffer.line_start(line), select);
    }

    pub fn end(&mut self, buffer: &Buffer, select: bool) {
        let line = buffer.line_col(self.cursor).line;
        self.move_to(buffer.line_end(line), select);
    }

    pub fn page_up(&mut self, buffer: &Buffer, height: usize, select: bool) {
        self.move_vertical(
            buffer,
            -isize::try_from(height).unwrap_or(isize::MAX),
            select,
        );
    }

    pub fn page_down(&mut self, buffer: &Buffer, height: usize, select: bool) {
        self.move_vertical(
            buffer,
            isize::try_from(height).unwrap_or(isize::MAX),
            select,
        );
    }

    pub fn ensure_visible(&mut self, buffer: &Buffer, width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        let line = buffer.line_col(self.cursor).line;
        if line < self.scroll_line {
            self.scroll_line = line;
        } else if line >= self.scroll_line.saturating_add(height) {
            self.scroll_line = line + 1 - height;
        }
        let col = display_col(buffer, self.cursor, self.tab_width);
        if col < self.scroll_col {
            self.scroll_col = col;
        } else if col >= self.scroll_col.saturating_add(width) {
            self.scroll_col = col + 1 - width;
        }
    }

    fn move_to(&mut self, at: CharIdx, select: bool) {
        if select {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
        self.cursor = at;
        self.goal_col = None;
    }

    fn move_vertical(&mut self, buffer: &Buffer, delta: isize, select: bool) {
        let line = buffer.line_col(self.cursor).line;
        let goal = self
            .goal_col
            .unwrap_or_else(|| display_col(buffer, self.cursor, self.tab_width));
        let target = line
            .saturating_add_signed(delta)
            .min(buffer.len_lines() - 1);
        let at = char_idx_at_display_col(buffer, target, goal, self.tab_width);
        self.move_to(at, select);
        self.goal_col = Some(goal);
    }
}

#[derive(Debug)]
pub(crate) struct DisplayCell {
    pub(crate) text: String,
    pub(crate) width: usize,
    pub(crate) start: usize,
}

pub(crate) fn layout_line(line: &str, tab_width: usize) -> Vec<DisplayCell> {
    let mut cells = Vec::new();
    let mut col = 0;
    let mut start = 0;
    for grapheme in line.graphemes(true) {
        let chars = grapheme.chars().count();
        if grapheme == "\t" {
            let width = tab_width - (col % tab_width);
            cells.push(DisplayCell {
                text: " ".repeat(width),
                width,
                start,
            });
            col += width;
            start += chars;
            continue;
        }
        if grapheme.chars().all(char::is_control) {
            cells.push(DisplayCell {
                text: " ".to_owned(),
                width: 1,
                start,
            });
            col += 1;
            start += chars;
            continue;
        }
        let width = UnicodeWidthStr::width(grapheme);
        if width == 0 {
            if let Some(last) = cells.last_mut() {
                last.text.push_str(grapheme);
            }
            start += chars;
            continue;
        }
        cells.push(DisplayCell {
            text: grapheme.to_owned(),
            width,
            start,
        });
        col += width;
        start += chars;
    }
    cells
}

pub(crate) fn display_col(buffer: &Buffer, at: CharIdx, tab_width: usize) -> usize {
    let pos = buffer.line_col(at);
    let cells = layout_line(&buffer.line_text(pos.line), tab_width);
    let mut col = 0;
    for cell in &cells {
        if cell.start >= pos.col {
            return col;
        }
        col += cell.width;
    }
    col
}

pub(crate) fn char_idx_at_display_col(
    buffer: &Buffer,
    line: usize,
    target: usize,
    tab_width: usize,
) -> CharIdx {
    let cells = layout_line(&buffer.line_text(line), tab_width);
    let mut col = 0;
    for cell in &cells {
        if target < col + cell.width {
            return match buffer.char_idx(LineCol {
                line,
                col: cell.start,
            }) {
                Some(at) => at,
                None => buffer.line_end(line),
            };
        }
        col += cell.width;
    }
    buffer.line_end(line)
}

pub(crate) fn gutter_width(buffer: &Buffer) -> u16 {
    let digits = buffer.len_lines().to_string().len();
    u16::try_from(digits + 1).unwrap_or(u16::MAX)
}
