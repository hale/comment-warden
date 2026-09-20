use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
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
