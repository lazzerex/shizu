use std::path::Path;

use unicode_width::UnicodeWidthStr;

use super::{Edit, Editor, Focus};
use crate::buffer::{Buffer, BufferId};
use crate::view::View;

#[derive(Debug)]
pub(super) struct Tab {
    pub(super) view: View,
    pub(super) undo: Vec<Edit>,
    pub(super) redo: Vec<Edit>,
}

impl Tab {
    pub(super) fn new(buffer_id: BufferId) -> Self {
        Self {
            view: View::new(buffer_id),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

impl Editor {
    pub fn open_path(&mut self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        if let Some(index) = self.tab_for_path(path) {
            if index != self.active {
                self.select_tab(index);
            }
            self.focus = Focus::Editor;
            return true;
        }
        match Buffer::open(path) {
            Ok(buffer) => {
                let id = BufferId::new(self.buffers.len());
                self.buffers.push(buffer);
                self.tabs.push(Tab::new(id));
                self.message = None;
                self.select_tab(self.tabs.len() - 1)
            }
            Err(error) => {
                self.message = Some(format!("{}: {error}", path.display()));
                true
            }
        }
    }

    pub fn select_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() || index == self.active {
            return false;
        }
        self.active = index;
        self.focus = Focus::Editor;
        self.prompt.matches.clear();
        self.prompt.current = None;
        self.bloom.clear();
        self.ensure_visible();
        self.refresh_highlights();
        true
    }

    pub fn next_tab(&mut self) -> bool {
        if self.tabs.len() < 2 {
            return false;
        }
        let index = (self.active + 1) % self.tabs.len();
        self.select_tab(index)
    }

    pub fn previous_tab(&mut self) -> bool {
        if self.tabs.len() < 2 {
            return false;
        }
        let index = (self.active + self.tabs.len() - 1) % self.tabs.len();
        self.select_tab(index)
    }

    pub fn request_close_tab(&mut self) {
        if self.buffer().is_dirty() && !self.pending_close {
            self.pending_close = true;
            self.message =
                Some("Unsaved changes. Press Ctrl+W again to close, Esc to cancel".to_owned());
            return;
        }
        self.close_active_tab();
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_tab(&self) -> usize {
        self.active
    }

    pub fn tab_bar_height(&self) -> u16 {
        u16::from(self.tabs.len() > 1)
    }

    pub fn tab_labels(&self) -> Vec<String> {
        (0..self.tabs.len())
            .map(|index| self.tab_label(index))
            .collect()
    }

    pub fn tab_at(&self, column: u16) -> Option<usize> {
        let mut offset = 0_u16;
        for (index, label) in self.tab_labels().iter().enumerate() {
            let width = u16::try_from(UnicodeWidthStr::width(label.as_str())).unwrap_or(u16::MAX);
            if column < offset.saturating_add(width) {
                return Some(index);
            }
            offset = offset.saturating_add(width);
        }
        None
    }

    fn tab_for_path(&self, path: &Path) -> Option<usize> {
        (0..self.tabs.len()).find(|&index| {
            let id = self.tabs[index].view.buffer_id().index();
            self.buffers
                .get(id)
                .and_then(Buffer::path)
                .is_some_and(|candidate| candidate == path)
        })
    }

    fn tab_label(&self, index: usize) -> String {
        let id = self.tabs[index].view.buffer_id().index();
        let buffer = &self.buffers[id];
        let name = buffer.path().and_then(Path::file_name).map_or_else(
            || "[No Name]".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let dirty = if buffer.is_dirty() { "*" } else { "" };
        format!(" {name}{dirty} ")
    }

    fn close_active_tab(&mut self) {
        self.pending_close = false;
        self.message = None;
        if self.tabs.len() == 1 {
            self.buffers.clear();
            self.buffers.push(Buffer::new());
            self.tabs.clear();
            self.tabs.push(Tab::new(BufferId::new(0)));
            self.active = 0;
        } else {
            let removed = self.tabs[self.active].view.buffer_id().index();
            self.buffers.remove(removed);
            self.tabs.remove(self.active);
            if self.active >= self.tabs.len() {
                self.active = self.tabs.len() - 1;
            }
            self.remap_buffers(removed);
        }
        self.focus = Focus::Editor;
        self.prompt.matches.clear();
        self.prompt.current = None;
        self.bloom.clear();
        self.ensure_visible();
        self.refresh_highlights();
    }

    fn remap_buffers(&mut self, removed: usize) {
        let fix = |id: BufferId| {
            let index = id.index();
            BufferId::new(if index > removed { index - 1 } else { index })
        };
        for tab in &mut self.tabs {
            tab.view.set_buffer_id(fix(tab.view.buffer_id()));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Editor;
    use crate::buffer::Buffer;
    use crate::command::Command;
    use crate::test_support::TempDir;

    fn editor_with(dir: &TempDir, name: &str, text: &str) -> Editor {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        editor.resize(40, 10);
        editor
    }

    #[test]
    fn opening_the_same_file_twice_reuses_the_buffer() {
        let dir = TempDir::new();
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        fs::write(&first, "one\n").unwrap();
        fs::write(&second, "two\n").unwrap();
        let mut editor = Editor::new(Buffer::open(&first).unwrap());
        editor.resize(40, 10);

        assert!(editor.open_path(&second));
        assert_eq!(editor.tab_count(), 2);
        assert!(editor.open_path(&first));
        assert_eq!(editor.tab_count(), 2);
        assert_eq!(editor.buffer().path(), Some(first.as_path()));
    }

    #[test]
    fn selecting_a_tab_switches_the_visible_buffer() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "first.txt", "one\n");
        let second = dir.join("second.txt");
        fs::write(&second, "two\n").unwrap();

        assert!(editor.open_path(&second));
        assert_eq!(editor.active_tab(), 1);
        assert!(editor.select_tab(0));
        assert_eq!(editor.buffer().text(), "one\n");
        assert!(!editor.select_tab(0));
    }

    #[test]
    fn next_and_previous_tab_wrap() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "first.txt", "one\n");
        let second = dir.join("second.txt");
        fs::write(&second, "two\n").unwrap();
        editor.open_path(&second);

        assert!(editor.next_tab());
        assert_eq!(editor.active_tab(), 0);
        assert!(editor.previous_tab());
        assert_eq!(editor.active_tab(), 1);
    }

    #[test]
    fn opening_an_invalid_file_reports_an_error() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "first.txt", "one\n");

        assert!(editor.open_path(dir.join("missing.txt")));
        assert_eq!(editor.tab_count(), 1);
        assert!(
            editor
                .message()
                .is_some_and(|message| message.contains("missing.txt"))
        );
    }

    #[test]
    fn closing_a_clean_tab_removes_it() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "first.txt", "one\n");
        let second = dir.join("second.txt");
        fs::write(&second, "two\n").unwrap();
        editor.open_path(&second);

        editor.apply(Command::CloseTab);

        assert_eq!(editor.tab_count(), 1);
        assert_eq!(editor.buffer().text(), "one\n");
        assert!(!editor.pending_close());
    }

    #[test]
    fn closing_a_dirty_buffer_asks_first() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "first.txt", "one\n");
        let second = dir.join("second.txt");
        fs::write(&second, "two\n").unwrap();
        editor.open_path(&second);
        editor.apply(Command::Insert('x'));

        assert!(editor.apply(Command::CloseTab));
        assert_eq!(editor.tab_count(), 2);
        assert!(editor.pending_close());

        assert!(editor.apply(Command::CloseTab));
        assert_eq!(editor.tab_count(), 1);
        assert!(!editor.pending_close());
    }

    #[test]
    fn other_commands_clear_pending_close() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "first.txt", "one\n");
        editor.apply(Command::Insert('x'));
        editor.apply(Command::CloseTab);
        assert!(editor.pending_close());

        editor.apply(Command::MoveLeft { select: false });

        assert!(!editor.pending_close());
    }

    #[test]
    fn closing_the_last_tab_leaves_an_empty_buffer() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "first.txt", "one\n");

        editor.apply(Command::CloseTab);

        assert_eq!(editor.tab_count(), 1);
        assert!(editor.buffer().path().is_none());
        assert!(editor.buffer().is_empty());
    }

    #[test]
    fn undo_history_is_per_tab() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "first.txt", "one\n");
        let second = dir.join("second.txt");
        fs::write(&second, "two\n").unwrap();
        editor.open_path(&second);
        editor.apply(Command::Insert('x'));
        assert_eq!(editor.buffer().text(), "xtwo\n");

        assert!(editor.apply(Command::Undo));
        assert_eq!(editor.buffer().text(), "two\n");

        assert!(editor.select_tab(0));
        assert_eq!(editor.buffer().text(), "one\n");
        assert!(!editor.apply(Command::Undo));
    }

    #[test]
    fn tab_labels_mark_the_dirty_buffer() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "first.txt", "one\n");
        let second = dir.join("second.txt");
        fs::write(&second, "two\n").unwrap();
        editor.open_path(&second);
        editor.apply(Command::Insert('x'));

        let labels = editor.tab_labels();

        assert_eq!(labels.len(), 2);
        assert!(labels[0].contains("first.txt"));
        assert!(labels[1].contains("second.txt*"));
    }

    #[test]
    fn tab_at_maps_click_columns_to_tabs() {
        let dir = TempDir::new();
        let mut editor = editor_with(&dir, "a.txt", "one\n");
        let second = dir.join("b.txt");
        fs::write(&second, "two\n").unwrap();
        editor.open_path(&second);

        let labels = editor.tab_labels();
        assert_eq!(editor.tab_at(0), Some(0));
        let second_start =
            u16::try_from(unicode_width::UnicodeWidthStr::width(labels[0].as_str())).unwrap();
        assert_eq!(editor.tab_at(second_start), Some(1));
        assert_eq!(
            editor.tab_at(second_start + u16::try_from(labels[1].len()).unwrap()),
            None
        );
    }
}
