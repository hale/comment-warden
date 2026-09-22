use comment_warden::config::Config;
use comment_warden::lang::LangId;
use comment_warden::parse::code_tokens;
use comment_warden::strip::{FileReport, process};

fn strip(id: LangId, src: &str) -> FileReport {
    process(id, src, false, &Config::empty(), true).unwrap()
}

fn stripped_src(id: LangId, src: &str) -> String {
    strip(id, src).new_source.unwrap_or_else(|| src.to_string())
}

#[test]
fn untagged_line_comment_is_removed() {
    let src = "// nothing\nfn main() {}\n";
    let out = stripped_src(LangId::Rust, src);
    assert_eq!(out, "fn main() {}\n");
}

#[test]
fn keep_tag_survives() {
    let src = "// TRIPWIRE: deleting this orphans the record\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn keep_with_empty_reason_is_untagged() {
    let src = "// TRIPWIRE:\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.stripped, 1);
}

#[test]
fn comment_shaped_text_in_string_is_not_a_comment() {
    let src = "fn f() -> &'static str { \"// not a comment\" }\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn raw_string_slashes_survive_strip() {
    let src = "fn f() -> &'static str {\n    r#\"\n    // looks like a comment\n    \"#\n}\n// real comment\n";
    let out = stripped_src(LangId::Rust, src);
    assert!(out.contains("// looks like a comment"));
    assert!(!out.contains("// real comment"));
}

#[test]
fn trailing_comment_keeps_the_code_and_newline() {
    let src = "let x = 1; // trailing\nlet y = 2;\n";
    let out = stripped_src(LangId::Rust, src);
    assert_eq!(out, "let x = 1;\nlet y = 2;\n");
}

#[test]
fn leading_inline_block_comment_keeps_code() {
    let src = "fn f() { /* x */ do_it() }\n";
    let out = stripped_src(LangId::Rust, src);
    assert_eq!(out, "fn f() { do_it() }\n");
}

#[test]
fn whole_line_block_takes_the_line() {
    let src = "/* alone */\nfn main() {}\n";
    let out = stripped_src(LangId::Rust, src);
    assert_eq!(out, "fn main() {}\n");
}

#[test]
fn untagged_continuation_under_a_tagged_leader_survives() {
    let src =
        "// TRIPWIRE: first line has the tag\n// second line is untagged prose\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Rust, src);
    assert!(out.contains("// TRIPWIRE: first line has the tag"));
    assert!(out.contains("second line is untagged prose"));
}

#[test]
fn untagged_above_keep_same_column_keeps_the_keep() {
    let src = "// ordinary prose\n// TRIPWIRE: deleting this orphans the record\nfn main() {}\n";
    let out = stripped_src(LangId::Rust, src);
    assert!(!out.contains("// ordinary prose"));
    assert!(out.contains("// TRIPWIRE: deleting this orphans the record"));
}

#[test]
fn untagged_above_a_tag_is_stripped_but_continuation_below_survives() {
    let src = "// untagged one\n// TRIPWIRE: real reason\n// untagged two\nfn main() {}\n";
    let out = stripped_src(LangId::Rust, src);
    assert!(
        !out.contains("untagged one"),
        "a separate block above the tag"
    );
    assert!(out.contains("// TRIPWIRE: real reason"));
    assert!(
        out.contains("untagged two"),
        "a continuation of the tag below it"
    );
    let recheck = process(LangId::Rust, &out, false, &Config::empty(), false).unwrap();
    assert_eq!(
        recheck.untagged_found, 0,
        "the surviving continuation belongs to the tagged block, not an untagged one"
    );
}

#[test]
fn two_untagged_same_column_both_stripped() {
    let src = "// untagged1\n// untagged2\nfn main() {}\n";
    let out = stripped_src(LangId::Rust, src);
    assert!(!out.contains("untagged1"));
    assert!(!out.contains("untagged2"));
    assert_eq!(out, "fn main() {}\n");
}

#[test]
fn untagged_above_directive_keeps_the_directive() {
    let src = "# untagged\n# shellcheck disable=SC2086\necho $x\n";
    let out = stripped_src(LangId::Bash, src);
    assert!(!out.contains("# untagged"));
    assert!(out.contains("# shellcheck disable=SC2086"));
}

#[test]
fn shebang_is_exempt() {
    let src = "#!/usr/bin/env bash\n# a comment\necho hi\n";
    let report = strip(LangId::Bash, src);
    assert_eq!(report.untagged_found, 1);
    let out = report.new_source.unwrap();
    assert!(out.starts_with("#!/usr/bin/env bash\n"));
    assert!(!out.contains("# a comment"));
}

#[test]
fn shellcheck_directive_is_exempt() {
    let src = "# shellcheck disable=SC2086\necho $x\n";
    let report = strip(LangId::Bash, src);
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn swift_tools_version_directive_is_exempt() {
    let src = "// swift-tools-version:5.9\nimport PackageDescription\n";
    let report = strip(LangId::Swift, src);
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn rubocop_directive_is_exempt() {
    let src = "# rubocop:disable Metrics/MethodLength\ndef x; end\n# rubocop:enable Metrics/MethodLength\n";
    let report = strip(LangId::Ruby, src);
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn frozen_string_literal_directive_is_exempt() {
    let src = "# frozen_string_literal: true\nx = 1\n";
    let report = strip(LangId::Ruby, src);
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn stylelint_directives_are_exempt() {
    let css = "/* stylelint-disable no-descending-specificity */\na { color: red; }\n/* stylelint-enable no-descending-specificity */\n";
    assert_eq!(strip(LangId::Css, css).untagged_found, 0);

    let scss = "/* stylelint-disable-next-line declaration-no-important */\n.a { color: red; }\n";
    assert_eq!(strip(LangId::Scss, scss).untagged_found, 0);

    let line = "// stylelint-disable-line declaration-no-important\n.a { color: red; }\n";
    assert_eq!(strip(LangId::Scss, line).untagged_found, 0);
}

#[test]
fn eslint_global_directive_is_exempt() {
    let src = "/* global ace */\nace.edit('editor');\n";
    assert_eq!(strip(LangId::JavaScript, src).untagged_found, 0);

    let many = "/* globals ace, jQuery:writable */\nace.edit('editor');\n";
    assert_eq!(strip(LangId::TypeScript, many).untagged_found, 0);
}

#[test]
fn global_prose_comment_is_not_exempt() {
    let src = "// global config lives here\nconst x = 1;\n";
    assert_eq!(strip(LangId::JavaScript, src).untagged_found, 1);
}

#[test]
fn webpack_magic_comments_are_exempt() {
    let src = "import(/* webpackChunkName: \"admin\" */ './admin');\n";
    assert_eq!(strip(LangId::JavaScript, src).untagged_found, 0);

    let siblings = "import(/* webpackPrefetch: true */ './a');\nimport(/* webpackMode: \"lazy\" */ './b');\nimport(/* webpackIgnore: true */ './c');\n";
    assert_eq!(strip(LangId::TypeScript, siblings).untagged_found, 0);
}

#[test]
fn generated_file_marker_skips_whole_file() {
    let src = "// @generated by prost\n// ordinary comment\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn generated_marker_after_shebang_skips_whole_file() {
    let src = "#!/usr/bin/env python3\n# @generated by tool\n# ordinary comment\nprint(1)\n";
    let report = strip(LangId::Python, src);
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn generated_mention_not_opening_first_line_comment_is_not_generated() {
    let src = "fn main() {}\n// keep in sync with the @generated bindings\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.untagged_found, 1);
}

#[test]
fn generated_marker_in_doc_comment_is_not_generated() {
    let src = "/// @generated\npub struct T;\n// ordinary\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert!(
        report.untagged_found > 0,
        "a /// @generated doc comment must not switch the gate off for the whole file"
    );
}

#[test]
fn doc_comment_untagged_without_config_is_reported() {
    let src = "/// docs\npub struct T;\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.untagged_found, 1);
}

#[test]
fn doc_comment_exempt_when_config_matches_attribute() {
    let src = "/// docs\n#[derive(SimpleObject)]\npub struct T;\n";
    let cfg = config_with_rust_surface(&["SimpleObject"]);
    let report = process(LangId::Rust, src, false, &cfg, true).unwrap();
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn doc_comment_in_test_target_is_never_exempt() {
    let src = "/// docs\n#[derive(SimpleObject)]\npub struct T;\n";
    let cfg = config_with_rust_surface(&["SimpleObject"]);
    let report = process(LangId::Rust, src, true, &cfg, true).unwrap();
    assert_eq!(report.untagged_found, 1);
}

#[test]
fn doc_surface_climbs_to_enclosing_struct_for_field_doc() {
    let src = "#[derive(SimpleObject)]\npub struct Widget {\n    /// the id, copied into the SDL\n    id: Uuid,\n}\n";
    let cfg = config_with_rust_surface(&["SimpleObject"]);
    let report = process(LangId::Rust, src, false, &cfg, true).unwrap();
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn doc_surface_climbs_to_enclosing_enum_for_variant_doc() {
    let src = "#[derive(Enum)]\npub enum Status {\n    /// the item is active\n    Active,\n}\n";
    let cfg = config_with_rust_surface(&["Enum"]);
    let report = process(LangId::Rust, src, false, &cfg, true).unwrap();
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn doc_surface_climbs_to_enclosing_impl_for_resolver_doc() {
    let src = "#[Object]\nimpl Widget {\n    /// resolves the items\n    async fn items(&self) -> Vec<String> {\n        vec![]\n    }\n}\n";
    let cfg = config_with_rust_surface(&["Object"]);
    let report = process(LangId::Rust, src, false, &cfg, true).unwrap();
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn field_doc_is_judged_when_enclosing_type_has_no_surface_attribute() {
    let src = "pub struct Plain {\n    /// an internal note, not a surface\n    id: Uuid,\n}\n";
    let cfg = config_with_rust_surface(&["SimpleObject"]);
    let report = process(LangId::Rust, src, false, &cfg, true).unwrap();
    assert_eq!(report.untagged_found, 1);
}

#[test]
fn field_doc_under_exported_type_is_judged_in_test_target() {
    let src = "#[derive(SimpleObject)]\npub struct Widget {\n    /// the id\n    id: Uuid,\n}\n";
    let cfg = config_with_rust_surface(&["SimpleObject"]);
    let report = process(LangId::Rust, src, true, &cfg, true).unwrap();
    assert_eq!(report.untagged_found, 1);
}

#[test]
fn erb_comment_directive_is_reported_not_stripped() {
    // TRIPWIRE: an erb `content` node is raw output text, so removing a comment line changes the projection and the rendered whitespace with it — reporting without rewriting is the only safe answer here, not a gap to close.
    let src = "<%# a note %>\n<h1>x</h1>\n";
    let report = strip(LangId::Erb, src);
    assert_eq!(report.untagged_found, 1);
    assert_eq!(report.left_in_place, 1);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn erb_hash_line_in_an_opaque_code_region_is_unclassified() {
    let src = "<%\n  # a ruby note\n  y = 1\n%>\n";
    let report = strip(LangId::Erb, src);
    assert_eq!(report.unclassified, 1);
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn erb_hash_inside_a_ruby_string_is_not_a_comment() {
    let src = "<%= \"a # hash\" %>\n";
    let report = strip(LangId::Erb, src);
    assert_eq!(report.untagged_found, 0);
    assert_eq!(report.unclassified, 0);
}

#[test]
fn html_comment_in_a_quoted_attribute_value_is_left_alone() {
    let src = "<p title=\"<!-- not a comment -->\">x</p>\n";
    let report = strip(LangId::Html, src);
    assert_eq!(report.untagged_found, 0);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn html_comment_in_script_raw_text_is_left_alone() {
    let src = "<script>\n// a js note\n</script>\n";
    let report = strip(LangId::Html, src);
    assert_eq!(report.untagged_found, 0);
    assert_eq!(report.unclassified, 0);
}

#[test]
fn html_comment_survives_alongside_erb_tags_an_mjml_file_carries() {
    let src = "<mj-section>\n<!-- a note -->\n<mj-text><%= x %></mj-text>\n</mj-section>\n";
    let report = strip(LangId::Html, src);
    assert_eq!(report.untagged_found, 1);
    assert_eq!(report.stripped, 1);
    assert_eq!(report.unclassified, 0);
}

#[test]
fn embedded_nix_shell_comment_is_reported_not_stripped() {
    let src = "{ pkgs }:\n{\n  script = ''\n    #!/bin/bash\n    # embedded comment\n    echo hi\n  '';\n}\n";
    let report = strip(LangId::Nix, src);
    assert_eq!(report.left_in_place, 1);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Nix, src);
    assert!(out.contains("# embedded comment"));
}

#[test]
fn curried_write_shell_script_body_is_recognized_not_stripped() {
    let src =
        "{ pkgs }:\npkgs.writeShellScript \"deploy\" ''\n  # embedded comment\n  echo hi\n''\n";
    let report = strip(LangId::Nix, src);
    assert_eq!(report.left_in_place, 1);
    assert_eq!(report.unclassified, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Nix, src);
    assert!(out.contains("# embedded comment"));
}

#[test]
fn opaque_nix_string_comment_is_unclassified() {
    let src = "{\n  description = ''\n    # this string is not a known script\n  '';\n}\n";
    let report = strip(LangId::Nix, src);
    assert_eq!(report.unclassified, 1);
}

#[test]
fn python_string_template_comment_survives() {
    let src = "T = \"\"\"\n# not a comment\n\"\"\"\n# real one\nx = 1\n";
    let out = stripped_src(LangId::Python, src);
    assert!(out.contains("# not a comment"));
    assert!(!out.contains("# real one"));
}

#[test]
fn code_tokens_unchanged_after_strip_across_langs() {
    let cases = [
        (LangId::Rust, "// c\nfn a() { let x = 1; } // t\n"),
        (LangId::Python, "# c\ndef f():\n    return 1  # t\n"),
        (LangId::Go, "// c\npackage main\n"),
        (LangId::Hcl, "# c\nresource \"a\" \"b\" {}\n"),
        (LangId::Css, "/* c */\na { color: red; }\n"),
        (LangId::Scss, "// c\n$g: 1px;\na { color: red; } // t\n"),
        (LangId::Html, "<!-- c -->\n<p>x</p> <!-- t -->\n"),
        (LangId::Ruby, "# c\ndef f\n  1 # t\nend\n"),
    ];
    for (id, src) in cases {
        let out = stripped_src(id, src);
        assert_eq!(
            code_tokens(id, &out).unwrap(),
            code_tokens(id, src).unwrap(),
            "projection changed for {id:?}"
        );
    }
}

#[test]
fn doc_comment_inside_macro_invocation_is_exempt_and_survives() {
    let src = "macro_rules! m {\n    () => {\n        /// exported doc inside a macro\n        pub fn generated() {}\n    };\n}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn each_directive_kind_is_exempt() {
    let cases = [
        (LangId::Bash, "# shellcheck disable=SC2086\necho $x\n"),
        (
            LangId::Swift,
            "// swiftlint:disable force_cast\nlet x = 1\n",
        ),
        (LangId::Swift, "// swift-tools-version:5.9\nlet x = 1\n"),
        (
            LangId::JavaScript,
            "// eslint-disable-next-line no-unused\nvar x = 1;\n",
        ),
        (LangId::JavaScript, "// prettier-ignore\nvar x = 1;\n"),
        (
            LangId::TypeScript,
            "// @ts-expect-error legacy\nvar x = 1;\n",
        ),
        (LangId::Rust, "// clippy::all\nfn main() {}\n"),
    ];
    for (id, src) in cases {
        let report = strip(id, src);
        assert_eq!(report.untagged_found, 0, "directive not exempt: {src:?}");
    }
}

#[test]
fn keep_takes_precedence_over_directive_shape() {
    let src = "# TRIPWIRE: shellcheck note we rely on\necho hi\n";
    let report = strip(LangId::Bash, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn mid_line_keep_is_not_treated_as_tagged() {
    let src = "// see TRIPWIRE: guidance elsewhere\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.stripped, 1);
}

#[test]
fn context_tag_survives_by_default() {
    let src = "// CONTEXT: upstream returns 200 on failure, so we check the body\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn tags_are_configurable() {
    let cfg = config_with_tags(&["KEEP"]);
    let src = "// KEEP: still load-bearing here\nfn main() {}\n// TRIPWIRE: not a tag in this repo\nfn other() {}\n";
    let report = process(LangId::Rust, src, false, &cfg, true).unwrap();
    let out = report.new_source.clone().unwrap();
    assert!(out.contains("// KEEP: still load-bearing here"));
    assert!(!out.contains("TRIPWIRE"));
}

#[test]
fn rust_module_doc_is_exempt_in_app_target() {
    let src = "//! crate-level docs copied into the rendered docs\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn rust_block_module_doc_is_exempt_in_app_target() {
    let src = "/*! block form crate-level docs */\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn swift_module_doc_is_exempt_in_app_target() {
    let src = "//! file-level docs\nlet x = 1\n";
    let report = strip(LangId::Swift, src);
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn module_doc_is_judged_in_test_target() {
    let src = "//! not a shipped surface in a test target\nfn main() {}\n";
    let report = process(LangId::Rust, src, true, &Config::empty(), true).unwrap();
    assert_eq!(report.untagged_found, 1);
}

#[test]
fn multi_line_module_doc_block_all_survives() {
    let src = "//! first line of crate docs\n//! second line of crate docs\n//! third line of crate docs\nfn main() {}\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.untagged_found, 0);
    assert!(report.new_source.is_none());
    let out = stripped_src(LangId::Rust, src);
    assert!(out.contains("//! first line of crate docs"));
    assert!(out.contains("//! second line of crate docs"));
    assert!(out.contains("//! third line of crate docs"));
}

#[test]
fn outer_doc_without_surface_attribute_is_still_judged() {
    let src = "/// an internal outer doc with no exported enclosing item\npub struct T;\n";
    let report = strip(LangId::Rust, src);
    assert_eq!(report.untagged_found, 1);
}

fn config_with_tags(tags: &[&str]) -> Config {
    let list = tags
        .iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let dir = tempdir();
    let path = dir.join("comment-warden.toml");
    std::fs::write(&path, format!("tags = [{list}]\n")).unwrap();
    Config::load(Some(&path)).unwrap()
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
        "comment-warden-test-{}-{}-{:?}",
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
