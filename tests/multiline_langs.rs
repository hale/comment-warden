use comment_warden::comment::comment_body;
use comment_warden::config::Config;
use comment_warden::lang::{LangId, spec_for};
use comment_warden::parse::parse;
use comment_warden::strip::{FileReport, process};

fn strip(id: LangId, src: &str) -> FileReport {
    process(id, src, false, &Config::empty(), true).unwrap()
}

fn stripped_src(id: LangId, src: &str) -> String {
    strip(id, src).new_source.unwrap_or_else(|| src.to_string())
}

fn config_with_rust_surface(attrs: &[&str]) -> Config {
    let toml = format!(
        "[[doc_surface]]\nlang = \"rust\"\nitem_attributes = [{}]\n",
        attrs
            .iter()
            .map(|a| format!("\"{a}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let dir = tempdir();
    let path = dir.join("comment-warden.toml");
    std::fs::write(&path, toml).unwrap();
    Config::load(Some(&path)).unwrap()
}

fn tempdir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!(
        "comment-warden-mllang-{}-{}-{:?}",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&base).unwrap();
    base
}

#[test]
fn rust_multiline_tripwire_note_survives_whole() {
    let src = "// TRIPWIRE: deleting this orphans the record\n// the record is keyed by record id\n// and nothing else references it\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Rust, src);
    assert!(out.contains("// TRIPWIRE: deleting this orphans the record"));
    assert!(out.contains("// the record is keyed by record id"));
    assert!(out.contains("// and nothing else references it"));
}

#[test]
fn swift_multiline_context_note_survives_whole() {
    let src = "// CONTEXT: upstream returns 200 on failure\n// so we inspect the body instead of the status\nlet x = 1\n";
    let report = strip(LangId::Swift, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Swift, src);
    assert!(out.contains("// CONTEXT: upstream returns 200 on failure"));
    assert!(out.contains("// so we inspect the body instead of the status"));
}

#[test]
fn typescript_multiline_tripwire_note_survives_whole() {
    let src = "// TRIPWIRE: this cast is load-bearing for the codegen\n// removing it silently drops the field\nvar x = 1;\n";
    let report = strip(LangId::TypeScript, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::TypeScript, src);
    assert!(out.contains("// TRIPWIRE: this cast is load-bearing for the codegen"));
    assert!(out.contains("// removing it silently drops the field"));
}

#[test]
fn javascript_multiline_tripwire_note_survives_whole() {
    let src = "// TRIPWIRE: order matters, init runs before the handler binds\n// swapping these two lines drops the first event\nvar x = 1;\n";
    let report = strip(LangId::JavaScript, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::JavaScript, src);
    assert!(out.contains("// TRIPWIRE: order matters, init runs before the handler binds"));
    assert!(out.contains("// swapping these two lines drops the first event"));
}

#[test]
fn nix_multiline_tripwire_note_survives_whole() {
    let src = "# TRIPWIRE: this override pins the transitive dep\n# dropping it pulls in the broken version\n{ pkgs }: pkgs.hello\n";
    let report = strip(LangId::Nix, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Nix, src);
    assert!(out.contains("# TRIPWIRE: this override pins the transitive dep"));
    assert!(out.contains("# dropping it pulls in the broken version"));
}

#[test]
fn nix_multiline_context_note_survives_whole() {
    let src = "# CONTEXT: the daemon only reads this port on first boot\n# so a later change here has no effect until a full restart\n{ pkgs }: pkgs.hello\n";
    let report = strip(LangId::Nix, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Nix, src);
    assert!(out.contains("# CONTEXT: the daemon only reads this port on first boot"));
    assert!(out.contains("# so a later change here has no effect until a full restart"));
}

#[test]
fn ruby_multiline_tripwire_note_survives_whole() {
    let src = "# TRIPWIRE: this retry count matches the upstream timeout\n# lowering it makes the call fail before the server answers\nx = 1\n";
    let report = strip(LangId::Ruby, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Ruby, src);
    assert!(out.contains("# TRIPWIRE: this retry count matches the upstream timeout"));
    assert!(out.contains("# lowering it makes the call fail before the server answers"));
}

#[test]
fn scss_multiline_tripwire_note_survives_whole() {
    let src = "// TRIPWIRE: this z-index sits above the sticky header\n// lowering it hides the dropdown behind it\n.a { color: red; }\n";
    let report = strip(LangId::Scss, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Scss, src);
    assert!(out.contains("// TRIPWIRE: this z-index sits above the sticky header"));
    assert!(out.contains("// lowering it hides the dropdown behind it"));
}

fn block_bodies(id: LangId, src: &str) -> Vec<String> {
    let spec = spec_for(id);
    parse(id, src, false, Config::empty().tags())
        .unwrap()
        .comments
        .iter()
        .filter(|c| c.shape == comment_warden::comment::CommentShape::Block)
        .map(|c| {
            comment_body(
                &c.text,
                spec.line_prefixes,
                spec.block_open,
                spec.block_close,
            )
            .to_string()
        })
        .collect()
}

#[test]
fn ruby_begin_end_block_body_excludes_the_closer() {
    let src = "=begin\nTRIPWIRE: the closer is not part of the body\n=end\nx = 1\n";
    assert_eq!(
        block_bodies(LangId::Ruby, src),
        vec!["TRIPWIRE: the closer is not part of the body".to_string()]
    );
}

#[test]
fn ruby_begin_end_untagged_block_leaves_no_closer_fragment() {
    let src = "=begin\nuntagged block prose\n=end\nx = 1\n";
    let report = strip(LangId::Ruby, src);
    assert_eq!(report.stripped, 1);
    let out = stripped_src(LangId::Ruby, src);
    assert_eq!(out, "x = 1\n");
    assert!(!out.contains("=end"));
    assert!(!out.contains("=begin"));
    for (_, line) in report
        .untagged_lines
        .iter()
        .chain(report.unclassified_lines.iter())
    {
        assert!(
            !line.contains("=end"),
            "closer leaked into a report line: {line}"
        );
    }
}

#[test]
fn c_style_block_body_excludes_the_closer_and_stray_stars() {
    let plain = "/* a plain block body */\nfn main() {}\n";
    assert_eq!(
        block_bodies(LangId::Rust, plain),
        vec!["a plain block body".to_string()]
    );
    let starred = "/*** a starred block body ***/\nfn main() {}\n";
    assert_eq!(
        block_bodies(LangId::Rust, starred),
        vec!["a starred block body".to_string()]
    );
    let banged = "/* a body ending in a bang! */\nfn main() {}\n";
    assert_eq!(
        block_bodies(LangId::Rust, banged),
        vec!["a body ending in a bang!".to_string()]
    );
}

#[test]
fn ruby_begin_end_block_is_one_comment_not_a_line_run_leader() {
    let src = "=begin\nuntagged block prose\n=end\n# TRIPWIRE: an independent tagged line below the block\nx = 1\n";
    let report = strip(LangId::Ruby, src);
    assert_eq!(report.stripped, 1);
    let out = stripped_src(LangId::Ruby, src);
    assert!(!out.contains("untagged block prose"));
    assert!(out.contains("# TRIPWIRE: an independent tagged line below the block"));
}

#[test]
fn scss_block_and_line_comment_forms_are_judged_independently() {
    let src =
        "/* TRIPWIRE: the block form is tagged */\n// untagged line prose\n.a { color: red; }\n";
    let report = strip(LangId::Scss, src);
    assert_eq!(report.stripped, 1);
    let out = stripped_src(LangId::Scss, src);
    assert!(out.contains("/* TRIPWIRE: the block form is tagged */"));
    assert!(!out.contains("// untagged line prose"));
}

#[test]
fn rust_multiline_untagged_block_is_fully_stripped() {
    let src = "// this just explains the code below\n// which needs no explanation\nfn main() {}\n";
    let out = stripped_src(LangId::Rust, src);
    assert_eq!(out, "fn main() {}\n");
}

#[test]
fn swift_multiline_untagged_block_is_fully_stripped() {
    let src = "// obvious prose one\n// obvious prose two\nlet x = 1\n";
    let out = stripped_src(LangId::Swift, src);
    assert_eq!(out, "let x = 1\n");
}

#[test]
fn typescript_multiline_untagged_block_is_fully_stripped() {
    let src = "// obvious prose one\n// obvious prose two\nvar x = 1;\n";
    let out = stripped_src(LangId::TypeScript, src);
    assert_eq!(out, "var x = 1;\n");
}

#[test]
fn javascript_multiline_untagged_block_is_fully_stripped() {
    let src = "// obvious prose one\n// obvious prose two\nvar x = 1;\n";
    let out = stripped_src(LangId::JavaScript, src);
    assert_eq!(out, "var x = 1;\n");
}

#[test]
fn nix_multiline_untagged_block_is_fully_stripped() {
    let src = "# obvious prose one\n# obvious prose two\n{ pkgs }: pkgs.hello\n";
    let out = stripped_src(LangId::Nix, src);
    assert_eq!(out, "{ pkgs }: pkgs.hello\n");
}

#[test]
fn ruby_multiline_untagged_block_is_fully_stripped() {
    let src = "# obvious prose one\n# obvious prose two\nx = 1\n";
    let out = stripped_src(LangId::Ruby, src);
    assert_eq!(out, "x = 1\n");
}

#[test]
fn scss_multiline_untagged_block_is_fully_stripped() {
    let src = "// obvious prose one\n// obvious prose two\n.a { color: red; }\n";
    let out = stripped_src(LangId::Scss, src);
    assert_eq!(out, ".a { color: red; }\n");
}

#[test]
fn rust_untagged_leader_above_tag_is_stripped_and_tag_survives() {
    let src =
        "// ordinary prose leader\n// TRIPWIRE: deleting this orphans the record\nfn main() {}\n";
    assert_eq!(strip(LangId::Rust, src).stripped, 1);
    let out = stripped_src(LangId::Rust, src);
    assert!(!out.contains("// ordinary prose leader"));
    assert!(out.contains("// TRIPWIRE: deleting this orphans the record"));
}

#[test]
fn nix_untagged_leader_above_tag_is_stripped_and_tag_survives() {
    let src = "# ordinary prose leader\n# TRIPWIRE: dropping this pulls in the broken version\n{ pkgs }: pkgs.hello\n";
    assert_eq!(strip(LangId::Nix, src).stripped, 1);
    let out = stripped_src(LangId::Nix, src);
    assert!(!out.contains("# ordinary prose leader"));
    assert!(out.contains("# TRIPWIRE: dropping this pulls in the broken version"));
}

#[test]
fn rust_continuation_separated_by_blank_line_is_its_own_untagged_block() {
    let src = "// TRIPWIRE: this is the real reason\n\n// this is separated prose that is not part of the block\nfn main() {}\n";
    assert_eq!(strip(LangId::Rust, src).stripped, 1);
    let out = stripped_src(LangId::Rust, src);
    assert!(out.contains("// TRIPWIRE: this is the real reason"));
    assert!(!out.contains("this is separated prose that is not part of the block"));
}

#[test]
fn rust_continuation_separated_by_code_is_its_own_untagged_block() {
    let src = "// TRIPWIRE: this is the real reason\nlet a = 1;\n// separated prose after code\nfn main() {}\n";
    assert_eq!(strip(LangId::Rust, src).stripped, 1);
    let out = stripped_src(LangId::Rust, src);
    assert!(out.contains("// TRIPWIRE: this is the real reason"));
    assert!(out.contains("let a = 1;"));
    assert!(!out.contains("separated prose after code"));
}

#[test]
fn nix_continuation_separated_by_blank_line_is_its_own_untagged_block() {
    let src = "# TRIPWIRE: the real reason\n\n# separated prose that is not part of the block\n{ pkgs }: pkgs.hello\n";
    assert_eq!(strip(LangId::Nix, src).stripped, 1);
    let out = stripped_src(LangId::Nix, src);
    assert!(out.contains("# TRIPWIRE: the real reason"));
    assert!(!out.contains("separated prose that is not part of the block"));
}

#[test]
fn rust_doc_surface_untagged_is_exempt_in_app_target() {
    let src = "/// docs\n#[derive(SimpleObject)]\npub struct T;\n";
    let cfg = config_with_rust_surface(&["SimpleObject"]);
    let report = process(LangId::Rust, src, false, &cfg, true).unwrap();
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn rust_doc_surface_untagged_is_judged_and_stripped_in_test_target() {
    let src = "/// docs\n#[derive(SimpleObject)]\npub struct T;\n";
    let cfg = config_with_rust_surface(&["SimpleObject"]);
    let report = process(LangId::Rust, src, true, &cfg, true).unwrap();
    assert_eq!(report.untagged_found, 1);
    let out = report.new_source.unwrap();
    assert!(!out.contains("/// docs"));
    assert!(out.contains("#[derive(SimpleObject)]"));
    assert!(out.contains("pub struct T;"));
}

#[test]
fn tagged_leader_run_stops_at_a_directive_and_trailing_untagged_is_stripped() {
    let src = "# TRIPWIRE: keep the unquoted expansion below\n# shellcheck disable=SC2086\n# trailing untagged prose\necho $x\n";
    let report = strip(LangId::Bash, src);
    assert_eq!(report.stripped, 1);
    let out = stripped_src(LangId::Bash, src);
    assert!(out.contains("# TRIPWIRE: keep the unquoted expansion below"));
    assert!(out.contains("# shellcheck disable=SC2086"));
    assert!(!out.contains("trailing untagged prose"));
}

#[test]
fn tagged_leader_followed_by_a_second_tag_keeps_both() {
    let src = "// TRIPWIRE: first independent reason\n// CONTEXT: second independent fact\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn html_multiline_tripwire_note_survives_whole() {
    let src = "<!-- TRIPWIRE: this z-index sits above the sticky header\n     lowering it hides the dropdown behind it -->\n<p>x</p>\n";
    let report = strip(LangId::Html, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn html_multiline_untagged_block_is_fully_stripped() {
    let src = "<!-- obvious prose one\n     obvious prose two -->\n<p>x</p>\n";
    let out = stripped_src(LangId::Html, src);
    assert_eq!(out, "<p>x</p>\n");
}

#[test]
fn html_comment_body_drops_both_delimiters() {
    let spec = spec_for(LangId::Html);
    let body = comment_body(
        "<!-- TRIPWIRE: a note -->",
        spec.line_prefixes,
        spec.block_open,
        spec.block_close,
    );
    assert_eq!(body, "TRIPWIRE: a note");
}

#[test]
fn erb_comment_body_drops_both_delimiters() {
    let spec = spec_for(LangId::Erb);
    let body = comment_body(
        "<%# TRIPWIRE: a note %>",
        spec.line_prefixes,
        spec.block_open,
        spec.block_close,
    );
    assert_eq!(body, "TRIPWIRE: a note");
}

#[test]
fn erb_multiline_tripwire_directive_is_one_comment() {
    let src = "<%# TRIPWIRE: this partial is rendered by two mailers\n    renaming a local here breaks the other one %>\n<h1>x</h1>\n";
    let report = strip(LangId::Erb, src);
    assert_eq!(report.untagged_found, 0);
    assert_eq!(report.unclassified, 0);
    let parsed = parse(LangId::Erb, src, false, Config::empty().tags()).unwrap();
    assert_eq!(parsed.comments.len(), 1);
}
