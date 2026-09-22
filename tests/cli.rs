use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_comment-warden")
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tempdir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!(
        "comment-warden-cli-{}-{}-{:?}",
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

fn write(path: &Path, contents: &str) {
    std::fs::write(path, contents).unwrap();
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(bin()).args(args).output().unwrap()
}

fn run_in(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn run_stdin(args: &[&str], stdin: &str) -> std::process::Output {
    let mut child = Command::new(bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn code(out: &std::process::Output) -> i32 {
    out.status.code().unwrap()
}

#[test]
fn check_clean_tree_exits_zero() {
    let dir = tempdir();
    write(&dir.join("a.rs"), "// TRIPWIRE: needed\nfn main() {}\n");
    let out = run(&["check", dir.join("a.rs").to_str().unwrap()]);
    assert_eq!(code(&out), 0);
}

#[test]
fn check_untagged_exits_one() {
    let dir = tempdir();
    write(&dir.join("a.rs"), "// nothing\nfn main() {}\n");
    let out = run(&["check", dir.join("a.rs").to_str().unwrap()]);
    assert_eq!(code(&out), 1);
}

#[test]
fn check_reads_an_mjml_file_through_the_html_grammar() {
    let dir = tempdir();
    write(
        &dir.join("mail.html.mjml"),
        "<mj-section>\n<!-- nothing -->\n<mj-text><%= x %></mj-text>\n</mj-section>\n",
    );
    let out = run(&["check", dir.join("mail.html.mjml").to_str().unwrap()]);
    assert_eq!(code(&out), 1);
}

#[test]
fn check_reads_an_erb_file() {
    let dir = tempdir();
    write(&dir.join("show.html.erb"), "<%# nothing %>\n<h1>x</h1>\n");
    let out = run(&["check", dir.join("show.html.erb").to_str().unwrap()]);
    assert_eq!(code(&out), 1);
}

#[test]
fn check_only_unsupported_extensions_exits_two() {
    let dir = tempdir();
    write(&dir.join("readme.unknownext"), "whatever\n");
    let out = run(&["check", dir.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
}

#[test]
fn check_nonexistent_path_exits_three() {
    let out = run(&["check", "/no/such/path/here.rs"]);
    assert_eq!(code(&out), 3);
}

#[test]
fn strip_bad_config_exits_three() {
    let dir = tempdir();
    write(
        &dir.join("comment-warden.toml"),
        "this is = not valid toml [[[\n",
    );
    write(&dir.join("a.rs"), "// nothing\nfn main() {}\n");
    let out = run(&[
        "strip",
        "--config",
        dir.join("comment-warden.toml").to_str().unwrap(),
        dir.join("a.rs").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 3);
}

#[test]
fn check_exclude_from_stale_entry_exits_three_naming_the_path() {
    let dir = tempdir();
    write(&dir.join("a.rs"), "// nothing\nfn main() {}\n");
    let stale = dir.join("gone.rs");
    let excludes = dir.join("excludes.txt");
    write(&excludes, &format!("{}\n", stale.to_str().unwrap()));

    let out = run(&[
        "check",
        "--exclude-from",
        excludes.to_str().unwrap(),
        dir.join("a.rs").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 3);
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains(stale.to_str().unwrap()),
        "error must name the stale entry: {stderr}"
    );
}

#[test]
fn check_exclude_from_all_entries_exist_behaves_normally() {
    let dir = tempdir();
    write(&dir.join("a.rs"), "// nothing\nfn main() {}\n");
    let existing = dir.join("b.rs");
    write(&existing, "fn other() {}\n");
    let excludes = dir.join("excludes.txt");
    write(
        &excludes,
        &format!("# a comment\n{}\n", existing.to_str().unwrap()),
    );

    let out = run(&[
        "check",
        "--exclude-from",
        excludes.to_str().unwrap(),
        dir.join("a.rs").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 1, "a.rs has an untagged comment");
}

#[test]
fn hook_strips_and_emits_receipt() {
    let dir = tempdir();
    let file = dir.join("a.rs");
    write(&file, "// nothing\nfn main() {}\n");
    let payload = format!(
        "{{\"tool_input\":{{\"file_path\":{:?}}}}}",
        file.to_str().unwrap()
    );
    let out = run_stdin(&["hook"], &payload);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("PostToolUse"), "receipt missing: {stdout}");
    assert!(stdout.contains("stripped"));
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, "fn main() {}\n");
}

#[test]
fn hook_is_silent_on_clean_file() {
    let dir = tempdir();
    let file = dir.join("a.rs");
    write(&file, "fn main() {}\n");
    let payload = format!(
        "{{\"tool_input\":{{\"file_path\":{:?}}}}}",
        file.to_str().unwrap()
    );
    let out = run_stdin(&["hook"], &payload);
    assert_eq!(code(&out), 0);
    assert!(String::from_utf8(out.stdout).unwrap().is_empty());
}

#[test]
fn hook_with_bad_config_still_strips_and_exits_zero() {
    let dir = tempdir();
    write(&dir.join("comment-warden.toml"), "not = valid [[[ toml\n");
    let file = dir.join("a.rs");
    write(&file, "// nothing\nfn main() {}\n");
    let payload = format!(
        "{{\"tool_input\":{{\"file_path\":{:?}}}}}",
        file.to_str().unwrap()
    );
    let out = run_stdin(
        &[
            "hook",
            "--config",
            dir.join("comment-warden.toml").to_str().unwrap(),
        ],
        &payload,
    );
    assert_eq!(code(&out), 0);
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, "fn main() {}\n");
}

#[test]
fn hook_unsupported_extension_exits_zero_silent() {
    let dir = tempdir();
    let file = dir.join("data.unknownext");
    write(&file, "# nothing\n");
    let payload = format!(
        "{{\"tool_input\":{{\"file_path\":{:?}}}}}",
        file.to_str().unwrap()
    );
    let out = run_stdin(&["hook"], &payload);
    assert_eq!(code(&out), 0);
    assert!(String::from_utf8(out.stdout).unwrap().is_empty());
}

#[test]
fn hook_missing_file_exits_zero() {
    let payload = "{\"tool_input\":{\"file_path\":\"/no/such/file.rs\"}}";
    let out = run_stdin(&["hook"], payload);
    assert_eq!(code(&out), 0);
}

#[test]
fn advice_prints_default() {
    let out = run(&["advice"]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("SessionStart"));
    assert!(stdout.contains("TRIPWIRE:"));
}

#[test]
fn advice_honors_inline_override() {
    let dir = tempdir();
    write(
        &dir.join("comment-warden.toml"),
        "advice = \"custom advice sentinel\"\n",
    );
    let out = run(&[
        "advice",
        "--config",
        dir.join("comment-warden.toml").to_str().unwrap(),
    ]);
    let c = code(&out);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(c, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("custom advice sentinel"),
        "stdout: {stdout} / stderr: {stderr}"
    );
}

#[test]
fn advice_honors_advice_file_override() {
    let dir = tempdir();
    write(&dir.join("policy.md"), "advice from a file sentinel\n");
    write(
        &dir.join("comment-warden.toml"),
        "advice_file = \"policy.md\"\n",
    );
    let out = run(&[
        "advice",
        "--config",
        dir.join("comment-warden.toml").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 0);
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("advice from a file sentinel")
    );
}

#[test]
fn check_skips_build_output_directories() {
    let dir = tempdir();
    let build = dir.join("DerivedData");
    std::fs::create_dir_all(&build).unwrap();
    write(
        &build.join("generated.rs"),
        "// untagged generated\nfn g() {}\n",
    );
    write(&dir.join("real.rs"), "// untagged real\nfn r() {}\n");

    let out = run(&["check", dir.to_str().unwrap()]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("real.rs"), "real file scanned: {stdout}");
    assert!(
        !stdout.contains("generated.rs"),
        "build-output file must be skipped: {stdout}"
    );
    let untagged_lines = stdout.lines().filter(|l| l.contains("untagged")).count();
    assert_eq!(untagged_lines, 1, "only the real file reports: {stdout}");
}

#[test]
fn strip_does_not_follow_symlink_to_outside_file() {
    let dir = tempdir();
    let outside = tempdir().join("outside.rs");
    write(&outside, "// nothing\nfn main() {}\n");

    let tree = dir.join("tree");
    std::fs::create_dir_all(&tree).unwrap();
    let link = tree.join("linked.rs");
    std::os::unix::fs::symlink(&outside, &link).unwrap();

    let out = run(&["strip", tree.to_str().unwrap()]);
    assert_eq!(code(&out), 2, "symlink should be skipped, nothing scanned");
    let after = std::fs::read_to_string(&outside).unwrap();
    assert_eq!(after, "// nothing\nfn main() {}\n");
}

#[test]
fn strip_does_not_follow_a_symlinked_file_passed_as_the_root() {
    let dir = tempdir();
    let outside = tempdir().join("outside.rs");
    write(&outside, "// nothing\nfn main() {}\n");
    let link = dir.join("linked.rs");
    std::os::unix::fs::symlink(&outside, &link).unwrap();

    let out = run(&["strip", link.to_str().unwrap()]);
    assert_eq!(code(&out), 2, "a symlink root is skipped, nothing scanned");
    let after = std::fs::read_to_string(&outside).unwrap();
    assert_eq!(after, "// nothing\nfn main() {}\n");
}

#[test]
fn strip_does_not_descend_into_a_symlinked_directory() {
    let dir = tempdir();
    let outside_dir = tempdir();
    let outside = outside_dir.join("outside.rs");
    write(&outside, "// nothing\nfn main() {}\n");

    let tree = dir.join("tree");
    std::fs::create_dir_all(&tree).unwrap();
    std::os::unix::fs::symlink(&outside_dir, tree.join("linked")).unwrap();

    let out = run(&["strip", tree.to_str().unwrap()]);
    assert_eq!(code(&out), 2, "symlinked dir is not descended");
    let after = std::fs::read_to_string(&outside).unwrap();
    assert_eq!(after, "// nothing\nfn main() {}\n");
}

#[test]
fn check_skips_a_gitignored_directory() {
    let dir = tempdir();
    write(&dir.join(".gitignore"), "node_modules/\n.terraform/\n");

    for vendored in ["node_modules", ".terraform"] {
        let sub = dir.join(vendored);
        std::fs::create_dir_all(&sub).unwrap();
        write(
            &sub.join("vendored.rs"),
            "// untagged vendored\nfn v() {}\n",
        );
    }
    write(&dir.join("real.rs"), "// untagged real\nfn r() {}\n");

    let out = run(&["check", dir.to_str().unwrap()]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("real.rs"), "real file scanned: {stdout}");
    assert!(
        !stdout.contains("vendored.rs"),
        "gitignored file must be skipped: {stdout}"
    );
}

#[test]
fn check_skips_a_gitignored_file_by_name() {
    let dir = tempdir();
    write(&dir.join(".gitignore"), "generated.rs\n");
    write(
        &dir.join("generated.rs"),
        "// untagged generated\nfn g() {}\n",
    );
    write(&dir.join("real.rs"), "// untagged real\nfn r() {}\n");

    let out = run(&["check", dir.to_str().unwrap()]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("real.rs"), "real file scanned: {stdout}");
    assert!(
        !stdout.contains("generated.rs"),
        "gitignored file must be skipped: {stdout}"
    );
}

#[test]
fn check_scans_a_hidden_directory_passed_as_the_root() {
    let dir = tempdir();
    let hooks = dir.join(".hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    write(&hooks.join("gate.sh"), "# untagged hook prose\necho hi\n");

    let out = run(&["check", hooks.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "a hidden root is still scanned");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("gate.sh"), "hidden root scanned: {stdout}");
}

#[test]
fn check_scans_a_hidden_directory_reached_while_walking() {
    let dir = tempdir();
    let hooks = dir.join(".hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    write(&hooks.join("gate.sh"), "# untagged hook prose\necho hi\n");

    let out = run(&["check", dir.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "a hidden descendant is still scanned");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("gate.sh"),
        "hidden descendant scanned: {stdout}"
    );
}

#[test]
fn check_exclude_from_skips_an_excluded_directory_under_a_scanned_root() {
    let dir = tempdir();
    let skipped = dir.join("skipped");
    std::fs::create_dir_all(&skipped).unwrap();
    write(&skipped.join("b.rs"), "// untagged excluded\nfn b() {}\n");
    write(&dir.join("a.rs"), "// untagged real\nfn a() {}\n");

    let excludes = dir.join("excludes.txt");
    write(&excludes, &format!("{}\n", skipped.to_str().unwrap()));

    let out = run(&[
        "check",
        "--exclude-from",
        excludes.to_str().unwrap(),
        dir.to_str().unwrap(),
    ]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("a.rs"), "real file scanned: {stdout}");
    assert!(
        !stdout.contains("b.rs"),
        "excluded dir must be skipped: {stdout}"
    );
}

#[test]
fn exempt_patterns_from_config_are_treated_as_directives() {
    let dir = tempdir();
    write(
        &dir.join("comment-warden.toml"),
        "exempt_patterns = [\"my-bundler-pragma:\"]\n",
    );
    write(
        &dir.join("a.ts"),
        "/* my-bundler-pragma: keep */\nconst x = 1;\n",
    );

    let config = dir.join("comment-warden.toml");
    let file = dir.join("a.ts");
    let out = run(&[
        "check",
        "--config",
        config.to_str().unwrap(),
        file.to_str().unwrap(),
    ]);
    let c = code(&out);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(c, 0, "stdout: {stdout}");
}

#[test]
fn an_unconfigured_pragma_is_still_flagged() {
    let dir = tempdir();
    write(
        &dir.join("a.ts"),
        "/* my-bundler-pragma: keep */\nconst x = 1;\n",
    );
    let out = run(&["check", dir.join("a.ts").to_str().unwrap()]);
    assert_eq!(code(&out), 1);
}

#[test]
fn exempt_paths_skip_a_whole_directory() {
    let dir = tempdir();
    write(
        &dir.join("comment-warden.toml"),
        "exempt_paths = [\"db/migrate/**\", \"vendor/**\"]\n",
    );
    let migrate = dir.join("db/migrate");
    std::fs::create_dir_all(&migrate).unwrap();
    std::fs::create_dir_all(dir.join("vendor/pkg")).unwrap();
    write(&migrate.join("001_create.rb"), "# adds the table\nx = 1\n");
    write(
        &dir.join("vendor/pkg/gen.rs"),
        "// vendor-authored note\nfn v() {}\n",
    );
    write(&dir.join("app.rb"), "# real decoration\ny = 2\n");

    let out = run_in(
        dir.as_path(),
        &["check", "--config", "comment-warden.toml", "."],
    );
    let c = code(&out);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(c, 1, "stdout: {stdout}");
    assert!(stdout.contains("app.rb"), "stdout: {stdout}");
    assert!(
        !stdout.contains("001_create.rb"),
        "exempt_paths must skip migrations: {stdout}"
    );
    assert!(
        !stdout.contains("gen.rs"),
        "exempt_paths must skip vendor: {stdout}"
    );
}

#[test]
fn strip_leaves_an_exempt_path_untouched() {
    let dir = tempdir();
    write(
        &dir.join("comment-warden.toml"),
        "exempt_paths = [\"db/migrate/**\"]\n",
    );
    let migrate = dir.join("db/migrate");
    std::fs::create_dir_all(&migrate).unwrap();
    let migration = migrate.join("001_create.rb");
    let original = "# adds the table\nx = 1\n";
    write(&migration, original);

    let out = run_in(
        dir.as_path(),
        &["strip", "--config", "comment-warden.toml", "."],
    );
    assert_eq!(code(&out), 0);
    assert_eq!(std::fs::read_to_string(&migration).unwrap(), original);
}

#[test]
fn a_bad_exempt_paths_glob_exits_three() {
    let dir = tempdir();
    write(
        &dir.join("comment-warden.toml"),
        "exempt_paths = [\"a{\"]\n",
    );
    write(&dir.join("a.rs"), "fn main() {}\n");
    let config = dir.join("comment-warden.toml");
    let file = dir.join("a.rs");
    let out = run(&[
        "check",
        "--config",
        config.to_str().unwrap(),
        file.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 3);
}
