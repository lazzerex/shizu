use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::buffer::{Buffer, CharIdx};
use crate::theme::Scope;

mod languages;

pub use languages::SyntaxLanguage;

use languages::CAPTURES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub scope: Scope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct CacheKey {
    identity: u64,
    revision: u64,
    parse_first: usize,
    parse_count: usize,
    language: Option<SyntaxLanguage>,
}

const WINDOW_LINES: usize = 128;
const CONTEXT_LINES: usize = 64;

pub struct SyntaxHighlighter {
    highlighter: Highlighter,
    language: Option<SyntaxLanguage>,
    config: Option<HighlightConfiguration>,
    key: CacheKey,
    lines: Vec<Vec<HighlightSpan>>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            highlighter: Highlighter::new(),
            language: None,
            config: None,
            key: CacheKey::default(),
            lines: Vec::new(),
        }
    }

    pub fn ensure(&mut self, buffer: &Buffer, first_line: usize, line_count: usize) {
        let language = SyntaxLanguage::from_path(buffer.path());
        let total = buffer.len_lines();
        let block_start = first_line / WINDOW_LINES * WINDOW_LINES;
        let parse_first = block_start.saturating_sub(CONTEXT_LINES);
        let visible_end = first_line.saturating_add(line_count);
        let window_end = block_start.saturating_add(WINDOW_LINES).max(visible_end);
        let parse_end = window_end.min(total);
        let parse_count = parse_end.saturating_sub(parse_first);
        let key = CacheKey {
            identity: buffer.identity(),
            revision: buffer.revision(),
            parse_first,
            parse_count,
            language,
        };
        if key == self.key && self.lines.len() == parse_count {
            return;
        }
        self.key = key;
        self.lines = vec![Vec::new(); parse_count];
        let Some(language) = language else {
            return;
        };
        self.ensure_language(language);
        let Some(config) = self.config.as_ref() else {
            return;
        };
        if parse_count == 0 {
            return;
        }
        let start = buffer.line_start(parse_first);
        let end = if parse_end < total {
            buffer.line_start(parse_end)
        } else {
            CharIdx::new(buffer.len_chars())
        };
        let text = buffer.text_range(start, end);
        let spans = raw_spans(&mut self.highlighter, config, &text);
        fill_lines(&mut self.lines, &text, 0, parse_count, &spans);
    }

    pub fn line(&self, line: usize) -> &[HighlightSpan] {
        line.checked_sub(self.key.parse_first)
            .and_then(|index| self.lines.get(index))
            .map_or(&[], Vec::as_slice)
    }

    fn ensure_language(&mut self, language: SyntaxLanguage) {
        if self.language == Some(language) {
            return;
        }
        self.language = Some(language);
        self.config = language.config();
    }

    #[cfg(test)]
    fn highlight_text(
        &mut self,
        language: SyntaxLanguage,
        text: &str,
        first_line: usize,
        line_count: usize,
    ) -> Vec<Vec<HighlightSpan>> {
        self.ensure_language(language);
        let mut lines = vec![Vec::new(); line_count];
        let Some(config) = self.config.as_ref() else {
            return lines;
        };
        let spans = raw_spans(&mut self.highlighter, config, text);
        fill_lines(&mut lines, text, first_line, line_count, &spans);
        lines
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
struct RawSpan {
    start: usize,
    end: usize,
    scope: Scope,
}

fn raw_spans(
    highlighter: &mut Highlighter,
    config: &HighlightConfiguration,
    text: &str,
) -> Vec<RawSpan> {
    let mut spans = Vec::new();
    let mut stack: Vec<Option<Scope>> = Vec::new();
    let Ok(events) = highlighter.highlight(config, text.as_bytes(), None, None, |_| None) else {
        return spans;
    };
    for event in events {
        let Ok(event) = event else {
            break;
        };
        match event {
            HighlightEvent::HighlightStart(highlight) => {
                stack.push(CAPTURES.get(highlight.0).map(|(_, scope)| *scope));
            }
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
            HighlightEvent::Source { start, end } => {
                let scope = stack.iter().rev().flatten().next().copied();
                if let Some(scope) = scope {
                    spans.push(RawSpan { start, end, scope });
                }
            }
        }
    }
    spans
}

fn fill_lines(
    lines: &mut [Vec<HighlightSpan>],
    text: &str,
    first_line: usize,
    line_count: usize,
    spans: &[RawSpan],
) {
    let starts = line_starts(text);
    let last_line = first_line.saturating_add(line_count).min(starts.len());
    if first_line >= last_line {
        return;
    }
    let visible_start = starts[first_line];
    let visible_end = starts.get(last_line).copied().unwrap_or(text.len());
    for span in spans {
        if span.end <= visible_start || span.start >= visible_end {
            continue;
        }
        push_span(lines, text, &starts, first_line, last_line, span);
    }
}

fn push_span(
    lines: &mut [Vec<HighlightSpan>],
    text: &str,
    starts: &[usize],
    first_line: usize,
    last_line: usize,
    span: &RawSpan,
) {
    if span.start >= span.end {
        return;
    }
    let start_line = line_of(starts, span.start);
    let end_line = line_of(starts, span.end.saturating_sub(1));
    let from = start_line.max(first_line);
    let to = end_line.min(last_line.saturating_sub(1));
    if from > to {
        return;
    }
    for line in from..=to {
        let line_start = starts[line];
        let line_end = starts.get(line + 1).copied().unwrap_or(text.len());
        let inter_start = span.start.max(line_start);
        let inter_end = span.end.min(line_end);
        if inter_start >= inter_end
            || !text.is_char_boundary(inter_start)
            || !text.is_char_boundary(inter_end)
        {
            continue;
        }
        let start = text[line_start..inter_start].chars().count();
        let end = start + text[inter_start..inter_end].chars().count();
        if end > start {
            lines[line - first_line].push(HighlightSpan {
                start,
                end,
                scope: span.scope,
            });
        }
    }
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn line_of(starts: &[usize], offset: usize) -> usize {
    starts
        .partition_point(|&start| start <= offset)
        .saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;
    use std::fs;
    use std::path::Path;

    use super::{SyntaxHighlighter, SyntaxLanguage};
    use crate::buffer::Buffer;
    use crate::test_support::TempDir;
    use crate::theme::Scope;

    #[test]
    fn highlights_rust_keyword_number_string_and_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        let text = "fn main() {\n    let value = 42; // note\n    let text = \"hi\";\n}\n";
        let lines = highlighter.highlight_text(SyntaxLanguage::Rust, text, 0, 4);

        assert!(lines[0].iter().any(|span| span.scope == Scope::Keyword));
        assert!(lines[1].iter().any(|span| span.scope == Scope::Constant));
        assert!(lines[1].iter().any(|span| span.scope == Scope::Comment));
        assert!(lines[2].iter().any(|span| span.scope == Scope::String));
    }

    #[test]
    fn highlights_json_numbers() {
        let mut highlighter = SyntaxHighlighter::new();
        let text = "{\"value\": 42}\n";
        let lines = highlighter.highlight_text(SyntaxLanguage::Json, text, 0, 1);

        assert!(lines[0].iter().any(|span| span.scope == Scope::Number));
        assert!(lines[0].iter().any(|span| span.scope == Scope::String));
    }

    #[test]
    fn block_comment_spans_lines() {
        let mut highlighter = SyntaxHighlighter::new();
        let text = "/* a\nb */\nfn x() {}\n";
        let lines = highlighter.highlight_text(SyntaxLanguage::Rust, text, 0, 3);

        assert!(lines[0].iter().any(|span| span.scope == Scope::Comment));
        assert!(lines[1].iter().any(|span| span.scope == Scope::Comment));
    }

    #[test]
    fn only_requested_lines_are_filled() {
        let mut highlighter = SyntaxHighlighter::new();
        let text = "fn one() {}\nfn two() {}\nfn three() {}\n";
        let lines = highlighter.highlight_text(SyntaxLanguage::Rust, text, 1, 1);

        assert_eq!(lines.len(), 1);
        assert!(lines[0].iter().any(|span| span.scope == Scope::Keyword));
    }

    #[test]
    fn highlights_keywords_for_each_language() {
        let cases = [
            (SyntaxLanguage::Python, "def run(): return 1  # note\n"),
            (SyntaxLanguage::JavaScript, "const value = 1; // note\n"),
            (SyntaxLanguage::Jsx, "const view = <div />; // note\n"),
            (SyntaxLanguage::TypeScript, "interface Value {} // note\n"),
            (
                SyntaxLanguage::Tsx,
                "const view: Value = <div />; // note\n",
            ),
            (SyntaxLanguage::Go, "func run() {} // note\n"),
            (SyntaxLanguage::C, "int main(void) { return 0; } // note\n"),
            (SyntaxLanguage::Bash, "if true; then echo hi; fi # note\n"),
        ];
        let mut highlighter = SyntaxHighlighter::new();
        for (language, text) in cases {
            let lines = highlighter.highlight_text(language, text, 0, 1);
            assert!(
                lines[0].iter().any(|span| span.scope == Scope::Keyword),
                "{language:?} keyword"
            );
            assert!(
                lines[0].iter().any(|span| span.scope == Scope::Comment),
                "{language:?} comment"
            );
        }
    }

    #[test]
    fn language_comes_from_extension() {
        let cases = [
            ("main.rs", SyntaxLanguage::Rust),
            ("data.json", SyntaxLanguage::Json),
            ("config.toml", SyntaxLanguage::Toml),
            ("script.py", SyntaxLanguage::Python),
            ("stub.pyi", SyntaxLanguage::Python),
            ("app.js", SyntaxLanguage::JavaScript),
            ("app.mjs", SyntaxLanguage::JavaScript),
            ("app.cjs", SyntaxLanguage::JavaScript),
            ("view.jsx", SyntaxLanguage::Jsx),
            ("app.ts", SyntaxLanguage::TypeScript),
            ("app.mts", SyntaxLanguage::TypeScript),
            ("app.cts", SyntaxLanguage::TypeScript),
            ("view.tsx", SyntaxLanguage::Tsx),
            ("main.go", SyntaxLanguage::Go),
            ("main.c", SyntaxLanguage::C),
            ("main.h", SyntaxLanguage::C),
            ("run.sh", SyntaxLanguage::Bash),
            ("run.bash", SyntaxLanguage::Bash),
        ];
        for (path, expected) in cases {
            assert_eq!(
                SyntaxLanguage::from_path(Some(Path::new(path))),
                Some(expected),
                "{path}"
            );
        }
        assert_eq!(
            SyntaxLanguage::from_path(Some(Path::new("MAIN.RS"))),
            Some(SyntaxLanguage::Rust)
        );
        assert_eq!(
            SyntaxLanguage::from_path(Some(Path::new("notes.txt"))),
            None
        );
        assert_eq!(SyntaxLanguage::from_path(None), None);
    }

    #[test]
    fn window_limits_parsed_lines() {
        let dir = TempDir::new();
        let path = dir.join("big.rs");
        let mut text = String::new();
        for index in 0..2000 {
            writeln!(text, "fn item_{index}() {{}}").expect("write text");
        }
        fs::write(&path, text).expect("write source");
        let buffer = Buffer::open(&path).expect("open source");
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.ensure(&buffer, 1500, 20);

        assert!(!highlighter.line(1500).is_empty());
        assert!(highlighter.line(0).is_empty());
        assert!(highlighter.line(1999).is_empty());
    }

    #[test]
    fn ensure_caches_by_revision() {
        let dir = TempDir::new();
        let path = dir.join("sample.rs");
        fs::write(&path, "fn main() {}\n").expect("write source");
        let mut buffer = Buffer::open(&path).expect("open source");
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.ensure(&buffer, 0, 2);
        assert!(!highlighter.line(0).is_empty());

        buffer.insert(crate::buffer::CharIdx::new(0), "// x\n");
        highlighter.ensure(&buffer, 0, 2);
        assert!(
            highlighter
                .line(0)
                .iter()
                .any(|span| span.scope == Scope::Comment)
        );
    }
}
