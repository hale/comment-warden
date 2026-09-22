use comment_warden::config::Config;
use comment_warden::lang::LangId;
use comment_warden::parse::code_tokens;
use comment_warden::strip::process;

fn fixture(name: &str) -> &'static str {
    match name {
        "rust_strings.rs" => include_str!("fixtures/rust_strings.rs"),
        "swift_doc.swift" => include_str!("fixtures/swift_doc.swift"),
        "hcl_hash.tf" => include_str!("fixtures/hcl_hash.tf"),
        "real_shebang.sh" => include_str!("fixtures/real_shebang.sh"),
        "nix_shebang.nix" => include_str!("fixtures/nix_shebang.nix"),
        "python_comments.py" => include_str!("fixtures/python_comments.py"),
        "ruby_comments.rb" => include_str!("fixtures/ruby_comments.rb"),
        "scss_comments.scss" => include_str!("fixtures/scss_comments.scss"),
        "html_comments.html" => include_str!("fixtures/html_comments.html"),
        "erb_comments.erb" => include_str!("fixtures/erb_comments.erb"),
        other => panic!("unknown fixture {other}"),
    }
}

fn strip_fixture(id: LangId, name: &str) -> (String, comment_warden::strip::FileReport) {
    let src = fixture(name);
    let report = process(id, src, false, &Config::empty(), true).unwrap();
    let out = report.new_source.clone().unwrap_or_else(|| src.to_string());
    assert_eq!(
        code_tokens(id, &out).unwrap(),
        code_tokens(id, src).unwrap(),
        "projection changed stripping {name}"
    );
    (out, report)
}

#[test]
fn rust_strings_fixture_keeps_strings_and_tags() {
    let (out, report) = strip_fixture(LangId::Rust, "rust_strings.rs");
    assert!(out.contains("// this looks like a comment but is SQL-adjacent"));
    assert!(out.contains("//! TRIPWIRE:"));
    assert!(out.contains("// TRIPWIRE: a tagged line keeps the continuation lines below it"));
    assert!(
        out.contains("this second line is part of that note and survives with the tag above it")
    );
    assert!(!out.contains("this line explains nothing that the code doesn't already say"));
    assert!(!out.contains("// trailing untagged comment"));
    assert!(report.stripped >= 1);
}

#[test]
fn swift_doc_fixture_reports_untagged_doc() {
    let (out, report) = strip_fixture(LangId::Swift, "swift_doc.swift");
    assert!(out.contains("// not a comment, just a string"));
    assert!(
        report
            .untagged_lines
            .iter()
            .any(|(_, line)| line.contains("This doc comment carries no tag")),
        "the untagged Swift doc comment should be reported: {:?}",
        report.untagged_lines
    );
    assert!(!out.contains("// trailing untagged comment"));
}

#[test]
fn hcl_fixture_keeps_keep_tag() {
    let (out, _) = strip_fixture(LangId::Hcl, "hcl_hash.tf");
    assert!(out.contains("# TRIPWIRE: changing this endpoint"));
    assert!(!out.contains("# trailing untagged comment"));
    assert!(!out.contains("# this line explains nothing"));
}

#[test]
fn real_shebang_fixture_keeps_shebang_and_directives() {
    let (out, _) = strip_fixture(LangId::Bash, "real_shebang.sh");
    assert!(out.starts_with("#!/usr/bin/env bash\n"));
    assert!(out.contains("# shellcheck disable=SC2086"));
    assert!(out.contains("# shellcheck source=/dev/null"));
    assert!(out.contains("# TRIPWIRE: this script is invoked by a systemd unit"));
    assert!(!out.contains("# this line explains nothing"));
}

#[test]
fn nix_fixture_embedded_shell_left_in_place() {
    let src = fixture("nix_shebang.nix");
    let report = process(LangId::Nix, src, false, &Config::empty(), true).unwrap();
    let out = report.new_source.clone().unwrap_or_else(|| src.to_string());
    assert!(out.contains("# Verify authentication by listing vaults"));
    assert!(out.contains("# TRIPWIRE: removing this restart policy"));
    assert!(!out.contains("# This file contains the service account token"));
    assert!(report.left_in_place >= 1);
}

#[test]
fn python_fixture_keeps_string_template() {
    let (out, _) = strip_fixture(LangId::Python, "python_comments.py");
    assert!(out.contains("# not a comment, just a string"));
    assert!(out.contains("# TRIPWIRE: this fixture is the python positive control"));
    assert!(!out.contains("# ordinary prose that explains nothing"));
    assert!(!out.contains("# a trailing untagged comment"));
}

#[test]
fn ruby_fixture_keeps_strings_shebang_and_tags() {
    let (out, report) = strip_fixture(LangId::Ruby, "ruby_comments.rb");
    assert!(out.starts_with("#!/usr/bin/env ruby\n"));
    assert!(out.contains("\"# not a comment, just a string\""));
    assert!(out.contains("# still just a string"));
    assert!(out.contains("# TRIPWIRE: this fixture is the ruby positive control"));
    assert!(out.contains("# CONTEXT: the =begin block below"));
    assert!(out.contains("TRIPWIRE: this block form carries a tag and survives"));
    assert!(!out.contains("an untagged block comment that the gate removes"));
    assert!(!out.contains("# ordinary prose that explains nothing"));
    assert!(!out.contains("# a trailing untagged comment"));
    assert!(report.stripped >= 3);
}

#[test]
fn scss_fixture_keeps_strings_and_both_comment_forms() {
    let (out, report) = strip_fixture(LangId::Scss, "scss_comments.scss");
    assert!(out.contains("\"// not a comment, just a string\""));
    assert!(out.contains("\"/* also just a string */\""));
    assert!(out.contains("// TRIPWIRE: this fixture is the scss positive control"));
    assert!(out.contains("/* CONTEXT: the block form is the only comment css itself accepts */"));
    assert!(!out.contains("// ordinary prose that explains nothing"));
    assert!(!out.contains("an untagged block comment that the gate removes"));
    assert!(!out.contains("// a trailing untagged comment"));
    assert!(report.stripped >= 3);
}

#[test]
fn html_fixture_keeps_attribute_values_raw_text_and_tags() {
    let (out, report) = strip_fixture(LangId::Html, "html_comments.html");
    assert!(out.contains("title=\"<!-- not a comment, just an attribute value -->\""));
    assert!(out.contains("/* a css comment the html grammar reads as raw text"));
    assert!(out.contains("// a js comment the html grammar reads as raw text"));
    assert!(out.contains("<!-- TRIPWIRE: this fixture is the html positive control"));
    assert!(out.contains("html at all still passes. -->"));
    assert!(out.contains("<!-- CONTEXT: the block form is the only comment html has -->"));
    assert!(!out.contains("ordinary prose that explains nothing"));
    assert!(!out.contains("a trailing untagged comment"));
    assert_eq!(report.stripped, 2);
}

#[test]
fn erb_fixture_reports_untagged_directives_and_keeps_ruby_strings_and_tags() {
    let src = fixture("erb_comments.erb");
    let report = process(LangId::Erb, src, false, &Config::empty(), true).unwrap();
    assert!(report.new_source.is_none());
    assert_eq!(report.untagged_found, 2);
    assert_eq!(report.unclassified, 0);
    assert!(
        report
            .untagged_lines
            .iter()
            .any(|(_, line)| line.contains("ordinary prose that explains nothing"))
    );
    assert!(
        report
            .untagged_lines
            .iter()
            .any(|(_, line)| line.contains("a trailing untagged comment"))
    );
    assert!(
        !report
            .untagged_lines
            .iter()
            .any(|(_, line)| line.contains("a # hash inside a ruby string"))
    );
    assert!(
        !report
            .untagged_lines
            .iter()
            .any(|(_, line)| line.contains("TRIPWIRE") || line.contains("CONTEXT"))
    );
}
