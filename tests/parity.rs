use comment_warden::config::Config;
use comment_warden::lang::LangId;
use comment_warden::strip::{FileReport, process};

fn check(id: LangId, src: &str) -> FileReport {
    process(id, src, false, &Config::empty(), false).unwrap()
}

fn apply(id: LangId, src: &str) -> FileReport {
    process(id, src, false, &Config::empty(), true).unwrap()
}

#[test]
fn tagged_rust_doc_leader_absorbs_its_doc_continuation_lines() {
    let src = "/// TRIPWIRE: tag on doc line\n/// doc continuation\npub fn b() {}\n";
    let report = check(LangId::Rust, src);
    assert_eq!(report.untagged_found, 0);
    assert!(report.untagged_lines.is_empty());
}

#[test]
fn multi_line_tagged_rust_doc_note_survives_whole() {
    let src = "/// TRIPWIRE: this note\n/// spans three\n/// doc lines\npub fn b() {}\n";
    let report = check(LangId::Rust, src);
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn untagged_rust_doc_leader_still_reports_but_does_not_swallow_a_tag_below() {
    let src = "/// plain doc line\n/// TRIPWIRE: this one is tagged\npub fn b() {}\n";
    let report = check(LangId::Rust, src);
    assert_eq!(report.untagged_found, 1);
    assert_eq!(report.untagged_lines[0].0, 0);
}

#[test]
fn tagged_comment_in_run_command_body_is_clean() {
    let src =
        "{ runCommand \"x\" {} ''\n  set -e\n  # TRIPWIRE: asserts the thing\n  do_check\n'' }\n";
    let report = check(LangId::Nix, src);
    assert_eq!(report.untagged_found, 0);
    assert_eq!(report.unclassified, 0);
}

#[test]
fn multi_line_tagged_comment_in_post_fixup_body_survives_whole() {
    let src = "{ postFixup = lib.optionalString true ''\n  patch it\n  # TRIPWIRE: assert the patch took here\n  # rather than let it surface later as a broken binary.\n  check it\n''; }\n";
    let report = check(LangId::Nix, src);
    assert_eq!(report.untagged_found, 0);
    assert_eq!(report.unclassified, 0);
}

#[test]
fn untagged_comment_in_shell_body_is_reported_as_untagged() {
    let src =
        "{ buildPhase = ''\n  make all\n  # just a note nobody tagged\n  make install\n''; }\n";
    let report = check(LangId::Nix, src);
    assert_eq!(report.untagged_found, 1);
    assert_eq!(report.unclassified, 0);
}

#[test]
fn comment_in_python_heredoc_inside_shell_body_is_parsed() {
    let src =
        "{ buildPhase = ''\n  python3 <<EOF\n  # untagged python note\n  print(1)\n  EOF\n''; }\n";
    let report = check(LangId::Nix, src);
    assert_eq!(report.untagged_found, 1);
}

#[test]
fn hash_line_in_opaque_nix_string_is_unclassified() {
    let src = "{ description = ''\n  # this is prose in an opaque string\n''; }\n";
    let report = check(LangId::Nix, src);
    assert_eq!(report.unclassified, 1);
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn tagged_hash_line_in_opaque_nix_string_is_clean() {
    let src = "{ description = ''\n  # TRIPWIRE: a human judged this prose\n''; }\n";
    let report = check(LangId::Nix, src);
    assert_eq!(report.unclassified, 0);
    assert_eq!(report.untagged_found, 0);
}

#[test]
fn tagged_rust_doc_leader_with_continuation_survives_an_apply() {
    let src = "/// TRIPWIRE: x\n/// cont\npub fn a() {}\n";
    let report = apply(LangId::Rust, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn untagged_rust_doc_over_code_strips_the_note_and_keeps_the_code_line_byte_exact() {
    let src = "/// junk\npub fn a() {}\n";
    let report = apply(LangId::Rust, src);
    assert_eq!(report.new_source, Some("pub fn a() {}\n".to_string()));
}

#[test]
fn tagged_rust_doc_note_at_eof_without_trailing_newline_survives_an_apply() {
    let src = "/// TRIPWIRE: x\n/// cont\npub fn a() {}";
    let report = apply(LangId::Rust, src);
    assert_eq!(report.stripped, 0);
    assert!(report.new_source.is_none());
}

#[test]
fn mixed_line_and_doc_leaders_do_not_merge_so_tagged_line_survives_and_untagged_doc_strips() {
    let src = "// TRIPWIRE: tag\n/// doc line\npub fn a() {}\n";
    let report = apply(LangId::Rust, src);
    assert_eq!(
        report.new_source,
        Some("// TRIPWIRE: tag\npub fn a() {}\n".to_string())
    );
}

#[test]
fn opaque_nix_string_with_non_ascii_comment_block_classifies_without_panicking() {
    let src =
        "{ description = ''\n  # this is prose in an opaque string\n'';\n}\n# TRIPWIRE: note é\n";
    let report = check(LangId::Nix, src);
    assert_eq!(report.unclassified, 1);
}
