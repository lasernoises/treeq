use clap::ValueEnum;

use crate::Lang;

#[derive(Copy, Clone, ValueEnum)]
pub enum CliLang {
    Rust,
    Php,
    Markdown,
}

impl CliLang {
    pub fn to_lang(&self) -> Lang {
        match self {
            CliLang::Rust => RUST,
            CliLang::Php => PHP,
            CliLang::Markdown => MARKDOWN,
        }
    }
}

pub const RUST: Lang = Lang {
    file_type: "rust",
    language_fn: tree_sitter_rust::LANGUAGE,
};

pub const PHP: Lang = Lang {
    file_type: "php",
    language_fn: tree_sitter_php::LANGUAGE_PHP,
};

pub const MARKDOWN: Lang = Lang {
    file_type: "md",
    language_fn: tree_sitter_md::LANGUAGE,
};
