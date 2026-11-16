use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::command::Command;
use crate::keymap;

const MAX_FILES: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaletteEntry {
    pub(crate) label: String,
    pub(crate) command: Option<Command>,
    pub(crate) path: Option<PathBuf>,
    pub(crate) shortcut: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct Palette {
    query: String,
    files: Vec<PaletteEntry>,
    commands: Vec<PaletteEntry>,
    matches: Vec<usize>,
    selected: usize,
    scroll: usize,
}

impl Palette {
    pub(crate) fn open(&mut self, root: &Path) {
        self.query.clear();
        self.files = collect_files(root);
        self.commands = keymap::command_entries()
            .into_iter()
            .map(|entry| PaletteEntry {
                label: entry.label.to_owned(),
                command: Some(entry.command),
                path: None,
                shortcut: entry.keys,
            })
            .collect();
        self.refresh();
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn command_mode(&self) -> bool {
        self.query.starts_with('>')
    }

    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    pub(crate) fn scroll(&self) -> usize {
        self.scroll
    }

    pub(crate) fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub(crate) fn visible(&self) -> impl Iterator<Item = &PaletteEntry> {
        let candidates = if self.command_mode() {
            &self.commands
        } else {
            &self.files
        };
        self.matches.iter().map(move |&index| &candidates[index])
    }

    pub(crate) fn selected_entry(&self) -> Option<&PaletteEntry> {
        self.visible().nth(self.selected)
    }

    pub(crate) fn insert(&mut self, character: char) {
        self.query.push(character);
        self.refresh();
    }

    pub(crate) fn backspace(&mut self) {
        self.query.pop();
        self.refresh();
    }

    pub(crate) fn move_selection(&mut self, delta: isize, height: usize) -> bool {
        if self.matches.is_empty() {
            return false;
        }
        let last = self.matches.len() - 1;
        let next = self.selected.saturating_add_signed(delta).min(last);
        let changed = next != self.selected;
        self.selected = next;
        self.ensure_visible(height);
        changed
    }

    fn refresh(&mut self) {
        let (candidates, needle) = match self.query.strip_prefix('>') {
            Some(needle) => (&self.commands, needle),
            None => (&self.files, self.query.as_str()),
        };
        let mut scored: Vec<(i32, usize)> = candidates
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                fuzzy_score(&entry.label, needle).map(|score| (score, index))
            })
            .collect();
        scored.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| {
                    candidates[left.1]
                        .label
                        .len()
                        .cmp(&candidates[right.1].label.len())
                })
                .then_with(|| candidates[left.1].label.cmp(&candidates[right.1].label))
        });
        self.matches = scored.into_iter().map(|(_, index)| index).collect();
        self.selected = 0;
        self.scroll = 0;
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
        let max = self.matches.len().saturating_sub(height);
        self.scroll = self.scroll.min(max);
    }
}

fn collect_files(root: &Path) -> Vec<PaletteEntry> {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(true)
        .require_git(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false);
    let mut entries: Vec<PaletteEntry> = builder
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.depth() > 0)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .take(MAX_FILES)
        .map(|entry| {
            let label = entry
                .path()
                .strip_prefix(root)
                .unwrap_or_else(|_| entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            PaletteEntry {
                label,
                command: None,
                path: Some(entry.path().to_path_buf()),
                shortcut: None,
            }
        })
        .collect();
    entries.sort_by_cached_key(|entry| entry.label.to_lowercase());
    entries
}

fn fuzzy_score(candidate: &str, needle: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }
    let haystack: Vec<char> = candidate.to_lowercase().chars().collect();
    let wanted: Vec<char> = needle.to_lowercase().chars().collect();
    let mut score = 0;
    let mut last_match = None;
    let mut next = 0;
    for (index, character) in haystack.iter().enumerate() {
        if wanted.get(next) != Some(character) {
            continue;
        }
        score += 1;
        if last_match.is_some_and(|last| last + 1 == index) {
            score += 5;
        }
        if index == 0
            || haystack
                .get(index - 1)
                .is_some_and(|previous| matches!(previous, '/' | '\\' | '_' | '-' | '.' | ' '))
        {
            score += 8;
        }
        last_match = Some(index);
        next += 1;
        if next == wanted.len() {
            return Some(score);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Palette, PaletteEntry, fuzzy_score};
    use crate::command::Command;

    fn entry(label: &str) -> PaletteEntry {
        PaletteEntry {
            label: label.to_owned(),
            command: None,
            path: Some(PathBuf::from(label)),
            shortcut: None,
        }
    }

    fn command(label: &str, command: Command) -> PaletteEntry {
        PaletteEntry {
            label: label.to_owned(),
            command: Some(command),
            path: None,
            shortcut: Some("Ctrl+S".to_owned()),
        }
    }

    fn palette(files: &[&str], commands: Vec<PaletteEntry>) -> Palette {
        Palette {
            query: String::new(),
            files: files.iter().map(|label| entry(label)).collect(),
            commands,
            matches: Vec::new(),
            selected: 0,
            scroll: 0,
        }
    }

    fn labels(palette: &Palette) -> Vec<&str> {
        palette
            .visible()
            .map(|entry| entry.label.as_str())
            .collect()
    }

    #[test]
    fn fuzzy_score_matches_subsequences_case_insensitively() {
        assert!(fuzzy_score("src/main.rs", "smr").is_some());
        assert!(fuzzy_score("README.md", "readme").is_some());
        assert!(fuzzy_score("src/main.rs", "nope").is_none());
    }

    #[test]
    fn fuzzy_score_prefers_consecutive_matches() {
        let consecutive = fuzzy_score("zmain.rs", "main").expect("consecutive match");
        let scattered = fuzzy_score("zmaixn.rs", "main").expect("scattered match");

        assert!(consecutive > scattered);
    }

    #[test]
    fn fuzzy_score_rewards_word_starts() {
        let word_start = fuzzy_score("src/main.rs", "main").expect("word-start match");
        let middle = fuzzy_score("src/zmain.rs", "main").expect("middle match");

        assert!(word_start > middle);
    }

    #[test]
    fn empty_query_lists_files_and_prefix_lists_commands() {
        let mut palette = palette(
            &["src/lib.rs", "README.md"],
            vec![command("Save", Command::Save)],
        );
        palette.refresh();

        assert_eq!(labels(&palette), vec!["README.md", "src/lib.rs"]);

        palette.insert('>');

        assert!(palette.command_mode());
        assert_eq!(labels(&palette), vec!["Save"]);
    }

    #[test]
    fn query_filters_files_by_fuzzy_match() {
        let mut palette = palette(&["src/main.rs", "src/view.rs", "README.md"], Vec::new());
        palette.refresh();
        palette.insert('m');
        palette.insert('r');

        assert_eq!(labels(&palette), vec!["src/main.rs"]);
    }

    #[test]
    fn selection_and_scroll_stay_in_bounds() {
        let mut palette = palette(&["a", "b", "c", "d"], Vec::new());
        palette.refresh();

        assert!(!palette.move_selection(-1, 2));
        assert_eq!(palette.selected(), 0);

        assert!(palette.move_selection(3, 2));
        assert_eq!(palette.selected(), 3);
        assert_eq!(palette.scroll(), 2);
        assert!(!palette.move_selection(1, 2));
    }
}
