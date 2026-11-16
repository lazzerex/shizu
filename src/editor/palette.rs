use std::path::PathBuf;

use super::{Editor, Focus};

impl Editor {
    pub(crate) fn palette_height(&self) -> u16 {
        self.viewport.height.saturating_sub(2).min(12)
    }

    pub(super) fn open_palette(&mut self) {
        let root = self.file_root().unwrap_or_else(|| PathBuf::from("."));
        self.palette.open(&root);
        self.focus = Focus::Palette;
        self.bloom.clear();
    }

    pub(super) fn palette_up(&mut self) {
        self.move_palette_selection(-1);
    }

    pub(super) fn palette_down(&mut self) {
        self.move_palette_selection(1);
    }

    pub(super) fn palette_insert(&mut self, character: char) {
        self.palette.insert(character);
    }

    pub(super) fn palette_backspace(&mut self) {
        self.palette.backspace();
    }

    pub(super) fn palette_accept(&mut self) {
        let Some(entry) = self.palette.selected_entry().cloned() else {
            self.focus = Focus::Editor;
            return;
        };
        self.focus = Focus::Editor;
        if let Some(path) = entry.path {
            self.open_path(path);
        } else if let Some(command) = entry.command {
            self.apply(command);
        }
    }

    fn move_palette_selection(&mut self, delta: isize) {
        let height = usize::from(self.palette_height().saturating_sub(2));
        self.palette.move_selection(delta, height);
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

    fn editor_with_files() -> (TempDir, Editor) {
        let dir = TempDir::new();
        fs::write(dir.join("alpha.txt"), "alpha\n").unwrap();
        fs::write(dir.join("beta.txt"), "beta\n").unwrap();
        let mut editor = Editor::new(Buffer::open(dir.join("alpha.txt")).unwrap());
        editor.resize(60, 10);
        (dir, editor)
    }

    fn type_palette(editor: &mut Editor, text: &str) {
        for character in text.chars() {
            editor.apply(Command::PaletteInsert(character));
        }
    }

    #[test]
    fn palette_lists_files_and_filters_by_query() {
        let (_dir, mut editor) = editor_with_files();

        assert!(editor.apply(Command::OpenPalette));
        assert_eq!(editor.focus(), Focus::Palette);
        assert!(editor.palette().is_some());

        type_palette(&mut editor, "beta");

        let palette = editor.palette().expect("palette open");
        assert_eq!(palette.match_count(), 1);
        assert_eq!(
            palette.selected_entry().map(|entry| entry.label.as_str()),
            Some("beta.txt")
        );
    }

    #[test]
    fn accepting_a_file_opens_it() {
        let (dir, mut editor) = editor_with_files();
        editor.apply(Command::OpenPalette);
        type_palette(&mut editor, "beta");

        assert!(editor.apply(Command::PaletteAccept));

        assert_eq!(editor.focus(), Focus::Editor);
        assert_eq!(editor.buffer().path(), Some(dir.join("beta.txt").as_path()));
    }

    #[test]
    fn prefix_switches_to_commands_and_runs_them() {
        let (dir, mut editor) = editor_with_files();
        editor.apply(Command::Insert('x'));
        assert!(editor.buffer().is_dirty());

        editor.apply(Command::OpenPalette);
        type_palette(&mut editor, ">save");
        assert_eq!(editor.focus(), Focus::Palette);

        assert!(editor.apply(Command::PaletteAccept));

        assert_eq!(editor.focus(), Focus::Editor);
        assert!(!editor.buffer().is_dirty());
        assert_eq!(fs::read(dir.join("alpha.txt")).unwrap(), b"xalpha\n");
    }

    #[test]
    fn escape_closes_the_palette_without_running_anything() {
        let (_dir, mut editor) = editor_with_files();
        editor.apply(Command::OpenPalette);
        type_palette(&mut editor, "beta");

        assert!(editor.apply(Command::Cancel));

        assert_eq!(editor.focus(), Focus::Editor);
        assert_eq!(editor.buffer().text(), "alpha\n");
    }

    #[test]
    fn navigation_moves_the_selection_and_scrolls() {
        let (dir, mut editor) = editor_with_files();
        for index in 0..12 {
            fs::write(dir.join(format!("file{index}.txt")), "").unwrap();
        }
        editor.apply(Command::OpenPalette);

        assert!(editor.apply(Command::PaletteDown));
        assert_eq!(editor.palette().expect("palette open").selected(), 1);

        for _ in 0..11 {
            editor.apply(Command::PaletteDown);
        }
        let palette = editor.palette().expect("palette open");
        assert_eq!(palette.selected(), 12);
        assert!(palette.scroll() > 0);
    }
}
