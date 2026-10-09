//! `.sember.toml`: per-repository settings, laid out like Vale's `.vale.ini`.
//!
//! ```toml
//! # Top level: project settings, and formatting defaults for every file.
//! style = "sentence"
//! exclude = ["CHANGELOG.md", "vendor/**"]
//!
//! # Treat other extensions as a format sember knows.
//! [formats]
//! qmd = "md"
//! mdx = "md"
//!
//! # Sections named by glob. Every matching section applies, in file order,
//! # so later sections win.
//! ["docs/**"]
//! style = "sembr"
//! width = 80
//!
//! ["docs/legacy/**"]
//! existing-breaks = "reflow"
//! ```
//!
//! A glob containing `/` matches the path relative to the config file's
//! directory, where `*` stays within one directory and `**` crosses them. A
//! glob without `/` matches the file name at any depth, as in `.gitignore`.
//!
//! For each file, sember uses the nearest `.sember.toml` in the file's
//! directory or a parent, stopping at the repository root (the directory
//! holding `.git`).

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use globset::{GlobBuilder, GlobMatcher};
use serde::Deserialize;
use toml::{Table, Value};

use crate::{ClausePunctuation, Config, ExistingBreaks, LeadIn, Level, Style};

/// The config file's name.
pub const FILE_NAME: &str = ".sember.toml";

/// A document format sember can format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Markdown,
}

impl Format {
    /// The format of a recognized extension (without the dot).
    fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "md" | "markdown" => Some(Format::Markdown),
            _ => None,
        }
    }
}

/// The format of `path` from its built-in extension, with no config.
pub fn builtin_format(path: &Path) -> Option<Format> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(Format::from_name)
}

/// Formatting settings that may each be left unset, so that a later layer (a
/// glob section, a command-line flag) only replaces what it names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Partial {
    pub style: Option<StyleName>,
    pub width: Option<usize>,
    pub existing_breaks: Option<ExistingName>,
    pub lead_in: Option<LeadInName>,
    pub sembr: SembrPartial,
}

/// The `[sembr]` table: SemBr rule levels, each optional.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SembrPartial {
    pub independent_clauses: Option<Level>,
    pub clause_punctuation: Option<ClausePunctuation>,
    pub dependent_clauses: Option<Level>,
    pub before_lists: Option<Level>,
    pub list_items: Option<Level>,
    pub links: Option<Level>,
    pub inline_markup: Option<Level>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LevelName {
    Always,
    Width,
    Never,
}

impl From<LevelName> for Level {
    fn from(name: LevelName) -> Self {
        match name {
            LevelName::Always => Level::Always,
            LevelName::Width => Level::Width,
            LevelName::Never => Level::Never,
        }
    }
}

impl SembrPartial {
    const KEYS: &str = "`independent-clauses`, `clause-punctuation`, `dependent-clauses`, `before-lists`, `list-items`, `links` or `inline-markup`";

    fn parse(table: &Table, context: &str) -> Result<Self, String> {
        let mut rules = SembrPartial::default();
        for (key, value) in table {
            let level = || -> Result<Level, String> {
                let name: LevelName = value
                    .clone()
                    .try_into()
                    .map_err(|e: toml::de::Error| format!("{context} `{key}`: {}", e.message()))?;
                Ok(name.into())
            };
            match key.as_str() {
                "independent-clauses" => rules.independent_clauses = Some(level()?),
                "dependent-clauses" => rules.dependent_clauses = Some(level()?),
                "before-lists" => rules.before_lists = Some(level()?),
                "list-items" => rules.list_items = Some(level()?),
                "links" => rules.links = Some(level()?),
                "inline-markup" => rules.inline_markup = Some(level()?),
                "clause-punctuation" => {
                    let bad = || {
                        format!(
                            "{context} `clause-punctuation`: expected a list drawn from \",\", \";\", \":\", \"—\" and \"–\""
                        )
                    };
                    let Value::Array(marks) = value else {
                        return Err(bad());
                    };
                    let mut set = ClausePunctuation {
                        comma: false,
                        semicolon: false,
                        colon: false,
                        em_dash: false,
                        en_dash: false,
                    };
                    for mark in marks {
                        match mark.as_str() {
                            Some(",") => set.comma = true,
                            Some(";") => set.semicolon = true,
                            Some(":") => set.colon = true,
                            Some("—") => set.em_dash = true,
                            Some("–") => set.en_dash = true,
                            _ => return Err(bad()),
                        }
                    }
                    rules.clause_punctuation = Some(set);
                }
                _ => {
                    return Err(format!(
                        "{context}: unknown rule `{key}`; expected {}",
                        Self::KEYS
                    ));
                }
            }
        }
        Ok(rules)
    }

    fn apply(&self, config: &mut Config) {
        let rules = &mut config.sembr;
        let set = |target: &mut Level, value: Option<Level>| {
            if let Some(value) = value {
                *target = value;
            }
        };
        set(&mut rules.independent_clauses, self.independent_clauses);
        set(&mut rules.dependent_clauses, self.dependent_clauses);
        set(&mut rules.before_lists, self.before_lists);
        set(&mut rules.list_items, self.list_items);
        set(&mut rules.links, self.links);
        set(&mut rules.inline_markup, self.inline_markup);
        if let Some(punctuation) = self.clause_punctuation {
            rules.clause_punctuation = punctuation;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StyleName {
    Sentence,
    Sembr,
    Paragraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExistingName {
    Auto,
    Keep,
    Reflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LeadInName {
    OwnLine,
    SameLine,
}

impl Partial {
    /// Applies the settings this layer sets on top of `config`.
    pub fn apply(&self, config: &mut Config) {
        if let Some(style) = self.style {
            config.style = match style {
                StyleName::Sentence => Style::Sentence,
                StyleName::Sembr => Style::Sembr,
                StyleName::Paragraph => Style::Paragraph,
            };
        }
        self.sembr.apply(config);
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
        if let Some(lead_in) = self.lead_in {
            config.lead_in = match lead_in {
                LeadInName::OwnLine => LeadIn::OwnLine,
                LeadInName::SameLine => LeadIn::SameLine,
            };
        }
    }

    /// Reads a formatting setting, returning false if `key` isn't one.
    fn set(&mut self, key: &str, value: &Value) -> Result<bool, String> {
        fn typed<T: for<'de> Deserialize<'de>>(key: &str, value: &Value) -> Result<T, String> {
            value
                .clone()
                .try_into()
                .map_err(|e: toml::de::Error| format!("`{key}`: {}", e.message()))
        }
        match key {
            "style" => self.style = Some(typed(key, value)?),
            "width" => self.width = Some(typed(key, value)?),
            "existing-breaks" => self.existing_breaks = Some(typed(key, value)?),
            "lead-in" => self.lead_in = Some(typed(key, value)?),
            "sembr" => {
                let Value::Table(table) = value else {
                    return Err("`sembr`: expected a table of rules".to_owned());
                };
                self.sembr = SembrPartial::parse(table, "[sembr]")?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// A glob, matched like a `.gitignore` pattern.
#[derive(Debug)]
struct Pattern {
    matcher: GlobMatcher,
    /// No `/` in the pattern: match the file name, not the path.
    by_name: bool,
}

impl Pattern {
    fn new(pattern: &str) -> Result<Self, String> {
        let by_name = !pattern.contains('/');
        let glob = GlobBuilder::new(pattern.trim_start_matches('/'))
            .literal_separator(true)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            matcher: glob.compile_matcher(),
            by_name,
        })
    }

    fn matches(&self, relative: &Path) -> bool {
        if self.by_name {
            relative
                .file_name()
                .is_some_and(|name| self.matcher.is_match(name))
        } else {
            self.matcher.is_match(relative)
        }
    }
}

/// A parsed `.sember.toml`.
#[derive(Debug)]
pub struct ConfigFile {
    /// The directory globs are relative to.
    root: PathBuf,
    base: Partial,
    sections: Vec<(Pattern, Partial)>,
    exclude: Vec<Pattern>,
    formats: HashMap<String, Format>,
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

/// A table name that isn't a glob or path is far more likely a misspelt
/// `[formats]` than a section for a file literally named that.
fn looks_like_glob(name: &str) -> bool {
    name.contains(['/', '.', '*', '?', '[', '{'])
}

impl ConfigFile {
    /// Reads and parses a config file. Unknown keys, values and sections
    /// are errors, so a typo can't silently fall back to the defaults.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let error = |message: String| Error {
            path: path.to_owned(),
            message,
        };
        let text = fs::read_to_string(path).map_err(|e| error(e.to_string()))?;
        Self::parse(&text, path.parent().unwrap_or(Path::new("."))).map_err(error)
    }

    fn parse(text: &str, root: &Path) -> Result<Self, String> {
        let table: Table = text
            .parse()
            .map_err(|e: toml::de::Error| e.message().to_owned())?;
        let mut file = ConfigFile {
            root: root.to_owned(),
            base: Partial::default(),
            sections: Vec::new(),
            exclude: Vec::new(),
            formats: HashMap::new(),
        };

        for (key, value) in &table {
            if file.base.set(key, value)? {
                continue;
            }
            match (key.as_str(), value) {
                ("exclude", Value::Array(patterns)) => {
                    for pattern in patterns {
                        let Value::String(pattern) = pattern else {
                            return Err("`exclude`: expected a list of globs".to_owned());
                        };
                        let pattern =
                            Pattern::new(pattern).map_err(|e| format!("`exclude`: {e}"))?;
                        file.exclude.push(pattern);
                    }
                }
                ("exclude", _) => return Err("`exclude`: expected a list of globs".to_owned()),
                ("formats", Value::Table(formats)) => {
                    for (ext, target) in formats {
                        let format =
                            target.as_str().and_then(Format::from_name).ok_or_else(|| {
                                format!("[formats] `{ext}`: expected \"md\" or \"markdown\"")
                            })?;
                        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
                        file.formats.insert(ext, format);
                    }
                }
                ("formats", _) => return Err("`formats`: expected a table".to_owned()),
                (name, Value::Table(section)) => {
                    if !looks_like_glob(name) {
                        return Err(format!(
                            "unknown section [{name}]; sections are named by a glob such as [\"docs/**\"] or [\"*.mdx\"]"
                        ));
                    }
                    let pattern = Pattern::new(name).map_err(|e| format!("[\"{name}\"]: {e}"))?;
                    let mut settings = Partial::default();
                    for (key, value) in section {
                        let known = settings
                            .set(key, value)
                            .map_err(|e| format!("[\"{name}\"] {e}"))?;
                        if !known {
                            return Err(format!(
                                "[\"{name}\"]: unknown key `{key}`; expected `style`, `width`, `existing-breaks`, `lead-in` or `sembr`"
                            ));
                        }
                    }
                    file.sections.push((pattern, settings));
                }
                (key, _) => {
                    return Err(format!(
                        "unknown key `{key}`; expected `style`, `width`, `existing-breaks`, `lead-in`, `sembr`, `exclude`, `formats` or a glob section"
                    ));
                }
            }
        }
        Ok(file)
    }

    fn relative<'a>(&self, path: &'a Path) -> &'a Path {
        path.strip_prefix(&self.root).unwrap_or(path)
    }

    /// Applies this file's settings for `path`: the top-level settings,
    /// then every matching section in order.
    pub fn apply(&self, path: &Path, config: &mut Config) {
        self.base.apply(config);
        let relative = self.relative(path);
        for (pattern, settings) in &self.sections {
            if pattern.matches(relative) {
                settings.apply(config);
            }
        }
    }

    /// Whether `path` is excluded from formatting.
    pub fn is_excluded(&self, path: &Path) -> bool {
        let relative = self.relative(path);
        self.exclude.iter().any(|pattern| pattern.matches(relative))
    }

    /// The format of `path`, from `[formats]` first and then the built-in
    /// extensions.
    pub fn format_of(&self, path: &Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        self.formats
            .get(&ext)
            .copied()
            .or_else(|| Format::from_name(&ext))
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

    use super::{ConfigFile, Format};
    use crate::{Config, ExistingBreaks, LeadIn, Level, Style};

    fn parse(text: &str) -> ConfigFile {
        ConfigFile::parse(text, Path::new("/repo")).unwrap()
    }

    fn config_for(file: &ConfigFile, path: &str) -> Config {
        let mut config = Config::default();
        file.apply(Path::new(path), &mut config);
        config
    }

    #[test]
    fn top_level_settings() {
        let file = parse("style = \"sembr\"\nwidth = 80\nexisting-breaks = \"keep\"\n");
        let config = config_for(&file, "/repo/README.md");
        assert_eq!(config.style, Style::Sembr);
        assert_eq!(config.width, 80);
        assert_eq!(config.existing_breaks, ExistingBreaks::Keep);
    }

    #[test]
    fn lead_in_setting() {
        let file = parse("lead-in = \"same-line\"\n[\"docs/**\"]\nlead-in = \"own-line\"\n");
        assert_eq!(config_for(&file, "/repo/a.md").lead_in, LeadIn::SameLine);
        assert_eq!(
            config_for(&file, "/repo/docs/a.md").lead_in,
            LeadIn::OwnLine
        );
        assert_eq!(Config::default().lead_in, LeadIn::OwnLine);
        assert!(ConfigFile::parse("lead-in = \"below\"\n", Path::new("/repo")).is_err());
    }

    #[test]
    fn sections_apply_in_order() {
        let file = parse(
            r#"
style = "sembr"

["docs/legacy/**"]
existing-breaks = "reflow"

["docs/legacy/keep-me.md"]
existing-breaks = "keep"
"#,
        );
        let legacy = config_for(&file, "/repo/docs/legacy/a/b.md");
        assert_eq!(legacy.existing_breaks, ExistingBreaks::Reflow);
        assert_eq!(legacy.style, Style::Sembr);
        let kept = config_for(&file, "/repo/docs/legacy/keep-me.md");
        assert_eq!(kept.existing_breaks, ExistingBreaks::Keep);
        let other = config_for(&file, "/repo/docs/guide.md");
        assert_eq!(other.existing_breaks, ExistingBreaks::Auto);
    }

    #[test]
    fn globs_match_like_gitignore() {
        let file = parse("[\"*.mdx\"]\nstyle = \"sembr\"\n\n[\"docs/*.md\"]\nwidth = 70\n");
        // No slash: the file name, at any depth.
        assert_eq!(config_for(&file, "/repo/a/b/c.mdx").style, Style::Sembr);
        // With a slash: the relative path, where `*` stays in one directory.
        assert_eq!(config_for(&file, "/repo/docs/a.md").width, 70);
        assert_eq!(config_for(&file, "/repo/docs/sub/a.md").width, 80);
    }

    #[test]
    fn formats_and_exclude() {
        let file = parse(
            "exclude = [\"CHANGELOG.md\", \"vendor/**\"]\n\n[formats]\nqmd = \"md\"\n\".MDX\" = \"markdown\"\n",
        );
        assert_eq!(file.format_of(Path::new("a.qmd")), Some(Format::Markdown));
        assert_eq!(file.format_of(Path::new("a.mdx")), Some(Format::Markdown));
        assert_eq!(file.format_of(Path::new("a.MD")), Some(Format::Markdown));
        assert_eq!(file.format_of(Path::new("a.txt")), None);
        assert!(file.is_excluded(Path::new("/repo/docs/CHANGELOG.md")));
        assert!(file.is_excluded(Path::new("/repo/vendor/x/y.md")));
        assert!(!file.is_excluded(Path::new("/repo/docs/vendor.md")));
    }

    #[test]
    fn sembr_rules_top_level_and_in_sections() {
        let file = parse(
            r#"
style = "sembr"

[sembr]
dependent-clauses = "always"
clause-punctuation = [";", "—"]

["docs/**".sembr]
links = "width"
"#,
        );
        let top = config_for(&file, "/repo/README.md");
        assert_eq!(top.sembr.dependent_clauses, Level::Always);
        assert!(!top.sembr.clause_punctuation.comma);
        assert!(top.sembr.clause_punctuation.semicolon);
        assert_eq!(top.sembr.links, Level::Never);
        let docs = config_for(&file, "/repo/docs/a.md");
        assert_eq!(docs.sembr.links, Level::Width);
        assert_eq!(docs.sembr.dependent_clauses, Level::Always);
    }

    #[test]
    fn rejects_mistakes() {
        for text in [
            "stlye = \"sembr\"\n",
            "style = \"clauses\"\n",
            "width = \"wide\"\n",
            "exclude = \"vendor\"\n",
            "[formats]\nqmd = \"quarto\"\n",
            "[fromats]\nqmd = \"md\"\n",
            "[\"docs/**\"]\nwidht = 3\n",
            "[\"docs/**\"]\nexclude = [\"x\"]\n",
            "[\"docs/[x\"]\nstyle = \"sembr\"\n",
            "[sembr]\nlinks = \"sometimes\"\n",
            "[sembr]\nlnks = \"always\"\n",
            "[sembr]\nclause-punctuation = [\"!\"]\n",
            "sembr = \"on\"\n",
        ] {
            assert!(
                ConfigFile::parse(text, Path::new("/repo")).is_err(),
                "{text:?}"
            );
        }
    }
}
