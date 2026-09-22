use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::WalkBuilder;

use crate::config::Config;
use crate::lang::LangId;
use crate::strip::{FileReport, process};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Clean,
    Violations,
    NothingScanned,
}

impl Outcome {
    pub fn exit_code(self) -> i32 {
        match self {
            Outcome::Clean => 0,
            Outcome::Violations => 1,
            Outcome::NothingScanned => 2,
        }
    }
}

#[derive(Debug, Default)]
pub struct RunTotals {
    pub scanned: usize,
    pub stripped: usize,
    pub left_in_place: usize,
    pub untagged_found: usize,
    pub unclassified: usize,
    pub reports: Vec<(PathBuf, FileReport)>,
}

impl RunTotals {
    pub fn outcome(&self) -> Outcome {
        if self.scanned == 0 {
            return Outcome::NothingScanned;
        }
        let remaining = self.untagged_found.saturating_sub(self.stripped);
        if remaining > 0 || self.left_in_place > 0 || self.unclassified > 0 {
            Outcome::Violations
        } else {
            Outcome::Clean
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Check,
    Strip,
}

pub fn run(
    paths: &[PathBuf],
    excludes: &HashSet<PathBuf>,
    config: &Config,
    mode: Mode,
) -> Result<RunTotals> {
    let mut files = Vec::new();
    for p in paths {
        gather_files(p, excludes, config, &mut files)?;
    }
    files.sort();
    files.dedup();

    let mut totals = RunTotals::default();
    for file in files {
        let Some(id) = ext_of(&file).and_then(LangId::from_extension) else {
            continue;
        };
        totals.scanned += 1;
        let source = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let is_test = is_test_target(&file);
        let apply = matches!(mode, Mode::Strip);
        let report = process(id, &source, is_test, config, apply)
            .with_context(|| format!("processing {}", file.display()))?;
        if let Some(new_source) = &report.new_source {
            std::fs::write(&file, new_source)
                .with_context(|| format!("writing {}", file.display()))?;
        }
        totals.stripped += report.stripped;
        totals.left_in_place += report.left_in_place;
        totals.untagged_found += report.untagged_found;
        totals.unclassified += report.unclassified;
        totals.reports.push((file, report));
    }
    Ok(totals)
}

pub fn process_one(path: &Path, config: &Config, mode: Mode) -> Result<Option<FileReport>> {
    let Some(id) = ext_of(path).and_then(LangId::from_extension) else {
        return Ok(None);
    };
    if config.path_is_exempt(path) {
        return Ok(None);
    }
    let source =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let is_test = is_test_target(path);
    let apply = matches!(mode, Mode::Strip);
    let report = process(id, &source, is_test, config, apply)?;
    if let Some(new_source) = &report.new_source {
        std::fs::write(path, new_source).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(Some(report))
}

fn gather_files(
    path: &Path,
    excludes: &HashSet<PathBuf>,
    config: &Config,
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    // TRIPWIRE: stat the root before walking — the walker resolves a symlink given as its own root even with follow_links off, so without this a symlinked file or dir is scanned (and stripped) through the link.
    let root_meta =
        std::fs::symlink_metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if root_meta.file_type().is_symlink() {
        return Ok(());
    }

    if config.path_is_exempt(path) {
        return Ok(());
    }

    let excluded = excludes.clone();
    let exempt = config.clone();
    let mut builder = WalkBuilder::new(path);
    builder
        .hidden(false)
        .follow_links(false)
        // TRIPWIRE: require_git(false) keeps a .gitignore authoritative outside a checkout — a vendored cache in an unpacked tarball or a worktree without .git is the case the walk exists to skip.
        .require_git(false)
        .filter_entry(move |entry| {
            let p = entry.path();
            if is_ignored_dir(p) || exempt.path_is_exempt(p) {
                return false;
            }
            let canonical = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
            !excluded.contains(&canonical) && !excluded.contains(p)
        });

    for result in builder.build() {
        let entry = result.with_context(|| format!("walking {}", path.display()))?;
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }
        out.push(entry.into_path());
    }
    Ok(())
}

fn is_ignored_dir(path: &Path) -> bool {
    // TRIPWIRE: keep this denylist as a floor under .gitignore — .git is never self-ignored, and a build dir like target or DerivedData is often absent from a repo's own .gitignore (this repo's included).
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some(".git" | ".build" | ".direnv" | "target" | "node_modules" | "result" | "DerivedData")
    )
}

fn ext_of(path: &Path) -> Option<&str> {
    path.extension().and_then(|e| e.to_str())
}

fn is_test_target(path: &Path) -> bool {
    path.components()
        .any(|c| matches!(c.as_os_str().to_str(), Some("tests" | "test" | "__tests__")))
        || path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.ends_with("_test") || s.ends_with(".test") || s.ends_with("_tests"))
}

pub fn read_exclude_from(path: &Path) -> Result<HashSet<PathBuf>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading exclude file {}", path.display()))?;
    let mut set = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let p = PathBuf::from(line);
        if !p.exists() {
            bail!("--exclude-from entry {line} does not exist — the exclusion list went stale");
        }
        set.insert(p.canonicalize().unwrap_or(p));
    }
    Ok(set)
}
