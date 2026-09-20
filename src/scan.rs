use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

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
        gather_files(p, excludes, &mut files)?;
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

fn gather_files(path: &Path, excludes: &HashSet<PathBuf>, out: &mut Vec<PathBuf>) -> Result<()> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if excludes.contains(&canonical) || excludes.contains(path) {
        return Ok(());
    }
    let meta =
        std::fs::symlink_metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if meta.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    if meta.is_dir() {
        if is_ignored_dir(path) {
            return Ok(());
        }
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
            .with_context(|| format!("reading dir {}", path.display()))?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::result::Result<_, _>>()?;
        entries.sort();
        for entry in entries {
            gather_files(&entry, excludes, out)?;
        }
    }
    Ok(())
}

fn is_ignored_dir(path: &Path) -> bool {
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
