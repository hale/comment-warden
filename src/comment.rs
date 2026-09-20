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
) -> &'a str {
    let t = text.trim_start();
    if let Some(open) = block_open
        && let Some(rest) = t.strip_prefix(open)
    {
        let rest = rest.trim_start_matches(['*', '!']);
        let rest = rest.trim_end_matches(['/', '*']);
        return rest.trim();
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
        | "@ts-nocheck" => true,
        _ => {
            body.starts_with("swift-tools-version:")
                || body.starts_with("clippy::")
                || first == "clippy"
                || (body.starts_with("-*-") && body.contains("coding:"))
        }
    }
}
