use anyhow::Result;

use crate::comment::{
    Comment, CommentShape, Exemption, Placement, Verdict, comment_body, is_directive, is_tagged,
};
use crate::config::Config;
use crate::lang::{LangId, spec_for};
use crate::parse::{code_tokens, parse};

#[derive(Debug, Clone)]
pub struct Block {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_row: usize,
    pub end_row: usize,
    pub end_col: usize,
    pub placement: Placement,
    pub verdict: Verdict,
    pub first_line: String,
}

#[derive(Debug, Default)]
pub struct FileReport {
    pub stripped: usize,
    pub left_in_place: usize,
    pub untagged_found: usize,
    pub unclassified: usize,
    pub new_source: Option<String>,
    pub untagged_lines: Vec<(usize, String)>,
    pub left_lines: Vec<(usize, String)>,
    pub unclassified_lines: Vec<(usize, String)>,
}

pub fn process(
    id: LangId,
    source: &str,
    is_test_target: bool,
    config: &Config,
    apply: bool,
) -> Result<FileReport> {
    let parsed = parse(id, source, is_test_target, config.tags())?;
    let mut report = FileReport::default();

    if parsed.generated {
        return Ok(report);
    }

    let blocks = build_blocks(id, source, &parsed.comments, is_test_target, config);

    for (row, text) in unclassified_rows(id, source, &parsed.unknown_spans, &blocks, config) {
        report.unclassified += 1;
        report.unclassified_lines.push((row, text));
    }

    let mut candidates: Vec<&Block> = Vec::new();
    for block in &blocks {
        match block.verdict {
            Verdict::Untagged => {
                report.untagged_found += 1;
                report
                    .untagged_lines
                    .push((block.start_row, block.first_line.clone()));
                candidates.push(block);
            }
            Verdict::Tagged | Verdict::Exempt(_) => {}
        }
    }

    if candidates.is_empty() {
        return Ok(report);
    }

    if !apply {
        return Ok(report);
    }

    let before = code_tokens(id, source)?;
    let mut ordered = candidates.clone();
    ordered.sort_by_key(|b| std::cmp::Reverse(b.start_byte));

    let mut working = source.to_string();
    for block in ordered {
        let span = deletion_span(&working, block);
        let mut trial = working.clone();
        trial.replace_range(span.clone(), "");
        match code_tokens(id, &trial) {
            Ok(after) if after == before => {
                working = trial;
                report.stripped += 1;
            }
            _ => {
                report.left_in_place += 1;
                report
                    .left_lines
                    .push((block.start_row, block.first_line.clone()));
            }
        }
    }

    let final_tokens = code_tokens(id, &working)?;
    if final_tokens != before {
        anyhow::bail!("projection changed after strip — refusing to write");
    }

    if working != source {
        report.new_source = Some(working);
    }
    Ok(report)
}

fn build_blocks(
    id: LangId,
    source: &str,
    comments: &[Comment],
    is_test_target: bool,
    config: &Config,
) -> Vec<Block> {
    let verdicts: Vec<Verdict> = comments
        .iter()
        .map(|c| verdict_of(id, c, is_test_target, config))
        .collect();

    let mut blocks = Vec::new();
    let mut i = 0;
    while i < comments.len() {
        let first_idx = i;
        let first = &comments[i];
        let mut last = first;
        let is_shebang = first.start_row == 0 && first.text.starts_with("#!");
        // TRIPWIRE: merge only Untagged|Tagged leaders over Untagged continuations — a tagged leader must absorb its own continuation lines so a multi-line note survives whole, but the run must still stop at a tagged line so an untagged leader can never swallow (and delete) a tag below it (the block-swallow bug).
        if !is_shebang
            && first.shape == CommentShape::Line
            && placement_of(source, first) == Placement::WholeLine
            && matches!(verdicts[first_idx], Verdict::Untagged | Verdict::Tagged)
        {
            let mut prev = first;
            let mut j = i + 1;
            while j < comments.len() {
                let next = &comments[j];
                let is_plain_continuation = verdicts[j] == Verdict::Untagged;
                if is_plain_continuation
                    && next.shape == CommentShape::Line
                    && next.start_col == first.start_col
                    && next.start_row == row_after(prev.end_row, prev.end_col)
                    && next.is_doc == first.is_doc
                    && placement_of(source, next) == Placement::WholeLine
                    && no_code_between(source, prev, next)
                {
                    last = next;
                    prev = next;
                    j += 1;
                } else {
                    break;
                }
            }
            i = j;
        } else {
            i += 1;
        }
        blocks.push(make_block(source, first, last, verdicts[first_idx]));
    }
    blocks
}

fn make_block(source: &str, first: &Comment, last: &Comment, verdict: Verdict) -> Block {
    let placement = placement_of(source, first);
    Block {
        start_byte: first.start_byte,
        end_byte: last.end_byte,
        start_row: first.start_row,
        end_row: last.end_row,
        end_col: last.end_col,
        placement,
        verdict,
        first_line: first.text.lines().next().unwrap_or("").trim().to_string(),
    }
}

fn verdict_of(id: LangId, comment: &Comment, is_test_target: bool, config: &Config) -> Verdict {
    // TRIPWIRE: judge an embedded shell comment as bash on its script-relative row — a nix `#` written into a runCommand body follows bash rules, and its shebang is only a shebang at the script's own row 0, not the file's.
    let lang = match comment.embedded_script_row {
        Some(_) => LangId::Bash,
        None => id,
    };
    let effective_row = match comment.embedded_script_row {
        Some(script_row) => comment.start_row.saturating_sub(script_row),
        None => comment.start_row,
    };
    let spec = spec_for(lang);

    if effective_row == 0 && comment.text.starts_with("#!") {
        return Verdict::Exempt(Exemption::Shebang);
    }

    if comment.in_macro {
        return Verdict::Exempt(Exemption::DocSurface);
    }

    let body = comment_body(
        &comment.text,
        spec.line_prefixes,
        spec.block_open,
        spec.block_close,
    );

    if is_directive(body) || config.matches_exempt_pattern(body) {
        return Verdict::Exempt(Exemption::Directive);
    }

    if is_tagged(body, config.tags()) {
        return Verdict::Tagged;
    }

    if !is_test_target && is_module_doc(lang, &comment.text) {
        return Verdict::Exempt(Exemption::ModuleDoc);
    }

    if comment.is_doc && !is_test_target && doc_surface_matches(lang, comment, config) {
        return Verdict::Exempt(Exemption::DocSurface);
    }

    Verdict::Untagged
}

fn unclassified_rows(
    id: LangId,
    source: &str,
    unknown_spans: &[(usize, usize)],
    blocks: &[Block],
    config: &Config,
) -> Vec<(usize, String)> {
    // TRIPWIRE: only report a comment-shaped line where no grammar read it AND no block already covers it AND it carries no tag — a block-covered row was classified, and a tag is the human judgement this gate asks for, so flagging either turns a clean tree red.
    if unknown_spans.is_empty() {
        return Vec::new();
    }
    let spec = spec_for(id);
    let mut covered = std::collections::HashSet::new();
    for b in blocks {
        for r in b.start_row..row_after(b.end_row, b.end_col) {
            covered.insert(r);
        }
    }

    let mut rows = Vec::new();
    let mut offset = 0usize;
    for (row, line) in source.split('\n').enumerate() {
        if let Some(delim_col) = comment_delimiter_col(line, id) {
            let byte = offset + delim_col;
            let text = &line[delim_col..];
            let in_unknown = unknown_spans
                .iter()
                .any(|&(lo, hi)| lo <= byte && byte < hi);
            let body = comment_body(text, spec.line_prefixes, spec.block_open, spec.block_close);
            if in_unknown && !covered.contains(&row) && !is_tagged(body, config.tags()) {
                rows.push((row, text.trim_end().to_string()));
            }
        }
        offset += line.len() + 1;
    }
    rows
}

fn comment_delimiter_col(line: &str, id: LangId) -> Option<usize> {
    let leading = line.len() - line.trim_start().len();
    let rest = &line[leading..];
    let spec = spec_for(id);
    let opens_line = spec.line_prefixes.iter().any(|o| rest.starts_with(o));
    let opens_block = spec.block_open.is_some_and(|o| rest.starts_with(o));
    if opens_line || opens_block {
        Some(leading)
    } else {
        None
    }
}

fn is_module_doc(id: LangId, text: &str) -> bool {
    if !matches!(id, LangId::Rust | LangId::Swift) {
        return false;
    }
    let opener = text.trim_start();
    opener.starts_with("//!") || opener.starts_with("/*!")
}

fn doc_surface_matches(id: LangId, comment: &Comment, config: &Config) -> bool {
    let rules = config.doc_surface_for(id);
    if rules.is_empty() {
        return false;
    }
    rules.iter().any(|rule| {
        rule.item_attributes
            .iter()
            .any(|want| comment.enclosing_attributes.iter().any(|have| have == want))
    })
}

fn placement_of(source: &str, comment: &Comment) -> Placement {
    let bytes = source.as_bytes();
    let line_start = line_start_byte(bytes, comment.start_byte);
    let before = &source[line_start..comment.start_byte];
    let has_code_before = !before.trim().is_empty();

    // TRIPWIRE: treat end_col==0 as ending the line — tree-sitter-rust folds the newline after `///` into the node, so end_byte lands at the next line's start; measuring "code after" from there reads the following line as trailing code and misclassifies every `///` as LeadingInline.
    let has_code_after = if comment.end_col == 0 {
        false
    } else {
        let after_start = comment.end_byte;
        let line_end = line_end_byte(bytes, after_start);
        let after = &source[after_start..line_end];
        !after.trim().is_empty()
    };

    match (has_code_before, has_code_after) {
        (true, _) => Placement::Trailing,
        (false, true) => Placement::LeadingInline,
        (false, false) => Placement::WholeLine,
    }
}

fn row_after(end_row: usize, end_col: usize) -> usize {
    // TRIPWIRE: keep the end_col==0 branch — tree-sitter-rust swallows the newline after `///` but not after `//`, so a `///` block ends one row later than a `//` one; +1 unconditionally would step past the continuation line and break multi-line `///` note merging (and over-cover the code line below in unclassified_rows).
    if end_col == 0 { end_row } else { end_row + 1 }
}

fn no_code_between(source: &str, a: &Comment, b: &Comment) -> bool {
    source[a.end_byte..b.start_byte].trim().is_empty()
}

fn line_start_byte(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i > 0 && bytes[i - 1] != b'\n' {
        i -= 1;
    }
    i
}

fn line_end_byte(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn deletion_span(source: &str, block: &Block) -> std::ops::Range<usize> {
    let bytes = source.as_bytes();
    match block.placement {
        Placement::WholeLine => {
            let start = line_start_byte(bytes, block.start_byte);
            // TRIPWIRE: when end_col==0 the block already ends past its own newline (tree-sitter folds it into a `///` node), so seeking a further line end would swallow the code line below; only extend to the newline when the block stopped mid-line.
            let end = if block.end_col == 0 {
                block.end_byte
            } else {
                let mut e = line_end_byte(bytes, block.end_byte);
                if e < bytes.len() && bytes[e] == b'\n' {
                    e += 1;
                }
                e
            };
            start..end
        }
        Placement::Trailing => {
            let mut start = block.start_byte;
            while start > 0 && (bytes[start - 1] == b' ' || bytes[start - 1] == b'\t') {
                start -= 1;
            }
            start..block.end_byte
        }
        Placement::LeadingInline => {
            let mut end = block.end_byte;
            while end < bytes.len() && (bytes[end] == b' ' || bytes[end] == b'\t') {
                end += 1;
            }
            block.start_byte..end
        }
    }
}
