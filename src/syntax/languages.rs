use std::path::Path;
use std::sync::OnceLock;

use tree_sitter_highlight::HighlightConfiguration;

use crate::theme::Scope;

pub(super) const CAPTURES: &[(&str, Scope)] = &[
    ("keyword", Scope::Keyword),
    ("string", Scope::String),
    ("comment", Scope::Comment),
    ("constant.numeric", Scope::Number),
    ("number", Scope::Number),
    ("boolean", Scope::Constant),
    ("type", Scope::Type),
    ("type.builtin", Scope::Type),
    ("function", Scope::Function),
    ("function.builtin", Scope::Function),
    ("function.method", Scope::Function),
    ("function.special", Scope::Function),
    ("variable", Scope::Variable),
    ("variable.builtin", Scope::Variable),
    ("variable.parameter", Scope::Variable),
    ("operator", Scope::Operator),
    ("punctuation", Scope::Punctuation),
    ("delimiter", Scope::Punctuation),
    ("constant", Scope::Constant),
    ("constant.builtin", Scope::Constant),
    ("attribute", Scope::Attribute),
    ("property", Scope::Property),
    ("tag", Scope::Tag),
    ("escape", Scope::Escape),
    ("constructor", Scope::Type),
    ("label", Scope::Constant),
    ("embedded", Scope::Text),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxLanguage {
    Rust,
    Json,
    Toml,
    Python,
    JavaScript,
    Jsx,
    TypeScript,
    Tsx,
    Go,
    C,
    Bash,
}

fn jsx_highlights() -> &'static str {
    static QUERY: OnceLock<String> = OnceLock::new();
    QUERY.get_or_init(|| {
        format!(
            "{}\n{}",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
        )
    })
}

fn typescript_highlights() -> &'static str {
    static QUERY: OnceLock<String> = OnceLock::new();
    QUERY.get_or_init(|| {
        format!(
            "{}\n{}",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_typescript::HIGHLIGHTS_QUERY
        )
    })
}

fn tsx_highlights() -> &'static str {
    static QUERY: OnceLock<String> = OnceLock::new();
    QUERY.get_or_init(|| {
        format!(
            "{}\n{}\n{}",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
            tree_sitter_typescript::HIGHLIGHTS_QUERY
        )
    })
}

struct LanguageDefinition {
    language: SyntaxLanguage,
    name: &'static str,
    extensions: &'static [&'static str],
    grammar: fn() -> tree_sitter::Language,
    highlights: fn() -> &'static str,
}

const LANGUAGES: &[LanguageDefinition] = &[
    LanguageDefinition {
        language: SyntaxLanguage::Rust,
        name: "rust",
        extensions: &["rs"],
        grammar: || tree_sitter_rust::LANGUAGE.into(),
        highlights: || tree_sitter_rust::HIGHLIGHTS_QUERY,
    },
    LanguageDefinition {
        language: SyntaxLanguage::Json,
        name: "json",
        extensions: &["json"],
        grammar: || tree_sitter_json::LANGUAGE.into(),
        highlights: || tree_sitter_json::HIGHLIGHTS_QUERY,
    },
    LanguageDefinition {
        language: SyntaxLanguage::Toml,
        name: "toml",
        extensions: &["toml"],
        grammar: || tree_sitter_toml_ng::LANGUAGE.into(),
        highlights: || tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
    },
    LanguageDefinition {
        language: SyntaxLanguage::Python,
        name: "python",
        extensions: &["py", "pyi"],
        grammar: || tree_sitter_python::LANGUAGE.into(),
        highlights: || tree_sitter_python::HIGHLIGHTS_QUERY,
    },
    LanguageDefinition {
        language: SyntaxLanguage::JavaScript,
        name: "javascript",
        extensions: &["js", "mjs", "cjs"],
        grammar: || tree_sitter_javascript::LANGUAGE.into(),
        highlights: || tree_sitter_javascript::HIGHLIGHT_QUERY,
    },
    LanguageDefinition {
        language: SyntaxLanguage::Jsx,
        name: "jsx",
        extensions: &["jsx"],
        grammar: || tree_sitter_javascript::LANGUAGE.into(),
        highlights: jsx_highlights,
    },
    LanguageDefinition {
        language: SyntaxLanguage::TypeScript,
        name: "typescript",
        extensions: &["ts", "mts", "cts"],
        grammar: || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        highlights: typescript_highlights,
    },
    LanguageDefinition {
        language: SyntaxLanguage::Tsx,
        name: "tsx",
        extensions: &["tsx"],
        grammar: || tree_sitter_typescript::LANGUAGE_TSX.into(),
        highlights: tsx_highlights,
    },
    LanguageDefinition {
        language: SyntaxLanguage::Go,
        name: "go",
        extensions: &["go"],
        grammar: || tree_sitter_go::LANGUAGE.into(),
        highlights: || tree_sitter_go::HIGHLIGHTS_QUERY,
    },
    LanguageDefinition {
        language: SyntaxLanguage::C,
        name: "c",
        extensions: &["c", "h"],
        grammar: || tree_sitter_c::LANGUAGE.into(),
        highlights: || tree_sitter_c::HIGHLIGHT_QUERY,
    },
    LanguageDefinition {
        language: SyntaxLanguage::Bash,
        name: "bash",
        extensions: &["sh", "bash"],
        grammar: || tree_sitter_bash::LANGUAGE.into(),
        highlights: || tree_sitter_bash::HIGHLIGHT_QUERY,
    },
];

impl SyntaxLanguage {
    pub fn from_path(path: Option<&Path>) -> Option<Self> {
        let extension = path?.extension()?.to_str()?;
        LANGUAGES
            .iter()
            .find(|definition| {
                definition
                    .extensions
                    .iter()
                    .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            })
            .map(|definition| definition.language)
    }

    pub(super) fn config(self) -> Option<HighlightConfiguration> {
        let definition = LANGUAGES
            .iter()
            .find(|definition| definition.language == self)?;
        let mut config = HighlightConfiguration::new(
            (definition.grammar)(),
            definition.name,
            (definition.highlights)(),
            "",
            "",
        )
        .ok()?;
        let names: Vec<&str> = CAPTURES.iter().map(|(name, _)| *name).collect();
        config.configure(&names);
        Some(config)
    }
}
