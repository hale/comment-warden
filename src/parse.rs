use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

use crate::comment::{Comment, CommentShape};
use crate::lang::{LangId, LangSpec, spec_for};

pub struct Parsed {
    pub comments: Vec<Comment>,
    pub unknown_spans: Vec<(usize, usize)>,
    pub generated: bool,
}

pub fn parse(id: LangId, source: &str, is_test_target: bool, _tags: &[String]) -> Result<Parsed> {
    let spec = spec_for(id);
    let tree = parse_tree(&spec, source)?;
    let root = tree.root_node();
    let src = source.as_bytes();

    let generated = detect_generated(&root, src, &spec);

    let mut comments = Vec::new();
    collect_comments(root, src, &spec, false, is_test_target, &mut comments);

    let mut unknown_spans = Vec::new();
    collect_error_spans(root, &mut unknown_spans);

    if id == LangId::Nix {
        let opaque = collect_embedded_nix(&root, src, &mut comments)?;
        unknown_spans.extend(opaque);
    }

    if id == LangId::Erb {
        unknown_spans.extend(collect_opaque_erb(&root));
    }

    comments.sort_by_key(|c| c.start_byte);

    Ok(Parsed {
        comments,
        unknown_spans,
        generated,
    })
}

fn collect_error_spans(node: Node, out: &mut Vec<(usize, usize)>) {
    if !node.has_error() {
        return;
    }
    if node.kind() == "ERROR" {
        out.push((node.start_byte(), node.end_byte()));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_error_spans(child, out);
    }
}

fn parse_tree(spec: &LangSpec, source: &str) -> Result<tree_sitter::Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&spec.grammar)
        .context("setting tree-sitter language")?;
    parser
        .parse(source, None)
        .context("tree-sitter returned no tree")
}

fn is_comment_kind(spec: &LangSpec, kind: &str) -> bool {
    spec.comment_kinds.contains(&kind)
}

const GENERATED_DOC_PREFIXES: [&str; 4] = ["///", "//!", "/**", "/*!"];

fn detect_generated(root: &Node, src: &[u8], spec: &LangSpec) -> bool {
    let mut comments: Vec<Node> = Vec::new();
    let mut stack = vec![*root];
    while let Some(n) = stack.pop() {
        if node_holds_data(n.kind()) {
            continue;
        }
        if is_comment_kind(spec, n.kind()) {
            comments.push(n);
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            stack.push(ch);
        }
    }
    comments.sort_by_key(|n| n.start_byte());

    let Some(&first) = comments.first() else {
        return false;
    };
    let first_is_shebang =
        first.start_position().row == 0 && node_first_line(first, src).starts_with("#!");

    let candidate = if first_is_shebang {
        match comments.get(1) {
            Some(&second) if second.start_position().row == 1 => second,
            _ => return false,
        }
    } else if first.start_position().row == 0 {
        first
    } else {
        return false;
    };

    if !node_owns_line(candidate, src) {
        return false;
    }

    let first_line = node_first_line(candidate, src);
    if GENERATED_DOC_PREFIXES
        .iter()
        .any(|p| first_line.trim_start().starts_with(p))
    {
        return false;
    }

    let body = crate::comment::comment_body(
        first_line,
        spec.line_prefixes,
        spec.block_open,
        spec.block_close,
    );
    body.starts_with("@generated")
}

fn node_first_line<'a>(node: Node, src: &'a [u8]) -> &'a str {
    let text = std::str::from_utf8(&src[node.byte_range()]).unwrap_or("");
    text.lines().next().unwrap_or("")
}

fn node_owns_line(node: Node, src: &[u8]) -> bool {
    let start = node.start_byte();
    let line_start = src[..start]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1);
    src[line_start..start]
        .iter()
        .all(|&b| b == b' ' || b == b'\t')
}

fn node_holds_data(kind: &str) -> bool {
    // TRIPWIRE: an html attribute value is data, and the grammar parses `<!--` inside one as a real comment node — without this the gate rewrites `class="<!-- x -->"` to `class=""` and the projection cannot see it, because a comment leaf is excluded from the projection.
    matches!(kind, "quoted_attribute_value")
}

fn node_is_macro(kind: &str) -> bool {
    matches!(
        kind,
        "macro_invocation" | "macro_definition" | "token_tree" | "macro_rules"
    )
}

fn is_doc(text: &str, spec: &LangSpec) -> bool {
    let t = text.trim_start();
    spec.doc_prefixes.line.iter().any(|p| t.starts_with(p))
        || spec.doc_prefixes.block.iter().any(|p| t.starts_with(p))
}

fn collect_comments(
    node: Node,
    src: &[u8],
    spec: &LangSpec,
    in_macro: bool,
    is_test_target: bool,
    out: &mut Vec<Comment>,
) {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();

    for (i, child) in children.iter().enumerate() {
        if node_holds_data(child.kind()) {
            continue;
        }
        if is_comment_kind(spec, child.kind()) {
            let text = std::str::from_utf8(&src[child.byte_range()])
                .unwrap_or("")
                .to_string();
            let is_block = child.kind().contains("block")
                || child.kind() == "multiline_comment"
                || spec
                    .block_open
                    .is_some_and(|open| text.trim_start().starts_with(open));
            let shape = if is_block {
                CommentShape::Block
            } else {
                CommentShape::Line
            };
            let doc = is_doc(&text, spec);
            let enclosing = if is_test_target {
                Vec::new()
            } else {
                let mut attrs = attributes_for_following_item(&children, i, src);
                attrs.extend(ancestor_item_attributes(*child, src));
                attrs
            };
            out.push(Comment {
                start_byte: child.start_byte(),
                end_byte: child.end_byte(),
                start_row: child.start_position().row,
                start_col: child.start_position().column,
                end_row: child.end_position().row,
                end_col: child.end_position().column,
                shape,
                is_doc: doc,
                text,
                in_macro,
                enclosing_attributes: enclosing,
                embedded_script_row: None,
            });
            continue;
        }
        let child_in_macro = in_macro || node_is_macro(child.kind());
        collect_comments(*child, src, spec, child_in_macro, is_test_target, out);
    }
}

fn attributes_for_following_item(siblings: &[Node], comment_idx: usize, src: &[u8]) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut j = comment_idx + 1;
    while j < siblings.len() {
        let k = siblings[j].kind();
        if k == "attribute_item" || k == "inner_attribute_item" {
            attrs.extend(attribute_paths(siblings[j], src));
            j += 1;
            continue;
        }
        if k.contains("comment") {
            j += 1;
            continue;
        }
        attrs.extend(item_own_attributes(siblings[j], src));
        break;
    }
    attrs
}

fn ancestor_item_attributes(node: Node, src: &[u8]) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut current = node;
    while let Some(parent) = current.parent() {
        attrs.extend(item_own_attributes(parent, src));
        current = parent;
    }
    attrs
}

fn item_own_attributes(item: Node, src: &[u8]) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut cursor = item.walk();
    for ch in item.children(&mut cursor) {
        if ch.kind() == "attribute_item" || ch.kind() == "inner_attribute_item" {
            attrs.extend(attribute_paths(ch, src));
        }
    }
    attrs
}

fn attribute_paths(attr_item: Node, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let text = std::str::from_utf8(&src[attr_item.byte_range()]).unwrap_or("");
    let inner = text
        .trim_start_matches("#")
        .trim_start_matches("!")
        .trim_start_matches('[')
        .trim_end_matches(']');
    for part in inner.split([',', '(']) {
        let name = part.trim().trim_end_matches(')').trim();
        if name.is_empty() {
            continue;
        }
        let head = name.split(['(', '=', ' ']).next().unwrap_or("").trim();
        if head.is_empty() {
            continue;
        }
        out.push(head.to_string());
        if let Some(last) = head.rsplit("::").next()
            && last != head
        {
            out.push(last.to_string());
        }
    }
    out
}

const NIX_SCRIPT_ATTRS: &[&str] = &[
    "buildCommand",
    "buildPhase",
    "checkPhase",
    "extraCommands",
    "installPhase",
    "installPhaseCommand",
    "postFixup",
    "postInstall",
    "postPatch",
    "preStart",
    "script",
    "shellHook",
];

const NIX_SCRIPT_FNS: &[&str] = &[
    "runCommand",
    "runCommandLocal",
    "writeScript",
    "writeScriptBin",
    "writeShellApplication",
    "writeShellScript",
    "writeShellScriptBin",
];

const PYTHON_INTERPRETERS: &[&str] = &["python", "python3"];

fn line_starts(src: &[u8]) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, &b) in src.iter().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn collect_embedded_nix(
    root: &Node,
    src: &[u8],
    out: &mut Vec<Comment>,
) -> Result<Vec<(usize, usize)>> {
    let starts = line_starts(src);
    let mut opaque = Vec::new();

    let mut stack = vec![*root];
    while let Some(node) = stack.pop() {
        let mut cursor = node.walk();
        for ch in node.children(&mut cursor) {
            stack.push(ch);
        }
        if node.kind() != "indented_string_expression" {
            continue;
        }
        let fragments: Vec<Node> = {
            let mut c = node.walk();
            node.children(&mut c)
                .filter(|n| n.kind() == "string_fragment")
                .collect()
        };
        if fragments.is_empty() {
            continue;
        }
        if !nix_string_is_script(node, src) {
            for f in &fragments {
                opaque.push((f.start_byte(), f.end_byte()));
            }
            continue;
        }
        embed_shell_comments(src, &starts, &fragments, out)?;
    }

    Ok(opaque)
}

fn collect_opaque_erb(root: &Node) -> Vec<(usize, usize)> {
    // TRIPWIRE: report an erb `code` region as opaque rather than ignoring it — the ruby inside `<% %>` is never parsed here, and a fragment like ` if x ` is not standalone ruby, so a `#` line in one is unread, not comment-free.
    let mut spans = Vec::new();
    let mut stack = vec![*root];
    while let Some(node) = stack.pop() {
        if node.kind() == "code" {
            spans.push((node.start_byte(), node.end_byte()));
            continue;
        }
        let mut cursor = node.walk();
        for ch in node.children(&mut cursor) {
            stack.push(ch);
        }
    }
    spans
}

fn embed_shell_comments(
    src: &[u8],
    starts: &[usize],
    fragments: &[Node],
    out: &mut Vec<Comment>,
) -> Result<()> {
    // TRIPWIRE: rebuild the script over a blanked, indent-stripped copy — a nix ''…'' is one token, its antiquotations must not read as bash, and an indented heredoc terminator does not close its heredoc, so a body left as written makes the grammar swallow the rest as heredoc content and report no comments.
    let start = fragments[0].start_byte();
    let end = fragments[fragments.len() - 1].end_byte();
    let mut body: Vec<u8> = src[start..end]
        .iter()
        .map(|&b| if b == b'\n' { b'\n' } else { b' ' })
        .collect();
    for f in fragments {
        let fs = f.start_byte() - start;
        let fe = f.end_byte() - start;
        body[fs..fe].copy_from_slice(&src[f.start_byte()..f.end_byte()]);
    }

    let body_lines: Vec<&[u8]> = split_lines(&body);
    let indent = body_lines
        .iter()
        .filter(|l| !l.iter().all(|&b| b == b' ' || b == b'\t'))
        .map(|l| l.len() - l.iter().skip_while(|&&b| b == b' ' || b == b'\t').count())
        .min()
        .unwrap_or(0);

    let body_row = count_newlines(&src[..start]);
    let script_row = body_row + if body.first() == Some(&b'\n') { 1 } else { 0 };

    let mut dedented = Vec::new();
    for (i, line) in body_lines.iter().enumerate() {
        if i > 0 {
            dedented.push(b'\n');
        }
        let take_from = indent.min(line.len());
        dedented.extend_from_slice(&line[take_from..]);
    }

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .context("setting bash language for embedded nix")?;
    let tree = parser
        .parse(&dedented, None)
        .context("tree-sitter returned no tree for embedded shell")?;

    let mut positions: Vec<(usize, usize)> = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if n.kind() == "comment" {
            positions.push((n.start_position().row, n.start_position().column));
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            stack.push(ch);
        }
    }
    positions.extend(heredoc_script_comments(&tree, &dedented));

    for (comment_row, comment_column) in positions {
        let row = body_row + comment_row;
        if row >= starts.len() {
            continue;
        }
        let begin = starts[row] + comment_column + indent;
        let line_end = line_end_byte(src, starts[row]).min(end);
        if begin >= line_end {
            continue;
        }
        let text = std::str::from_utf8(&src[begin..line_end])
            .unwrap_or("")
            .to_string();
        out.push(Comment {
            start_byte: begin,
            end_byte: line_end,
            start_row: row,
            start_col: begin - starts[row],
            end_row: row,
            end_col: line_end - starts[row],
            shape: CommentShape::Line,
            is_doc: false,
            text,
            in_macro: false,
            enclosing_attributes: Vec::new(),
            embedded_script_row: Some(script_row),
        });
    }
    Ok(())
}

fn heredoc_script_comments(tree: &tree_sitter::Tree, buf: &[u8]) -> Vec<(usize, usize)> {
    // TRIPWIRE: a heredoc is one token to bash; only reparse its body as python when the consuming command is a python interpreter — text fed to `cat` is a file, where a `#` is content, not a comment.
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        let mut c = node.walk();
        for ch in node.children(&mut c) {
            stack.push(ch);
        }
        if node.kind() != "heredoc_body" {
            continue;
        }
        let mut statement = node.parent();
        while let Some(s) = statement {
            if s.kind() == "redirected_statement" {
                break;
            }
            statement = s.parent();
        }
        let Some(statement) = statement else {
            continue;
        };
        let mut sc = statement.walk();
        let Some(first) = statement.children(&mut sc).next() else {
            continue;
        };
        let cmd_text = std::str::from_utf8(&buf[first.byte_range()]).unwrap_or("");
        let interp = cmd_text
            .split_whitespace()
            .next()
            .unwrap_or("")
            .rsplit('/')
            .next()
            .unwrap_or("");
        if !PYTHON_INTERPRETERS.contains(&interp) {
            continue;
        }
        let body = &buf[node.byte_range()];
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .is_err()
        {
            continue;
        }
        let Some(py_tree) = parser.parse(body, None) else {
            continue;
        };
        let base_row = node.start_position().row;
        let mut pstack = vec![py_tree.root_node()];
        while let Some(n) = pstack.pop() {
            if n.kind() == "comment" {
                out.push((base_row + n.start_position().row, n.start_position().column));
            }
            let mut cc = n.walk();
            for ch in n.children(&mut cc) {
                pstack.push(ch);
            }
        }
    }
    out
}

fn split_lines(buf: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, &b) in buf.iter().enumerate() {
        if b == b'\n' {
            lines.push(&buf[start..i]);
            start = i + 1;
        }
    }
    lines.push(&buf[start..]);
    lines
}

fn count_newlines(buf: &[u8]) -> usize {
    buf.iter().filter(|&&b| b == b'\n').count()
}

fn line_end_byte(src: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < src.len() && src[i] != b'\n' {
        i += 1;
    }
    i
}

fn nix_string_is_script(string_node: Node, src: &[u8]) -> bool {
    // TRIPWIRE: a non-script binding must keep climbing, not return false — a `lib.optionalString` body bound to `postFixup` is still a postFixup body, so only a positive match ends the search; reaching the root with no evidence is not shell.
    let mut current = string_node;
    while let Some(parent) = current.parent() {
        match parent.kind() {
            "binding" => {
                if let Some(name) = binding_attr_name(parent, src)
                    && NIX_SCRIPT_ATTRS.contains(&name.as_str())
                {
                    return true;
                }
            }
            "apply_expression" => {
                if let Some(fname) = apply_head_name(parent, src)
                    && NIX_SCRIPT_FNS.contains(&fname.as_str())
                {
                    return true;
                }
            }
            _ => {}
        }
        current = parent;
    }
    false
}

fn binding_attr_name(binding: Node, src: &[u8]) -> Option<String> {
    let attrpath = binding.child_by_field_name("attrpath")?;
    let text = std::str::from_utf8(&src[attrpath.byte_range()]).ok()?;
    text.rsplit('.').next().map(|s| s.trim().to_string())
}

fn apply_head_name(apply: Node, src: &[u8]) -> Option<String> {
    // TRIPWIRE: writeShellScript "name" ''body'' nests as apply(apply(fn,name),body), so reading the immediate function misses the body — descend to the head to recognise the common two-arg form.
    let mut func = apply.child_by_field_name("function")?;
    while func.kind() == "apply_expression" {
        func = func.child_by_field_name("function")?;
    }
    let text = std::str::from_utf8(&src[func.byte_range()]).ok()?;
    text.rsplit('.').next().map(|s| s.trim().to_string())
}

pub fn code_tokens(id: LangId, source: &str) -> Result<Vec<Vec<u8>>> {
    let spec = spec_for(id);
    let tree = parse_tree(&spec, source)?;
    let src = source.as_bytes();
    let mut ordered = Vec::new();
    collect_leaves(tree.root_node(), &spec, &mut ordered);
    Ok(ordered
        .into_iter()
        .map(|leaf| src[leaf.byte_range()].to_vec())
        .collect())
}

fn collect_leaves<'a>(node: Node<'a>, spec: &LangSpec, out: &mut Vec<Node<'a>>) {
    if node_holds_data(node.kind()) {
        out.push(node);
        return;
    }
    if is_comment_kind(spec, node.kind()) {
        return;
    }
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();
    if children.is_empty() {
        if !node.byte_range().is_empty() {
            out.push(node);
        }
        return;
    }
    for child in children {
        collect_leaves(child, spec, out);
    }
}
