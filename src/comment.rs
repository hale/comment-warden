#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    WholeLine,
    LeadingInline,
    Trailing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentShape {
    Line,
    Block,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
    pub shape: CommentShape,
    pub is_doc: bool,
    pub text: String,
    pub in_macro: bool,
    pub enclosing_attributes: Vec<String>,
    pub embedded_script_row: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exemption {
    Shebang,
    GeneratedFile,
    Directive,
    DocSurface,
    ModuleDoc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Exempt(Exemption),
    Tagged,
    Untagged,
}

pub fn comment_body<'a>(
    text: &'a str,
    line_prefixes: &[&str],
    block_open: Option<&str>,
    block_close: Option<&str>,
) -> &'a str {
    let t = text.trim_start();
    if let Some(open) = block_open
        && let Some(rest) = t.strip_prefix(open)
    {
        let rest = rest.trim_start_matches(['*', '!']).trim_end();
        let rest = block_close
            .and_then(|close| rest.strip_suffix(close))
            .unwrap_or(rest);
        return rest.trim_end_matches('*').trim();
    }
    let mut best: Option<&str> = None;
    for p in line_prefixes {
        if let Some(rest) = t.strip_prefix(p) {
            let rest = rest.trim_start_matches(['/', '!']);
            match best {
                Some(b) if b.len() <= rest.len() => {}
                _ => best = Some(rest),
            }
        }
    }
    best.unwrap_or(t).trim()
}

pub fn is_tagged(body: &str, tags: &[String]) -> bool {
    tags.iter().any(|tag| {
        body.strip_prefix(tag.as_str())
            .and_then(|rest| rest.strip_prefix(':'))
            .is_some_and(|reason| !reason.trim().is_empty())
    })
}

const WEBPACK_MAGIC_KEYS: &[&str] = &[
    "webpackChunkName:",
    "webpackPrefetch:",
    "webpackPreload:",
    "webpackIgnore:",
    "webpackMode:",
    "webpackExports:",
    "webpackInclude:",
    "webpackExclude:",
];

pub fn is_directive(body: &str) -> bool {
    let first = body.split_whitespace().next().unwrap_or("");
    match first {
        "shellcheck"
        | "swiftlint:disable"
        | "swiftlint:enable"
        | "eslint-disable"
        | "eslint-enable"
        | "eslint-disable-line"
        | "eslint-disable-next-line"
        | "eslint"
        | "prettier-ignore"
        | "@ts-ignore"
        | "@ts-expect-error"
        | "@ts-nocheck"
        | "rubocop:disable"
        | "rubocop:enable"
        | "stylelint-disable"
        | "stylelint-enable"
        | "stylelint-disable-line"
        | "stylelint-disable-next-line" => true,
        _ => {
            body.starts_with("swift-tools-version:")
                || body.starts_with("clippy::")
                || body.starts_with("frozen_string_literal:")
                || first == "clippy"
                || (body.starts_with("-*-") && body.contains("coding:"))
                || WEBPACK_MAGIC_KEYS.iter().any(|key| body.starts_with(key))
                || is_eslint_globals(body)
        }
    }
}

fn is_eslint_globals(body: &str) -> bool {
    // TRIPWIRE: keep the identifier-list check — a bare first word `global` would exempt prose like `// global config lives here` from ever needing a tag.
    let Some(rest) = body
        .strip_prefix("global ")
        .or_else(|| body.strip_prefix("globals "))
    else {
        return false;
    };
    let mut named = false;
    for entry in rest.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let name = entry.split(':').next().unwrap_or("").trim();
        let is_identifier = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '$');
        if !is_identifier {
            return false;
        }
        named = true;
    }
    named
}
