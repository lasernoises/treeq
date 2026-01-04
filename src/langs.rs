use clap::ValueEnum;

use crate::Lang;

#[derive(Copy, Clone, ValueEnum)]
pub enum CliLang {
    #[value(alias("js"))]
    Javascript,
    #[value(alias("md"))]
    Markdown,
    Php,
    Rust,
    #[value(alias("ts"))]
    Typescript,
}

impl CliLang {
    pub fn to_lang(&self) -> Lang {
        match self {
            CliLang::Javascript => JAVASCRIPT,
            CliLang::Markdown => MARKDOWN,
            CliLang::Php => PHP,
            CliLang::Rust => RUST,
            CliLang::Typescript => TYPESCRIPT,
        }
    }
}

pub const JAVASCRIPT: Lang = Lang {
    file_type: "js",
    language_fn: tree_sitter_javascript::LANGUAGE,
};

pub const MARKDOWN: Lang = Lang {
    file_type: "md",
    language_fn: tree_sitter_md::LANGUAGE,
};

pub const PHP: Lang = Lang {
    file_type: "php",
    language_fn: tree_sitter_php::LANGUAGE_PHP,
};

pub const RUST: Lang = Lang {
    file_type: "rust",
    language_fn: tree_sitter_rust::LANGUAGE,
};

pub const TYPESCRIPT: Lang = Lang {
    file_type: "ts",
    language_fn: tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
};
