use std::path::Path;

use super::{Editor, Focus};
use crate::tree::FileTree;

impl Editor {
    pub fn sidebar_visible(&self) -> bool {
        self.tree.as_ref().is_some_and(FileTree::visible)
    }

    pub fn sidebar_width(&self) -> u16 {
        if !self.sidebar_visible() || self.viewport.width < 50 {
            return 0;
        }
        (self.viewport.width / 4).clamp(16, 30)
    }

    pub fn sidebar_height(&self) -> u16 {
        self.viewport.height.saturating_sub(1)
    }

    pub fn file_tree(&self) -> Option<&FileTree> {
        self.tree.as_ref()
    }

    pub(super) fn toggle_sidebar(&mut self) {
        if self.tree.is_none() {
            let root = self
                .buffer()
                .path()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .or_else(|| std::env::current_dir().ok());
            let Some(root) = root else {
                self.message = Some("cannot determine working directory".to_owned());
                return;
            };
            self.tree = Some(FileTree::new(root));
        }
        let Some(tree) = self.tree.as_mut() else {
            return;
        };
        let visible = !tree.visible();
        tree.set_visible(visible);
        self.focus = if visible { Focus::Tree } else { Focus::Editor };
        self.bloom.clear();
    }

    pub(super) fn tree_up(&mut self) {
        self.move_tree_selection(-1);
    }

    pub(super) fn tree_down(&mut self) {
        self.move_tree_selection(1);
    }

    pub(super) fn tree_left(&mut self) {
        if let Some(tree) = self.tree.as_mut() {
            tree.collapse_selected();
        }
    }

    pub(super) fn tree_right(&mut self) {
        if let Some(tree) = self.tree.as_mut() {
            tree.right();
        }
    }

    pub(super) fn tree_open(&mut self) {
        let activated = self.tree.as_mut().and_then(FileTree::activate);
        if let Some(path) = activated {
            self.open_path(path);
        }
    }

    pub(super) fn click_tree(&mut self, row: u16) {
        self.focus = Focus::Tree;
        let height = usize::from(self.sidebar_height());
        let activated = match self.tree.as_mut() {
            Some(tree) => {
                let Some(index) = tree.index_at_row(usize::from(row)) else {
                    return;
                };
                tree.select(index, height);
                tree.activate()
            }
            None => None,
        };
        if let Some(path) = activated {
            self.open_path(path);
        }
    }

    fn move_tree_selection(&mut self, delta: isize) {
        let height = usize::from(self.sidebar_height());
        if let Some(tree) = self.tree.as_mut() {
            tree.move_selection(delta, height);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Editor;
    use crate::buffer::Buffer;
    use crate::command::Command;
    use crate::editor::Focus;
    use crate::test_support::TempDir;

    fn editor_in(dir: &TempDir) -> Editor {
        let path = dir.join("first.txt");
        fs::write(&path, "one\n").unwrap();
        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        editor.resize(80, 20);
        editor
    }

    #[test]
    fn toggling_the_sidebar_shows_a_tree_at_the_buffer_directory() {
        let dir = TempDir::new();
        let mut editor = editor_in(&dir);

        assert!(!editor.sidebar_visible());
        assert!(editor.apply(Command::ToggleSidebar));

        assert!(editor.sidebar_visible());
        assert_eq!(editor.focus(), Focus::Tree);
        assert_eq!(editor.sidebar_width(), 20);
        let tree = editor.file_tree().expect("tree exists");
        assert_eq!(tree.root(), dir.path());
    }

    #[test]
    fn toggling_again_hides_the_sidebar() {
        let dir = TempDir::new();
        let mut editor = editor_in(&dir);
        editor.apply(Command::ToggleSidebar);

        assert!(editor.apply(Command::ToggleSidebar));

        assert!(!editor.sidebar_visible());
        assert_eq!(editor.focus(), Focus::Editor);
        assert_eq!(editor.sidebar_width(), 0);
    }

    #[test]
    fn narrow_viewports_keep_the_sidebar_hidden() {
        let dir = TempDir::new();
        let mut editor = editor_in(&dir);
        editor.resize(40, 20);
        editor.apply(Command::ToggleSidebar);

        assert!(editor.sidebar_visible());
        assert_eq!(editor.sidebar_width(), 0);
    }

    #[test]
    fn tree_navigation_moves_and_opens_files() {
        let dir = TempDir::new();
        fs::write(dir.join("second.txt"), "two\n").unwrap();
        let mut editor = editor_in(&dir);
        editor.apply(Command::ToggleSidebar);

        editor.apply(Command::TreeDown);
        editor.apply(Command::TreeOpen);

        assert_eq!(
            editor.buffer().path(),
            Some(dir.join("second.txt").as_path())
        );
        assert_eq!(editor.focus(), Focus::Editor);
    }

    #[test]
    fn clicking_a_tree_row_opens_the_file() {
        let dir = TempDir::new();
        fs::write(dir.join("second.txt"), "two\n").unwrap();
        let mut editor = editor_in(&dir);
        editor.apply(Command::ToggleSidebar);

        editor.apply(Command::Click { column: 1, row: 1 });

        assert_eq!(
            editor.buffer().path(),
            Some(dir.join("second.txt").as_path())
        );
        assert_eq!(editor.focus(), Focus::Editor);
    }
}
