use tree_sitter::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LangId {
    Rust,
    Swift,
    Nix,
    Bash,
    Hcl,
    Python,
    TypeScript,
    JavaScript,
    Go,
    C,
    Cpp,
    Yaml,
    Toml,
    Css,
}

impl LangId {
    pub fn from_config_name(name: &str) -> Option<Self> {
        let id = match name {
            "rust" => Self::Rust,
            "swift" => Self::Swift,
            "nix" => Self::Nix,
            "bash" | "sh" | "shell" => Self::Bash,
            "hcl" | "terraform" => Self::Hcl,
            "python" => Self::Python,
            "typescript" | "ts" => Self::TypeScript,
            "javascript" | "js" => Self::JavaScript,
            "go" => Self::Go,
            "c" => Self::C,
            "cpp" | "c++" => Self::Cpp,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "css" => Self::Css,
            _ => return None,
        };
        Some(id)
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        let id = match ext {
            "rs" => Self::Rust,
            "swift" => Self::Swift,
            "nix" => Self::Nix,
            "sh" | "bash" => Self::Bash,
            "tf" | "hcl" | "tfvars" => Self::Hcl,
            "py" | "pyi" => Self::Python,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "go" => Self::Go,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" => Self::Cpp,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "css" => Self::Css,
            _ => return None,
        };
        Some(id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DocPrefixes {
    pub line: &'static [&'static str],
    pub block: &'static [&'static str],
}

pub struct LangSpec {
    pub grammar: Language,
    pub comment_kinds: &'static [&'static str],
    pub doc_prefixes: DocPrefixes,
    pub line_prefixes: &'static [&'static str],
    pub block_open: Option<&'static str>,
}

const NO_DOC: DocPrefixes = DocPrefixes {
    line: &[],
    block: &[],
};

pub fn spec_for(id: LangId) -> LangSpec {
    match id {
        LangId::Rust => LangSpec {
            grammar: Language::new(tree_sitter_rust::LANGUAGE),
            comment_kinds: &["line_comment", "block_comment"],
            doc_prefixes: DocPrefixes {
                line: &["///", "//!"],
                block: &["/**", "/*!"],
            },
            line_prefixes: &["//"],
            block_open: Some("/*"),
        },
        LangId::Swift => LangSpec {
            grammar: tree_sitter_swift::LANGUAGE.into(),
            comment_kinds: &["comment", "multiline_comment"],
            doc_prefixes: DocPrefixes {
                line: &["///", "//!"],
                block: &["/**", "/*!"],
            },
            line_prefixes: &["//"],
            block_open: Some("/*"),
        },
        LangId::Nix => LangSpec {
            grammar: tree_sitter_nix::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: NO_DOC,
            line_prefixes: &["#"],
            block_open: Some("/*"),
        },
        LangId::Bash => LangSpec {
            grammar: tree_sitter_bash::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: NO_DOC,
            line_prefixes: &["#"],
            block_open: None,
        },
        LangId::Hcl => LangSpec {
            grammar: tree_sitter_hcl::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: NO_DOC,
            line_prefixes: &["#", "//"],
            block_open: Some("/*"),
        },
        LangId::Python => LangSpec {
            grammar: tree_sitter_python::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: NO_DOC,
            line_prefixes: &["#"],
            block_open: None,
        },
        LangId::TypeScript => LangSpec {
            grammar: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            comment_kinds: &["comment"],
            doc_prefixes: DocPrefixes {
                line: &[],
                block: &["/**"],
            },
            line_prefixes: &["//"],
            block_open: Some("/*"),
        },
        LangId::JavaScript => LangSpec {
            grammar: tree_sitter_javascript::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: DocPrefixes {
                line: &[],
                block: &["/**"],
            },
            line_prefixes: &["//"],
            block_open: Some("/*"),
        },
        LangId::Go => LangSpec {
            grammar: tree_sitter_go::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: NO_DOC,
            line_prefixes: &["//"],
            block_open: Some("/*"),
        },
        LangId::C => LangSpec {
            grammar: tree_sitter_c::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: DocPrefixes {
                line: &[],
                block: &["/**"],
            },
            line_prefixes: &["//"],
            block_open: Some("/*"),
        },
        LangId::Cpp => LangSpec {
            grammar: tree_sitter_cpp::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: DocPrefixes {
                line: &["///"],
                block: &["/**"],
            },
            line_prefixes: &["//"],
            block_open: Some("/*"),
        },
        LangId::Yaml => LangSpec {
            grammar: tree_sitter_yaml::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: NO_DOC,
            line_prefixes: &["#"],
            block_open: None,
        },
        LangId::Toml => LangSpec {
            grammar: tree_sitter_toml_ng::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: NO_DOC,
            line_prefixes: &["#"],
            block_open: None,
        },
        LangId::Css => LangSpec {
            grammar: tree_sitter_css::LANGUAGE.into(),
            comment_kinds: &["comment"],
            doc_prefixes: NO_DOC,
            line_prefixes: &[],
            block_open: Some("/*"),
        },
    }
}
