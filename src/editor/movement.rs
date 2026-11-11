use super::Editor;
use crate::buffer::{Buffer, CharIdx, Transaction};
use crate::view::{View, char_idx_at_display_col, display_col, gutter_width};

impl Editor {
    pub(super) fn move_left(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.move_left(&self.buffers[index], select);
    }

    pub(super) fn move_right(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.move_right(&self.buffers[index], select);
    }

    pub(super) fn move_up(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.move_up(&self.buffers[index], select);
    }

    pub(super) fn move_down(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.move_down(&self.buffers[index], select);
    }

    pub(super) fn home(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.home(&self.buffers[index], select);
    }

    pub(super) fn end(&mut self, select: bool) {
        let index = self.view.buffer_id().index();
        self.view.end(&self.buffers[index], select);
    }

    pub(super) fn page_up(&mut self, select: bool) {
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        self.view.page_up(&self.buffers[index], height, select);
    }

    pub(super) fn page_down(&mut self, select: bool) {
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        self.view.page_down(&self.buffers[index], height, select);
    }
    pub(super) fn click(&mut self, column: u16, row: u16) {
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        let at = char_at_screen(&self.buffers[index], &self.view, height, column, row);
        if let Some(at) = at {
            self.view.click(at);
        }
    }

    pub(super) fn drag(&mut self, column: u16, row: u16) {
        let height = self.text_height();
        let index = self.view.buffer_id().index();
        let at = char_at_screen(&self.buffers[index], &self.view, height, column, row);
        if let Some(at) = at {
            self.view.extend_to(at);
        }
    }

    pub(super) fn scroll_lines(&mut self, delta: isize) {
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
}

pub(super) fn char_at_screen(
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

pub(super) fn move_line(
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
