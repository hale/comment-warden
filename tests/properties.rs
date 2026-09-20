use comment_warden::config::Config;
use comment_warden::lang::LangId;
use comment_warden::parse::code_tokens;
use comment_warden::strip::{FileReport, process};
use proptest::prelude::*;

struct Template {
    id: LangId,
    lines: &'static [&'static str],
    line_prefix: &'static str,
}

const TEMPLATES: &[Template] = &[
    Template {
        id: LangId::Rust,
        lines: &[
            "fn main() {",
            "    let x = 1;",
            "    let s = \"COMMENT_LOOKALIKE\";",
            "    let _ = x;",
            "}",
        ],
        line_prefix: "//",
    },
    Template {
        id: LangId::Python,
        lines: &[
            "def f():",
            "    x = 1",
            "    s = \"COMMENT_LOOKALIKE\"",
            "    return x",
        ],
        line_prefix: "#",
    },
    Template {
        id: LangId::Go,
        lines: &[
            "package main",
            "func f() int {",
            "\tx := 1",
            "\ts := \"COMMENT_LOOKALIKE\"",
            "\t_ = s",
            "\treturn x",
            "}",
        ],
        line_prefix: "//",
    },
];

#[derive(Debug, Clone)]
enum Insert {
    Untagged,
    Tagged,
    InString,
    None,
}

fn insert_strategy() -> impl Strategy<Value = Insert> {
    prop_oneof![
        Just(Insert::Untagged),
        Just(Insert::Tagged),
        Just(Insert::InString),
        Just(Insert::None),
    ]
}

const TAGGED_COMMENT_BODY: &str = "TRIPWIRE: this edit breaks something real";

fn build_source(template: &Template, inserts: &[Insert]) -> String {
    let p = template.line_prefix;
    let mut out = String::new();
    for (line, insert) in template.lines.iter().zip(inserts.iter().cycle()) {
        match insert {
            Insert::Untagged => {
                out.push_str(p);
                out.push_str(" ordinary prose here\n");
                out.push_str(line);
                out.push('\n');
            }
            Insert::Tagged => {
                out.push_str(p);
                out.push(' ');
                out.push_str(TAGGED_COMMENT_BODY);
                out.push('\n');
                out.push_str(line);
                out.push('\n');
            }
            Insert::InString => {
                let injected = line.replace(
                    "COMMENT_LOOKALIKE",
                    "// TRIPWIRE: this looks like a comment but is inside a string",
                );
                out.push_str(&injected);
                out.push('\n');
            }
            Insert::None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

fn strip(id: LangId, src: &str) -> FileReport {
    process(id, src, false, &Config::empty(), true).unwrap()
}

fn result_source(id: LangId, src: &str) -> String {
    strip(id, src).new_source.unwrap_or_else(|| src.to_string())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn projection_is_preserved(
        t in 0usize..TEMPLATES.len(),
        inserts in prop::collection::vec(insert_strategy(), 1..6),
    ) {
        let template = &TEMPLATES[t];
        let src = build_source(template, &inserts);
        let out = result_source(template.id, &src);
        prop_assert_eq!(
            code_tokens(template.id, &out).unwrap(),
            code_tokens(template.id, &src).unwrap()
        );
    }

    #[test]
    fn strip_is_idempotent(
        t in 0usize..TEMPLATES.len(),
        inserts in prop::collection::vec(insert_strategy(), 1..6),
    ) {
        let template = &TEMPLATES[t];
        let src = build_source(template, &inserts);
        let once = result_source(template.id, &src);
        let twice = result_source(template.id, &once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn check_is_clean_after_strip_except_left_and_unclassified(
        t in 0usize..TEMPLATES.len(),
        inserts in prop::collection::vec(insert_strategy(), 1..6),
    ) {
        let template = &TEMPLATES[t];
        let src = build_source(template, &inserts);
        let stripped = result_source(template.id, &src);
        let recheck = process(template.id, &stripped, false, &Config::empty(), false).unwrap();
        let remaining = recheck.untagged_found - recheck.left_in_place;
        prop_assert_eq!(remaining, 0);
    }

    #[test]
    fn tagged_comments_never_removed(
        t in 0usize..TEMPLATES.len(),
        inserts in prop::collection::vec(insert_strategy(), 1..6),
    ) {
        let template = &TEMPLATES[t];
        let src = build_source(template, &inserts);
        let out = result_source(template.id, &src);
        let tagged_before = src.matches(TAGGED_COMMENT_BODY).count();
        let tagged_after = out.matches(TAGGED_COMMENT_BODY).count();
        prop_assert_eq!(tagged_before, tagged_after);
    }

    #[test]
    fn keep_adjacent_to_untagged_survives(
        t in 0usize..TEMPLATES.len(),
        inserts in prop::collection::vec(insert_strategy(), 1..6),
    ) {
        let template = &TEMPLATES[t];
        let p = template.line_prefix;
        let planted_untagged = "GATE_PROBE untagged sentinel";
        let body = build_source(template, &inserts);
        let src = format!(
            "{p} {planted_untagged}\n{p} {TAGGED_COMMENT_BODY}\n{body}"
        );
        let out = result_source(template.id, &src);
        prop_assert!(out.contains(TAGGED_COMMENT_BODY));
        prop_assert!(!out.contains(planted_untagged));
    }
}
