use super::movement::move_line;
use super::{Editor, Focus, record};
use crate::buffer::{Buffer, CharIdx, Transaction};

impl Editor {
    pub(super) fn record_active(
        &mut self,
        transaction: Transaction,
        before: CharIdx,
        after: CharIdx,
    ) {
        let tab = &mut self.tabs[self.active];
        record(&mut tab.undo, &mut tab.redo, transaction, before, after);
    }

    pub(super) fn insert(&mut self, character: char) {
        let index = self.tabs[self.active].view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.tabs[self.active].view;
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
        self.record_active(transaction, cursor, after);
    }

    pub(super) fn backspace(&mut self) {
        let index = self.tabs[self.active].view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.tabs[self.active].view;
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
            self.record_active(transaction, cursor, from);
        }
    }

    pub(super) fn delete(&mut self) {
        let index = self.tabs[self.active].view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.tabs[self.active].view;
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
            self.record_active(transaction, cursor, from);
        }
    }

    pub(super) fn select_all(&mut self) {
        let index = self.tabs[self.active].view.buffer_id().index();
        self.tabs[self.active].view.select_all(&self.buffers[index]);
    }

    pub(super) fn cut(&mut self) {
        let index = self.tabs[self.active].view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.tabs[self.active].view;
        let Some((from, to)) = view.selection() else {
            return;
        };
        self.clipboard = buffer.text_range(from, to);
        self.pending_clipboard = Some(self.clipboard.clone());
        let cursor = view.cursor();
        let transaction = buffer.replace(from, to, "");
        view.set_cursor(from);
        self.record_active(transaction, cursor, from);
    }

    pub(super) fn copy(&mut self) {
        let index = self.tabs[self.active].view.buffer_id().index();
        let buffer = &self.buffers[index];
        let Some((from, to)) = self.tabs[self.active].view.selection() else {
            return;
        };
        self.clipboard = buffer.text_range(from, to);
        self.pending_clipboard = Some(self.clipboard.clone());
    }

    pub(super) fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        let text = self.clipboard.clone();
        let index = self.tabs[self.active].view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.tabs[self.active].view;
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
        self.record_active(transaction, cursor, after);
    }

    pub(super) fn undo(&mut self) {
        let tab = &mut self.tabs[self.active];
        let Some(edit) = tab.undo.pop() else {
            return;
        };
        let index = tab.view.buffer_id().index();
        self.buffers[index].revert(&edit.transaction);
        tab.view.set_cursor(edit.before);
        tab.redo.push(edit);
    }

    pub(super) fn redo(&mut self) {
        let tab = &mut self.tabs[self.active];
        let Some(edit) = tab.redo.pop() else {
            return;
        };
        let index = tab.view.buffer_id().index();
        self.buffers[index].apply(&edit.transaction);
        tab.view.set_cursor(edit.after);
        tab.undo.push(edit);
    }

    pub(super) fn move_line(&mut self, delta: isize) {
        let index = self.tabs[self.active].view.buffer_id().index();
        let buffer = &mut self.buffers[index];
        let view = &mut self.tabs[self.active].view;
        if let Some((transaction, cursor, after)) = move_line(buffer, view, delta) {
            self.record_active(transaction, cursor, after);
        }
    }

    pub(super) fn save(&mut self) {
        let index = self.tabs[self.active].view.buffer_id().index();
        match self.buffers[index].save() {
            Ok(()) => self.message = None,
            Err(error) => self.message = Some(error.to_string()),
        }
    }

    pub(super) fn request_quit(&mut self) {
        let dirty = self.buffers.iter().any(Buffer::is_dirty);
        if dirty && !self.pending_quit {
            self.pending_quit = true;
        } else {
            self.quit = true;
        }
    }

    pub(super) fn cancel(&mut self) {
        if self.focus != Focus::Editor {
            self.focus = Focus::Editor;
        }
    }
}
