use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
}

#[derive(Debug)]
pub struct FileTree {
    root: PathBuf,
    expanded: BTreeSet<PathBuf>,
    entries: Vec<TreeEntry>,
    selected: usize,
    scroll: usize,
    visible: bool,
    revision: u64,
}

impl FileTree {
    pub fn new(root: PathBuf) -> Self {
        let mut tree = Self {
            root,
            expanded: BTreeSet::new(),
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
            visible: false,
            revision: 0,
        };
        tree.refresh();
        tree
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.touch();
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn entries(&self) -> &[TreeEntry] {
        &self.entries
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn is_expanded(&self, path: &Path) -> bool {
        self.expanded.contains(path)
    }

    pub fn move_selection(&mut self, delta: isize, height: usize) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let last = self.entries.len() - 1;
        let next = self.selected.saturating_add_signed(delta).min(last);
        let changed = next != self.selected;
        self.selected = next;
        if changed {
            self.touch();
        }
        self.ensure_visible(height);
        changed
    }

    pub fn select(&mut self, index: usize, height: usize) -> bool {
        if index >= self.entries.len() {
            return false;
        }
        let changed = index != self.selected;
        self.selected = index;
        if changed {
            self.touch();
        }
        self.ensure_visible(height);
        changed
    }

    pub fn expand_selected(&mut self) -> bool {
        let Some(entry) = self.entries.get(self.selected) else {
            return false;
        };
        if !entry.is_dir || self.expanded.contains(&entry.path) {
            return false;
        }
        self.expanded.insert(entry.path.clone());
        self.refresh_keeping_selection();
        true
    }

    pub fn collapse_selected(&mut self) -> bool {
        let Some(entry) = self.entries.get(self.selected).cloned() else {
            return false;
        };
        if entry.is_dir && self.expanded.remove(&entry.path) {
            self.refresh_keeping_selection();
            return true;
        }
        let Some(parent) = self.parent_index(self.selected) else {
            return false;
        };
        self.selected = parent;
        self.touch();
        true
    }

    pub fn right(&mut self) -> bool {
        let Some(entry) = self.entries.get(self.selected) else {
            return false;
        };
        if !entry.is_dir {
            return false;
        }
        if !self.expanded.contains(&entry.path) {
            return self.expand_selected();
        }
        let Some(next) = self.entries.get(self.selected + 1) else {
            return false;
        };
        if next.depth <= entry.depth {
            return false;
        }
        self.selected += 1;
        self.touch();
        true
    }

    pub fn activate(&mut self) -> Option<PathBuf> {
        let entry = self.entries.get(self.selected)?.clone();
        if entry.is_dir {
            if !self.expanded.remove(&entry.path) {
                self.expanded.insert(entry.path.clone());
            }
            self.refresh_keeping_selection();
            None
        } else {
            Some(entry.path)
        }
    }

    pub fn index_at_row(&self, row: usize) -> Option<usize> {
        let index = self.scroll.checked_add(row)?;
        (index < self.entries.len()).then_some(index)
    }

    fn ensure_visible(&mut self, height: usize) {
        if height == 0 {
            return;
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll.saturating_add(height) {
            self.scroll = self.selected + 1 - height;
        }
        let max = self.entries.len().saturating_sub(height);
        self.scroll = self.scroll.min(max);
    }

    fn parent_index(&self, index: usize) -> Option<usize> {
        let depth = self.entries.get(index)?.depth;
        if depth == 0 {
            return None;
        }
        self.entries[..index]
            .iter()
            .rposition(|entry| entry.depth < depth)
    }

    fn refresh_keeping_selection(&mut self) {
        let path = self
            .entries
            .get(self.selected)
            .map(|entry| entry.path.clone());
        self.refresh();
        if let Some(path) = path
            && let Some(index) = self.entries.iter().position(|entry| entry.path == path)
        {
            self.selected = index;
        } else {
            self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        }
    }

    fn refresh(&mut self) {
        let mut entries = Vec::new();
        collect(&self.root, 0, &self.expanded, &mut entries);
        self.entries = entries;
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        let max = self.entries.len().saturating_sub(1);
        self.scroll = self.scroll.min(max);
        self.touch();
    }

    fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

fn collect(dir: &Path, depth: usize, expanded: &BTreeSet<PathBuf>, entries: &mut Vec<TreeEntry>) {
    for child in children(dir) {
        let path = child.path().to_path_buf();
        let is_dir = child.file_type().is_some_and(|kind| kind.is_dir());
        entries.push(TreeEntry {
            path: path.clone(),
            depth,
            is_dir,
        });
        if is_dir && expanded.contains(&path) {
            collect(&path, depth + 1, expanded, entries);
        }
    }
}

fn children(dir: &Path) -> Vec<ignore::DirEntry> {
    let mut builder = WalkBuilder::new(dir);
    builder
        .max_depth(Some(1))
        .hidden(true)
        .require_git(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false);
    let mut children: Vec<ignore::DirEntry> = builder
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.depth() == 1)
        .collect();
    children.sort_by(|left, right| {
        let left_dir = left.file_type().is_some_and(|kind| kind.is_dir());
        let right_dir = right.file_type().is_some_and(|kind| kind.is_dir());
        right_dir.cmp(&left_dir).then_with(|| {
            left.file_name()
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.file_name().to_string_lossy().to_lowercase())
        })
    });
    children
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::FileTree;
    use crate::test_support::TempDir;

    fn named(tree: &FileTree) -> Vec<String> {
        tree.entries()
            .iter()
            .map(|entry| {
                entry
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .collect()
    }

    fn index_of(tree: &FileTree, name: &str) -> usize {
        tree.entries()
            .iter()
            .position(|entry| entry.path.file_name().is_some_and(|file| file == name))
            .expect("entry exists")
    }

    #[test]
    fn lists_directory_entries_with_directories_first() {
        let dir = TempDir::new();
        fs::write(dir.join("b.txt"), "").unwrap();
        fs::write(dir.join("a.txt"), "").unwrap();
        fs::create_dir(dir.join("folder")).unwrap();

        let tree = FileTree::new(dir.path().to_path_buf());

        assert_eq!(named(&tree), vec!["folder", "a.txt", "b.txt"]);
        assert!(tree.entries()[0].is_dir);
    }

    #[test]
    fn ignores_gitignored_entries() {
        let dir = TempDir::new();
        fs::write(dir.join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(dir.join("ignored.txt"), "").unwrap();
        fs::write(dir.join("kept.txt"), "").unwrap();

        let tree = FileTree::new(dir.path().to_path_buf());
        let names = named(&tree);

        assert!(names.contains(&"kept.txt".to_owned()));
        assert!(!names.contains(&"ignored.txt".to_owned()));
    }

    #[test]
    fn expands_and_collapses_a_directory() {
        let dir = TempDir::new();
        fs::create_dir(dir.join("folder")).unwrap();
        fs::write(dir.join("folder/nested.txt"), "").unwrap();

        let mut tree = FileTree::new(dir.path().to_path_buf());
        assert_eq!(named(&tree), vec!["folder"]);

        tree.select(index_of(&tree, "folder"), 10);
        assert!(tree.expand_selected());
        assert!(tree.is_expanded(&dir.join("folder")));
        assert_eq!(named(&tree), vec!["folder", "nested.txt"]);
        assert_eq!(tree.entries()[1].depth, 1);

        assert!(tree.collapse_selected());
        assert!(!tree.is_expanded(&dir.join("folder")));
        assert_eq!(named(&tree), vec!["folder"]);
    }

    #[test]
    fn right_expands_then_enters_child_and_left_returns_to_parent() {
        let dir = TempDir::new();
        fs::create_dir(dir.join("folder")).unwrap();
        fs::write(dir.join("folder/nested.txt"), "").unwrap();

        let mut tree = FileTree::new(dir.path().to_path_buf());
        assert!(tree.right());
        assert_eq!(tree.selected(), 0);
        assert!(tree.right());
        assert_eq!(tree.selected(), 1);
        assert!(tree.collapse_selected());
        assert_eq!(tree.selected(), 0);
        assert!(tree.is_expanded(&dir.join("folder")));
        assert!(tree.collapse_selected());
        assert!(!tree.is_expanded(&dir.join("folder")));
    }

    #[test]
    fn activate_returns_file_paths_and_toggles_directories() {
        let dir = TempDir::new();
        fs::create_dir(dir.join("folder")).unwrap();
        fs::write(dir.join("file.txt"), "").unwrap();

        let mut tree = FileTree::new(dir.path().to_path_buf());
        tree.select(index_of(&tree, "folder"), 10);
        assert_eq!(tree.activate(), None);
        assert!(tree.is_expanded(&dir.join("folder")));

        tree.select(index_of(&tree, "file.txt"), 10);
        assert_eq!(tree.activate(), Some(dir.join("file.txt")));
    }

    #[test]
    fn selection_and_scroll_stay_in_bounds() {
        let dir = TempDir::new();
        for index in 0..6 {
            fs::write(dir.join(format!("file{index}.txt")), "").unwrap();
        }

        let mut tree = FileTree::new(dir.path().to_path_buf());
        assert!(!tree.move_selection(-1, 2));
        assert_eq!(tree.selected(), 0);

        assert!(tree.move_selection(5, 2));
        assert_eq!(tree.selected(), 5);
        assert_eq!(tree.scroll(), 4);
        assert!(!tree.move_selection(1, 2));
        assert_eq!(tree.selected(), 5);

        assert!(tree.move_selection(-5, 2));
        assert_eq!(tree.selected(), 0);
        assert_eq!(tree.scroll(), 0);
    }

    #[test]
    fn row_lookup_uses_scroll_offset() {
        let dir = TempDir::new();
        for index in 0..4 {
            fs::write(dir.join(format!("file{index}.txt")), "").unwrap();
        }

        let mut tree = FileTree::new(dir.path().to_path_buf());
        tree.select(3, 2);
        assert_eq!(tree.scroll(), 2);
        assert_eq!(tree.index_at_row(0), Some(2));
        assert_eq!(tree.index_at_row(1), Some(3));
        assert_eq!(tree.index_at_row(2), None);
    }

    #[test]
    fn visibility_updates_revision() {
        let dir = TempDir::new();
        let mut tree = FileTree::new(dir.path().to_path_buf());
        let revision = tree.revision();

        tree.set_visible(true);

        assert!(tree.visible());
        assert_ne!(tree.revision(), revision);
    }
}
