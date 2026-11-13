use super::{Editor, Focus};
use crate::buffer::{Buffer, CharIdx, Transaction};
use crate::view::{View, char_idx_at_display_col, display_col, gutter_width};

impl Editor {
    pub(super) fn move_left(&mut self, select: bool) {
        let index = self.tabs[self.active].view.buffer_id().index();
        self.tabs[self.active]
            .view
            .move_left(&self.buffers[index], select);
    }

    pub(super) fn move_right(&mut self, select: bool) {
        let index = self.tabs[self.active].view.buffer_id().index();
        self.tabs[self.active]
            .view
            .move_right(&self.buffers[index], select);
    }

    pub(super) fn move_up(&mut self, select: bool) {
        let index = self.tabs[self.active].view.buffer_id().index();
        self.tabs[self.active]
            .view
            .move_up(&self.buffers[index], select);
    }

    pub(super) fn move_down(&mut self, select: bool) {
        let index = self.tabs[self.active].view.buffer_id().index();
        self.tabs[self.active]
            .view
            .move_down(&self.buffers[index], select);
    }

    pub(super) fn home(&mut self, select: bool) {
        let index = self.tabs[self.active].view.buffer_id().index();
        self.tabs[self.active]
            .view
            .home(&self.buffers[index], select);
    }

    pub(super) fn end(&mut self, select: bool) {
        let index = self.tabs[self.active].view.buffer_id().index();
        self.tabs[self.active]
            .view
            .end(&self.buffers[index], select);
    }

    pub(super) fn page_up(&mut self, select: bool) {
        let height = self.text_height();
        let index = self.tabs[self.active].view.buffer_id().index();
        self.tabs[self.active]
            .view
            .page_up(&self.buffers[index], height, select);
    }

    pub(super) fn page_down(&mut self, select: bool) {
        let height = self.text_height();
        let index = self.tabs[self.active].view.buffer_id().index();
        self.tabs[self.active]
            .view
            .page_down(&self.buffers[index], height, select);
    }
    pub(super) fn click(&mut self, column: u16, row: u16) {
        let sidebar = self.sidebar_width();
        let bars = self.tab_bar_height();
        if bars > 0 && row < bars {
            if column >= sidebar {
                self.click_tab(column - sidebar);
            }
            return;
        }
        if column < sidebar {
            self.click_tree(row);
            return;
        }
        self.focus = Focus::Editor;
        let height = self.text_height();
        let text_column = column - sidebar;
        let text_row = row - bars;
        let index = self.tabs[self.active].view.buffer_id().index();
        let at = char_at_screen(
            &self.buffers[index],
            &self.tabs[self.active].view,
            height,
            text_column,
            text_row,
        );
        if let Some(at) = at {
            self.tabs[self.active].view.click(at);
        }
    }

    pub(super) fn drag(&mut self, column: u16, row: u16) {
        let sidebar = self.sidebar_width();
        if column < sidebar {
            return;
        }
        self.focus = Focus::Editor;
        let height = self.text_height();
        let text_column = column - sidebar;
        let text_row = row.saturating_sub(self.tab_bar_height());
        let index = self.tabs[self.active].view.buffer_id().index();
        let at = char_at_screen(
            &self.buffers[index],
            &self.tabs[self.active].view,
            height,
            text_column,
            text_row,
        );
        if let Some(at) = at {
            self.tabs[self.active].view.extend_to(at);
        }
    }

    pub(super) fn scroll_lines(&mut self, delta: isize) {
        let height = self.text_height();
        let index = self.tabs[self.active].view.buffer_id().index();
        let max = self.buffers[index].len_lines().saturating_sub(height);
        let next = self.tabs[self.active]
            .view
            .scroll_line()
            .saturating_add_signed(delta)
            .min(max);
        self.tabs[self.active].view.set_scroll_line(next);
    }

    fn click_tab(&mut self, column: u16) {
        if let Some(index) = self.tab_at(column) {
            self.select_tab(index);
            self.focus = Focus::Editor;
        }
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
