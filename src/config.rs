use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use crate::lang::LangId;

pub const DEFAULT_ADVICE: &str = "comment-warden is active.\n\nWrite a comment only when the code cannot say it itself. First reach for a clearer name, a type that makes the bad state impossible, or a test that fails loudly. Default to none. This applies to `///` doc comments too — an untagged `///` is deleted like any other comment, so don't add them.\n\nOnly two tags survive; everything else is deleted automatically after each write:\n- TRIPWIRE: name the future edit it prevents — e.g. `// TRIPWIRE: delete this sleep and retries hammer the server`.\n- CONTEXT: record a fact outside the code that no name or type can hold — an upstream bug, a wire-format quirk.\n\nUntagged comments vanish silently. Don't re-add them, and don't puzzle over a file that comes back without one.";

const CONFIG_FILENAME: &str = "comment-warden.toml";

const DEFAULT_TAGS: &[&str] = &["TRIPWIRE", "CONTEXT"];

fn default_tags() -> Vec<String> {
    DEFAULT_TAGS.iter().map(|s| s.to_string()).collect()
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    advice: Option<String>,
    advice_file: Option<PathBuf>,
    tags: Option<Vec<String>>,
    #[serde(default)]
    exempt_patterns: Vec<String>,
    #[serde(default)]
    exempt_paths: Vec<String>,
    #[serde(default, rename = "doc_surface")]
    doc_surface: Vec<RawDocSurface>,
}

#[derive(Debug, Deserialize)]
struct RawDocSurface {
    lang: String,
    #[serde(default)]
    item_attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DocSurfaceRule {
    pub lang: LangId,
    pub item_attributes: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Config {
    advice: Option<String>,
    advice_file: Option<PathBuf>,
    config_dir: Option<PathBuf>,
    tags: Vec<String>,
    exempt_patterns: Vec<String>,
    exempt_paths: Option<GlobSet>,
    pub doc_surface: Vec<DocSurfaceRule>,
}

impl Config {
    pub fn empty() -> Self {
        Self {
            tags: default_tags(),
            ..Self::default()
        }
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn matches_exempt_pattern(&self, body: &str) -> bool {
        self.exempt_patterns
            .iter()
            .any(|p| body.starts_with(p.as_str()))
    }

    pub fn path_is_exempt(&self, path: &Path) -> bool {
        let Some(globs) = &self.exempt_paths else {
            return false;
        };
        candidate_paths(path).iter().any(|c| globs.is_match(c))
    }

    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        let path = match explicit {
            Some(p) => Some(p.to_path_buf()),
            None => discover_upward(&std::env::current_dir()?),
        };
        let Some(path) = path else {
            return Ok(Self::empty());
        };
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let raw: RawConfig =
            toml::from_str(&text).with_context(|| format!("parsing config {}", path.display()))?;
        let mut doc_surface = Vec::new();
        for rule in raw.doc_surface {
            let Some(lang) = LangId::from_config_name(&rule.lang) else {
                anyhow::bail!(
                    "config {}: unknown doc_surface lang {:?}",
                    path.display(),
                    rule.lang
                );
            };
            doc_surface.push(DocSurfaceRule {
                lang,
                item_attributes: rule.item_attributes,
            });
        }
        Ok(Self {
            advice: raw.advice,
            advice_file: raw.advice_file,
            config_dir: path.parent().map(Path::to_path_buf),
            tags: raw.tags.unwrap_or_else(default_tags),
            exempt_patterns: raw.exempt_patterns,
            exempt_paths: build_globs(&raw.exempt_paths, &path)?,
            doc_surface,
        })
    }

    pub fn doc_surface_for(&self, lang: LangId) -> Vec<&DocSurfaceRule> {
        self.doc_surface.iter().filter(|r| r.lang == lang).collect()
    }

    pub fn advice(&self) -> Result<String> {
        if let Some(inline) = &self.advice {
            return Ok(inline.clone());
        }
        if let Some(rel) = &self.advice_file {
            let path = match &self.config_dir {
                Some(dir) => dir.join(rel),
                None => rel.clone(),
            };
            return std::fs::read_to_string(&path)
                .with_context(|| format!("reading advice_file {}", path.display()));
        }
        Ok(DEFAULT_ADVICE.to_string())
    }
}

fn build_globs(patterns: &[String], config_path: &Path) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).with_context(|| {
            format!(
                "config {}: bad exempt_paths glob {:?}",
                config_path.display(),
                pattern
            )
        })?;
        builder.add(glob);
    }
    Ok(Some(builder.build()?))
}

fn candidate_paths(path: &Path) -> Vec<PathBuf> {
    // TRIPWIRE: match the cwd-relative form too — a glob is written relative to the repo root, but a walk rooted at an absolute path yields absolute entries that no such glob can match.
    let mut candidates = vec![path.to_path_buf()];
    if let Ok(stripped) = path.strip_prefix("./") {
        candidates.push(stripped.to_path_buf());
    }
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(relative) = path.strip_prefix(&cwd)
    {
        candidates.push(relative.to_path_buf());
    }
    candidates
}

fn discover_upward(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = d.join(CONFIG_FILENAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = d.parent();
    }
    None
}
