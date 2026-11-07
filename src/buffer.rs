use std::ffi::OsString;
use std::fs::{self, Metadata, OpenOptions, Permissions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharIdx(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId(usize);

impl BufferId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

impl CharIdx {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    ops: Vec<Op>,
}

impl Transaction {
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    Insert { at: CharIdx, text: String },
    Delete { at: CharIdx, text: String },
}

#[derive(Debug)]
pub struct Buffer {
    rope: Rope,
    line_ending: LineEnding,
    revision: u64,
    saved_revision: u64,
    path: Option<PathBuf>,
    disk: Option<Fingerprint>,
}

#[derive(Debug)]
pub enum BufferError {
    Io(io::Error),
    InvalidUtf8 { path: PathBuf, offset: usize },
    ChangedOnDisk(PathBuf),
    ReadOnly(PathBuf),
    NoPath,
}

impl std::fmt::Display for BufferError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::InvalidUtf8 { path, offset } => {
                write!(
                    formatter,
                    "{}: invalid UTF-8 at byte {offset}",
                    path.display()
                )
            }
            Self::ChangedOnDisk(path) => {
                write!(
                    formatter,
                    "{}: file changed on disk since it was opened",
                    path.display()
                )
            }
            Self::ReadOnly(path) => write!(formatter, "{}: file is read-only", path.display()),
            Self::NoPath => write!(formatter, "buffer has no file path"),
        }
    }
}

impl std::error::Error for BufferError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidUtf8 { .. }
            | Self::ChangedOnDisk(_)
            | Self::ReadOnly(_)
            | Self::NoPath => None,
        }
    }
}

impl From<io::Error> for BufferError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fingerprint {
    len: u64,
    modified: Option<SystemTime>,
}

impl Buffer {
    pub fn new() -> Self {
        Self::from_text("")
    }

    pub fn from_text(text: &str) -> Self {
        let (text, line_ending) = normalize_line_endings(text);
        Self {
            rope: Rope::from_str(&text),
            line_ending,
            revision: 0,
            saved_revision: 0,
            path: None,
            disk: None,
        }
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn text_range(&self, from: CharIdx, to: CharIdx) -> String {
        assert!(from.0 <= to.0 && to.0 <= self.len_chars());
        self.rope.slice(from.0..to.0).to_string()
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn line_text(&self, line: usize) -> String {
        assert!(line < self.len_lines());
        let (start, content_len) = self.line_bounds(line);
        self.rope.slice(start..start + content_len).to_string()
    }

    pub fn line_start(&self, line: usize) -> CharIdx {
        assert!(line < self.len_lines());
        CharIdx(self.rope.line_to_char(line))
    }

    pub fn line_end(&self, line: usize) -> CharIdx {
        assert!(line < self.len_lines());
        let (start, content_len) = self.line_bounds(line);
        CharIdx(start + content_len)
    }

    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    pub fn mark_saved(&mut self) {
        self.saved_revision = self.revision;
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, BufferError> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;
        let text = String::from_utf8(bytes).map_err(|error| BufferError::InvalidUtf8 {
            path: path.to_path_buf(),
            offset: error.utf8_error().valid_up_to(),
        })?;
        let metadata = fs::metadata(path)?;
        let mut buffer = Self::from_text(&text);
        buffer.path = Some(path.to_path_buf());
        buffer.disk = Some(fingerprint(&metadata));
        Ok(buffer)
    }

    pub fn save(&mut self) -> Result<(), BufferError> {
        let path = self.path.clone().ok_or(BufferError::NoPath)?;
        let metadata = fs::metadata(&path).map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                BufferError::ChangedOnDisk(path.clone())
            } else {
                BufferError::Io(error)
            }
        })?;
        if self.disk != Some(fingerprint(&metadata)) {
            return Err(BufferError::ChangedOnDisk(path));
        }
        if metadata.permissions().readonly() {
            return Err(BufferError::ReadOnly(path));
        }
        write_atomic(&path, &self.encoded(), metadata.permissions())?;
        let metadata = fs::metadata(&path)?;
        self.disk = Some(fingerprint(&metadata));
        self.saved_revision = self.revision;
        Ok(())
    }

    fn encoded(&self) -> Vec<u8> {
        let text = self.rope.to_string().replace("\r\n", "\n");
        match self.line_ending {
            LineEnding::Lf => text.into_bytes(),
            LineEnding::Crlf => text.replace('\n', "\r\n").into_bytes(),
        }
    }

    pub fn line_col(&self, at: CharIdx) -> LineCol {
        assert!(at.0 <= self.len_chars());
        let line = self.rope.char_to_line(at.0);
        let start = self.rope.line_to_char(line);
        LineCol {
            line,
            col: at.0 - start,
        }
    }

    pub fn char_idx(&self, pos: LineCol) -> Option<CharIdx> {
        if pos.line >= self.len_lines() {
            return None;
        }
        let (start, content_len) = self.line_bounds(pos.line);
        if pos.col > content_len {
            return None;
        }
        Some(CharIdx(start + pos.col))
    }

    pub fn next_grapheme(&self, at: CharIdx) -> CharIdx {
        let len = self.len_chars();
        if at.0 >= len {
            return CharIdx(len);
        }
        let line = self.rope.char_to_line(at.0);
        let (start, content_len) = self.line_bounds(line);
        let offset = at.0 - start;
        if offset >= content_len {
            return CharIdx((start + content_len + 1).min(len));
        }
        let mut chars = 0;
        for grapheme in self.line_text(line).graphemes(true) {
            chars += grapheme.chars().count();
            if chars > offset {
                return CharIdx(start + chars);
            }
        }
        CharIdx(start + content_len)
    }

    pub fn prev_grapheme(&self, at: CharIdx) -> CharIdx {
        if at.0 == 0 {
            return CharIdx(0);
        }
        let at = at.0.min(self.len_chars());
        let line = self.rope.char_to_line(at);
        let (start, content_len) = self.line_bounds(line);
        let offset = at - start;
        if offset == 0 {
            let (prev_start, prev_len) = self.line_bounds(line - 1);
            return CharIdx(prev_start + prev_len);
        }
        let offset = offset.min(content_len);
        let mut chars = 0;
        for grapheme in self.line_text(line).graphemes(true) {
            let width = grapheme.chars().count();
            if chars + width >= offset {
                return CharIdx(start + chars);
            }
            chars += width;
        }
        CharIdx(start)
    }

    pub fn insert(&mut self, at: CharIdx, text: &str) -> Transaction {
        assert!(at.0 <= self.len_chars());
        if text.is_empty() {
            return Transaction { ops: Vec::new() };
        }
        let tx = Transaction {
            ops: vec![Op::Insert {
                at,
                text: text.to_owned(),
            }],
        };
        self.apply(&tx);
        tx
    }

    pub fn delete(&mut self, at: CharIdx, len: usize) -> Transaction {
        let end = at.0.saturating_add(len);
        assert!(end <= self.len_chars());
        if len == 0 {
            return Transaction { ops: Vec::new() };
        }
        let text = self.rope.slice(at.0..end).to_string();
        let tx = Transaction {
            ops: vec![Op::Delete { at, text }],
        };
        self.apply(&tx);
        tx
    }

    pub fn replace(&mut self, from: CharIdx, to: CharIdx, text: &str) -> Transaction {
        assert!(from.0 <= to.0 && to.0 <= self.len_chars());
        let mut ops = Vec::new();
        if to.0 > from.0 {
            ops.push(Op::Delete {
                at: from,
                text: self.rope.slice(from.0..to.0).to_string(),
            });
        }
        if !text.is_empty() {
            ops.push(Op::Insert {
                at: from,
                text: text.to_owned(),
            });
        }
        let tx = Transaction { ops };
        self.apply(&tx);
        tx
    }

    pub fn apply(&mut self, tx: &Transaction) {
        if tx.ops.is_empty() {
            return;
        }
        for op in &tx.ops {
            match op {
                Op::Insert { at, text } => self.rope.insert(at.0, text),
                Op::Delete { at, text } => {
                    let end = at.0 + text.chars().count();
                    self.rope.remove(at.0..end);
                }
            }
        }
        self.revision += 1;
    }

    pub fn revert(&mut self, tx: &Transaction) {
        if tx.ops.is_empty() {
            return;
        }
        for op in tx.ops.iter().rev() {
            match op {
                Op::Insert { at, text } => {
                    let end = at.0 + text.chars().count();
                    self.rope.remove(at.0..end);
                }
                Op::Delete { at, text } => self.rope.insert(at.0, text),
            }
        }
        self.revision += 1;
    }

    fn line_bounds(&self, line: usize) -> (usize, usize) {
        let start = self.rope.line_to_char(line);
        let len = self.rope.line(line).len_chars();
        if len > 0 && self.rope.char(start + len - 1) == '\n' {
            (start, len - 1)
        } else {
            (start, len)
        }
    }
}

fn fingerprint(metadata: &Metadata) -> Fingerprint {
    Fingerprint {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    }
}

fn write_atomic(path: &Path, bytes: &[u8], permissions: Permissions) -> io::Result<()> {
    let temp = temp_path(path)?;
    let result = write_temp(&temp, bytes, permissions).and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn write_temp(path: &Path, bytes: &[u8], permissions: Permissions) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.set_permissions(permissions)?;
    file.sync_all()
}

fn temp_path(path: &Path) -> io::Result<PathBuf> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "path has no file name"))?;
    let mut temp_name = OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(".shizu-{}-{counter}.tmp", process::id()));
    Ok(path.with_file_name(temp_name))
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_line_endings(text: &str) -> (String, LineEnding) {
    if text.contains("\r\n") {
        (text.replace("\r\n", "\n"), LineEnding::Crlf)
    } else {
        (text.to_owned(), LineEnding::Lf)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Buffer, BufferError, CharIdx, LineCol, LineEnding};
    use crate::test_support::TempDir;

    fn idx(index: usize) -> CharIdx {
        CharIdx::new(index)
    }

    fn buffer(text: &str) -> Buffer {
        Buffer::from_text(text)
    }

    #[test]
    fn insert_at_start_middle_and_end() {
        let mut start = buffer("hello");
        start.insert(idx(0), "X");
        assert_eq!(start.text(), "Xhello");

        let mut middle = buffer("hello");
        middle.insert(idx(2), "Y");
        assert_eq!(middle.text(), "heYllo");

        let mut end = buffer("hello");
        end.insert(idx(5), "Z");
        assert_eq!(end.text(), "helloZ");
    }

    #[test]
    fn delete_at_start_middle_and_end() {
        let mut start = buffer("hello");
        start.delete(idx(0), 1);
        assert_eq!(start.text(), "ello");

        let mut middle = buffer("hello");
        middle.delete(idx(1), 2);
        assert_eq!(middle.text(), "hlo");

        let mut end = buffer("hello");
        end.delete(idx(4), 1);
        assert_eq!(end.text(), "hell");
    }

    #[test]
    fn insert_and_delete_on_empty_buffer() {
        let mut empty = Buffer::new();
        assert!(empty.is_empty());
        assert_eq!(empty.len_lines(), 1);

        empty.insert(idx(0), "a\nb");
        assert_eq!(empty.text(), "a\nb");
        assert_eq!(empty.len_lines(), 2);

        empty.delete(idx(0), 3);
        assert!(empty.is_empty());
    }

    #[test]
    fn multiline_edits() {
        let mut text = buffer("one\ntwo");
        text.insert(idx(3), "\n");
        assert_eq!(text.text(), "one\n\ntwo");

        text.delete(idx(2), 3);
        assert_eq!(text.text(), "ontwo");
    }

    #[test]
    fn delete_across_line_boundary() {
        let mut text = buffer("abc\ndef");
        text.delete(idx(2), 3);
        assert_eq!(text.text(), "abef");
    }

    #[test]
    fn emoji_graphemes() {
        let text = buffer("a👩‍👩‍👧‍👦b");
        assert_eq!(text.next_grapheme(idx(1)), idx(8));
        assert_eq!(text.prev_grapheme(idx(8)), idx(1));
        assert_eq!(text.line_col(idx(8)), LineCol { line: 0, col: 8 });
    }

    #[test]
    fn combining_graphemes() {
        let text = buffer("e\u{301}x");
        assert_eq!(text.next_grapheme(idx(0)), idx(2));
        assert_eq!(text.prev_grapheme(idx(2)), idx(0));
    }

    #[test]
    fn crlf_content_is_normalized() {
        let text = Buffer::from_text("a\r\nb\r\n");
        assert_eq!(text.text(), "a\nb\n");
        assert_eq!(text.line_ending(), LineEnding::Crlf);
        assert_eq!(text.len_lines(), 3);
        assert_eq!(text.line_col(idx(2)), LineCol { line: 1, col: 0 });
        assert_eq!(text.char_idx(LineCol { line: 1, col: 0 }), Some(idx(2)));
    }

    #[test]
    fn lf_content_keeps_lf() {
        let text = Buffer::from_text("a\nb");
        assert_eq!(text.text(), "a\nb");
        assert_eq!(text.line_ending(), LineEnding::Lf);
    }

    #[test]
    fn grapheme_movement_crosses_lines() {
        let text = buffer("ab\ncd");
        assert_eq!(text.next_grapheme(idx(2)), idx(3));
        assert_eq!(text.prev_grapheme(idx(3)), idx(2));
        assert_eq!(text.line_col(idx(2)), LineCol { line: 0, col: 2 });
        assert_eq!(text.line_col(idx(3)), LineCol { line: 1, col: 0 });
    }

    #[test]
    fn grapheme_movement_stops_at_buffer_ends() {
        let text = buffer("ab");
        assert_eq!(text.next_grapheme(idx(2)), idx(2));
        assert_eq!(text.prev_grapheme(idx(0)), idx(0));
    }

    #[test]
    fn line_col_round_trip() {
        let text = buffer("ab\ncde\n");
        for index in 0..=text.len_chars() {
            let at = idx(index);
            let pos = text.line_col(at);
            assert_eq!(text.char_idx(pos), Some(at));
        }
    }

    #[test]
    fn char_idx_rejects_out_of_range() {
        let text = buffer("ab\ncd");
        assert_eq!(text.char_idx(LineCol { line: 3, col: 0 }), None);
        assert_eq!(text.char_idx(LineCol { line: 0, col: 3 }), None);
    }

    #[test]
    fn transaction_revert_restores_text() {
        let mut text = buffer("hello");
        let inserted = text.insert(idx(2), "XY");
        assert_eq!(text.text(), "heXYllo");
        text.revert(&inserted);
        assert_eq!(text.text(), "hello");

        let deleted = text.delete(idx(1), 3);
        assert_eq!(text.text(), "ho");
        text.revert(&deleted);
        assert_eq!(text.text(), "hello");
    }

    #[test]
    fn replace_swaps_range_and_reverts() {
        let mut text = buffer("abcdef");
        let tx = text.replace(idx(2), idx(4), "XY");
        assert_eq!(text.text(), "abXYef");
        text.revert(&tx);
        assert_eq!(text.text(), "abcdef");
    }

    #[test]
    fn replace_with_empty_text_deletes() {
        let mut text = buffer("abcdef");
        let tx = text.replace(idx(2), idx(4), "");
        assert_eq!(text.text(), "abef");
        text.revert(&tx);
        assert_eq!(text.text(), "abcdef");
    }

    #[test]
    fn empty_replace_is_empty_transaction() {
        let mut text = buffer("abc");
        let tx = text.replace(idx(1), idx(1), "");
        assert!(tx.is_empty());
        assert_eq!(text.text(), "abc");
    }

    #[test]
    fn text_range_slices_across_lines() {
        let text = buffer("abc\ndef");
        assert_eq!(text.text_range(idx(1), idx(6)), "bc\nde");
        assert_eq!(text.text_range(idx(2), idx(2)), "");
    }

    #[test]
    fn dirty_tracks_edits_and_save() {
        let mut text = buffer("x");
        assert!(!text.is_dirty());

        text.insert(idx(0), "a");
        assert!(text.is_dirty());

        text.mark_saved();
        assert!(!text.is_dirty());

        text.delete(idx(0), 1);
        assert!(text.is_dirty());
    }

    #[test]
    fn empty_edits_are_noops() {
        let mut text = buffer("x");
        text.insert(idx(0), "");
        text.delete(idx(0), 0);
        assert_eq!(text.text(), "x");
        assert!(!text.is_dirty());
    }

    #[test]
    fn unmodified_save_is_byte_identical() {
        let dir = TempDir::new();
        let contents = ["", "a", "a\n", "a\nb", "a\r\n", "a\r\nb\r\n"];
        for (index, content) in contents.iter().enumerate() {
            let path = dir.join(&format!("file-{index}.txt"));
            fs::write(&path, content).unwrap();
            let mut buffer = Buffer::open(&path).unwrap();
            buffer.save().unwrap();
            assert_eq!(fs::read(&path).unwrap(), content.as_bytes());
        }
    }

    #[test]
    fn crlf_stays_crlf_after_edit_and_save() {
        let dir = TempDir::new();
        let path = dir.join("crlf.txt");
        fs::write(&path, "a\r\nb\r\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        assert_eq!(buffer.line_ending(), LineEnding::Crlf);
        buffer.insert(idx(1), "x");
        buffer.save().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"ax\r\nb\r\n");
    }

    #[test]
    fn lf_stays_lf_after_edit_and_save() {
        let dir = TempDir::new();
        let path = dir.join("lf.txt");
        fs::write(&path, "a\nb\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        assert_eq!(buffer.line_ending(), LineEnding::Lf);
        buffer.insert(idx(1), "x");
        buffer.save().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"ax\nb\n");
    }

    #[test]
    fn save_preserves_final_newline_state() {
        let dir = TempDir::new();
        for (index, content) in ["a\nb", "a\nb\n"].iter().enumerate() {
            let path = dir.join(&format!("final-{index}.txt"));
            fs::write(&path, content).unwrap();
            let mut buffer = Buffer::open(&path).unwrap();
            buffer.insert(idx(0), "x");
            buffer.save().unwrap();
            assert_eq!(fs::read(&path).unwrap(), format!("x{content}").as_bytes());
        }
    }

    #[test]
    fn save_normalizes_inserted_crlf() {
        let dir = TempDir::new();
        let path = dir.join("mixed.txt");
        fs::write(&path, "a\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(idx(1), "\r\nb");
        buffer.save().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"a\nb\n");
    }

    #[test]
    fn invalid_utf8_is_refused() {
        let dir = TempDir::new();
        let path = dir.join("binary.bin");
        fs::write(&path, [0x66, 0x80]).unwrap();
        let Err(error) = Buffer::open(&path) else {
            panic!("expected invalid UTF-8");
        };
        assert!(matches!(error, BufferError::InvalidUtf8 { .. }));
        assert!(error.to_string().contains("invalid UTF-8"));
    }

    #[test]
    fn read_only_file_fails_without_damage() {
        let dir = TempDir::new();
        let path = dir.join("readonly.txt");
        fs::write(&path, "original\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(idx(0), "x");
        let original = fs::metadata(&path).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        let result = buffer.save();
        assert!(matches!(result, Err(BufferError::ReadOnly(_))));
        assert_eq!(fs::read(&path).unwrap(), b"original\n");
        fs::set_permissions(&path, original).unwrap();
    }

    #[test]
    fn changed_on_disk_is_refused() {
        let dir = TempDir::new();
        let path = dir.join("changed.txt");
        fs::write(&path, "one\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        fs::write(&path, "one\ntwo\n").unwrap();
        buffer.insert(idx(0), "x");
        let result = buffer.save();
        assert!(matches!(result, Err(BufferError::ChangedOnDisk(_))));
        assert_eq!(fs::read(&path).unwrap(), b"one\ntwo\n");
    }

    #[test]
    fn save_leaves_no_temp_file() {
        let dir = TempDir::new();
        let path = dir.join("clean.txt");
        fs::write(&path, "a\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.save().unwrap();
        for entry in fs::read_dir(dir.path()).unwrap() {
            let name = entry.unwrap().file_name();
            assert!(!name.to_string_lossy().contains(".tmp"));
        }
    }

    #[test]
    fn save_twice_succeeds() {
        let dir = TempDir::new();
        let path = dir.join("twice.txt");
        fs::write(&path, "a\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(idx(0), "x");
        buffer.save().unwrap();
        buffer.insert(idx(1), "y");
        buffer.save().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"xya\n");
        assert!(!buffer.is_dirty());
    }
}
