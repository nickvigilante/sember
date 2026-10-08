//! `.sember.toml`: per-repository settings.
//!
//! ```toml
//! style = "sembr"            # "sentence" (default) or "sembr"
//! width = 80                 # sembr: clause breaks only on longer lines
//! existing-breaks = "auto"   # sembr: "auto", "keep" or "reflow"
//!
//! [[override]]               # later overrides win
//! paths = ["docs/legacy/**"] # globs, relative to this file's directory
//! existing-breaks = "reflow"
//! ```
//!
//! For each Markdown file, sember uses the nearest `.sember.toml` in the
//! file's directory or a parent, stopping at the repository root (the
//! directory holding `.git`).

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use crate::{Config, ExistingBreaks, Style};

/// The config file's name.
pub const FILE_NAME: &str = ".sember.toml";

/// Settings that may each be left unset, so that a later layer (an
/// override, a command-line flag) only replaces what it names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Partial {
    pub style: Option<StyleName>,
    pub width: Option<usize>,
    pub existing_breaks: Option<ExistingName>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StyleName {
    Sentence,
    Sembr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExistingName {
    Auto,
    Keep,
    Reflow,
}

impl Partial {
    /// Applies the settings this layer sets on top of `config`.
    pub fn apply(&self, config: &mut Config) {
        if let Some(style) = self.style {
            config.style = match style {
                StyleName::Sentence => Style::Sentence,
                StyleName::Sembr => Style::Sembr,
            };
        }
        if let Some(width) = self.width {
            config.width = width;
        }
        if let Some(existing) = self.existing_breaks {
            config.existing_breaks = match existing {
                ExistingName::Auto => ExistingBreaks::Auto,
                ExistingName::Keep => ExistingBreaks::Keep,
                ExistingName::Reflow => ExistingBreaks::Reflow,
            };
        }
    }
}

// serde can't combine `flatten` with `deny_unknown_fields`, so the
// settings are spelled out here rather than embedding `Partial`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawFile {
    style: Option<StyleName>,
    width: Option<usize>,
    existing_breaks: Option<ExistingName>,
    #[serde(default, rename = "override")]
    overrides: Vec<RawOverride>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawOverride {
    paths: Vec<String>,
    style: Option<StyleName>,
    width: Option<usize>,
    existing_breaks: Option<ExistingName>,
}

/// A parsed `.sember.toml`.
#[derive(Debug)]
pub struct ConfigFile {
    /// The directory override globs are relative to.
    root: PathBuf,
    base: Partial,
    overrides: Vec<(GlobSet, Partial)>,
}

/// A config file that couldn't be read or parsed.
#[derive(Debug)]
pub struct Error {
    path: PathBuf,
    message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for Error {}

impl ConfigFile {
    /// Reads and parses a config file. Unknown keys are errors, so a typo
    /// can't silently fall back to the defaults.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let error = |message: String| Error {
            path: path.to_owned(),
            message,
        };
        let text = fs::read_to_string(path).map_err(|e| error(e.to_string()))?;
        Self::parse(&text, path.parent().unwrap_or(Path::new("."))).map_err(error)
    }

    fn parse(text: &str, root: &Path) -> Result<Self, String> {
        let raw: RawFile = toml::from_str(text).map_err(|e| e.message().to_owned())?;
        let mut overrides = Vec::new();
        for (i, raw_override) in raw.overrides.into_iter().enumerate() {
            if raw_override.paths.is_empty() {
                return Err(format!("override {}: `paths` is empty", i + 1));
            }
            let mut builder = GlobSetBuilder::new();
            for pattern in &raw_override.paths {
                let glob = Glob::new(pattern).map_err(|e| format!("override {}: {e}", i + 1))?;
                builder.add(glob);
            }
            let set = builder
                .build()
                .map_err(|e| format!("override {}: {e}", i + 1))?;
            overrides.push((
                set,
                Partial {
                    style: raw_override.style,
                    width: raw_override.width,
                    existing_breaks: raw_override.existing_breaks,
                },
            ));
        }
        Ok(Self {
            root: root.to_owned(),
            base: Partial {
                style: raw.style,
                width: raw.width,
                existing_breaks: raw.existing_breaks,
            },
            overrides,
        })
    }

    /// Applies this file's settings for `path`: the top-level settings,
    /// then every matching override in order.
    pub fn apply(&self, path: &Path, config: &mut Config) {
        self.base.apply(config);
        let relative = path.strip_prefix(&self.root).unwrap_or(path);
        for (globs, settings) in &self.overrides {
            if globs.is_match(relative) {
                settings.apply(config);
            }
        }
    }
}

/// Finds and caches the nearest config file for each directory.
#[derive(Debug, Default)]
pub struct Discovery {
    by_dir: HashMap<PathBuf, Option<Rc<ConfigFile>>>,
}

impl Discovery {
    /// The nearest `.sember.toml` at or above `dir`, stopping after the
    /// directory that contains `.git`.
    pub fn find(&mut self, dir: &Path) -> Result<Option<Rc<ConfigFile>>, Error> {
        if let Some(found) = self.by_dir.get(dir) {
            return Ok(found.clone());
        }
        let candidate = dir.join(FILE_NAME);
        let found = if candidate.is_file() {
            Some(Rc::new(ConfigFile::load(&candidate)?))
        } else if dir.join(".git").exists() {
            None
        } else {
            match dir.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => self.find(parent)?,
                _ => None,
            }
        };
        self.by_dir.insert(dir.to_owned(), found.clone());
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::ConfigFile;
    use crate::{Config, ExistingBreaks, Style};

    fn config_for(text: &str, path: &str) -> Config {
        let file = ConfigFile::parse(text, Path::new("/repo")).unwrap();
        let mut config = Config::default();
        file.apply(Path::new(path), &mut config);
        config
    }

    #[test]
    fn top_level_settings() {
        let config = config_for(
            "style = \"sembr\"\nwidth = 80\nexisting-breaks = \"keep\"\n",
            "/repo/README.md",
        );
        assert_eq!(config.style, Style::Sembr);
        assert_eq!(config.width, 80);
        assert_eq!(config.existing_breaks, ExistingBreaks::Keep);
    }

    #[test]
    fn overrides_apply_in_order_to_matching_paths() {
        let text = r#"
style = "sembr"

[[override]]
paths = ["docs/legacy/**"]
existing-breaks = "reflow"

[[override]]
paths = ["docs/legacy/keep-me.md"]
existing-breaks = "keep"
"#;
        let legacy = config_for(text, "/repo/docs/legacy/a/b.md");
        assert_eq!(legacy.existing_breaks, ExistingBreaks::Reflow);
        assert_eq!(legacy.style, Style::Sembr);

        let kept = config_for(text, "/repo/docs/legacy/keep-me.md");
        assert_eq!(kept.existing_breaks, ExistingBreaks::Keep);

        let other = config_for(text, "/repo/docs/guide.md");
        assert_eq!(other.existing_breaks, ExistingBreaks::Auto);
    }

    #[test]
    fn rejects_unknown_keys_and_values() {
        for text in [
            "stlye = \"sembr\"\n",
            "style = \"clauses\"\n",
            "[[override]]\npaths = [\"a\"]\nwidht = 3\n",
            "[[override]]\npaths = []\n",
            "[[override]]\npaths = [\"a[\"]\n",
        ] {
            assert!(
                ConfigFile::parse(text, Path::new("/repo")).is_err(),
                "{text:?}"
            );
        }
    }
}
