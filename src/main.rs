use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use sember::Config;
use std::rc::Rc;

mod changed;
use changed::{Changes, Lines};

use sember::config::{ConfigFile, Discovery, ExistingName, Partial, StyleName, builtin_format};

/// Put each sentence of your Markdown on its own line.
///
/// With no paths, reads stdin and writes the result to stdout. With paths,
/// rewrites each file in place; directories are searched for `.md` and
/// `.markdown` files.
///
/// Settings come from the nearest `.sember.toml` at or above each file (up to
/// the repository root), and command-line flags override them.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Files or directories to format. Use `-` for stdin.
    paths: Vec<PathBuf>,

    /// Don't write anything; exit with status 1 if any file would change.
    #[arg(long)]
    check: bool,

    /// How to break lines [default: sentence].
    #[arg(long, value_enum)]
    style: Option<StyleArg>,

    /// With `--style sembr`, the line length that rules set to `width`
    /// break to stay within. 0 means no limit [default: 80].
    #[arg(long, value_name = "CHARS")]
    width: Option<usize>,

    /// With `--style sembr`, what to do with line breaks already in a
    /// paragraph [default: auto].
    #[arg(long, value_enum)]
    existing_breaks: Option<ExistingArg>,

    /// Use this config file for every input instead of searching for
    /// `.sember.toml`.
    #[arg(long, value_name = "PATH", conflicts_with = "no_config")]
    config: Option<PathBuf>,

    /// Ignore `.sember.toml` files.
    #[arg(long)]
    no_config: bool,

    /// Format only the paragraphs changed since this git ref (a branch, tag
    /// or commit), counting uncommitted and untracked changes. With no
    /// paths, formats every changed file under the current directory.
    #[arg(long, value_name = "REF")]
    changed: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
enum StyleArg {
    /// One sentence per line; sentences are never split.
    Sentence,
    /// Semantic line breaks: sentences plus the SemBr rules set in
    /// `.sember.toml`.
    Sembr,
    /// One line per paragraph.
    Paragraph,
}

#[derive(Clone, Copy, ValueEnum)]
enum ExistingArg {
    /// Keep clause breaks unless the paragraph is hard-wrapped.
    Auto,
    /// Keep every existing break on a clause boundary.
    Keep,
    /// Ignore existing breaks.
    Reflow,
}

impl Cli {
    /// The flags the user passed, as the last settings layer.
    fn flags(&self) -> Partial {
        Partial {
            style: self.style.map(|style| match style {
                StyleArg::Sentence => StyleName::Sentence,
                StyleArg::Sembr => StyleName::Sembr,
                StyleArg::Paragraph => StyleName::Paragraph,
            }),
            width: self.width,
            sembr: Default::default(),
            existing_breaks: self.existing_breaks.map(|existing| match existing {
                ExistingArg::Auto => ExistingName::Auto,
                ExistingArg::Keep => ExistingName::Keep,
                ExistingArg::Reflow => ExistingName::Reflow,
            }),
        }
    }
}

/// Where each file's settings come from.
enum Settings {
    None,
    Fixed(Rc<ConfigFile>),
    Discover(Discovery),
}

impl Settings {
    /// The config file that governs `path`, if any.
    fn file_for(&mut self, path: &Path) -> Result<Option<Rc<ConfigFile>>, String> {
        match self {
            Settings::None => Ok(None),
            Settings::Fixed(file) => Ok(Some(file.clone())),
            Settings::Discover(discovery) => {
                let absolute = absolute(path)?;
                let dir = if path == Path::new("-") {
                    absolute.as_path()
                } else {
                    absolute.parent().unwrap_or(Path::new("/"))
                };
                discovery.find(dir).map_err(|e| e.to_string())
            }
        }
    }

    /// The config for one input: defaults, then the config file (top-level
    /// settings and matching sections), then the command-line flags.
    fn for_path(&mut self, path: &Path, flags: &Partial) -> Result<Config, String> {
        let mut config = Config::default();
        if let Some(file) = self.file_for(path)? {
            file.apply(&absolute(path)?, &mut config);
        }
        flags.apply(&mut config);
        Ok(config)
    }

    /// Whether to format `path`: `Ok(false)` when it's excluded or (found
    /// by walking a directory) isn't a format sember knows.
    fn wants(&mut self, path: &Path, explicit: bool) -> Result<bool, String> {
        let file = self.file_for(path)?;
        let absolute = absolute(path)?;
        if file
            .as_ref()
            .is_some_and(|file| file.is_excluded(&absolute))
        {
            return Ok(false);
        }
        let format = match &file {
            Some(file) => file.format_of(path),
            None => builtin_format(path),
        };
        match (format, explicit) {
            (Some(_), _) => Ok(true),
            (None, false) => Ok(false),
            // A missing file is reported when it's read.
            (None, true) if !path.exists() => Ok(true),
            (None, true) => Err(match path.extension().and_then(|ext| ext.to_str()) {
                Some(ext) => format!(
                    "unrecognized extension `.{ext}`; map it to a format in .sember.toml, e.g. [formats] {ext} = \"md\""
                ),
                None => "no file extension, so the format is unknown".to_owned(),
            }),
        }
    }
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    if path == Path::new("-") {
        std::env::current_dir().map_err(|e| e.to_string())
    } else {
        std::path::absolute(path).map_err(|e| e.to_string())
    }
}

const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target"];

fn main() -> ExitCode {
    let cli = Cli::parse();
    let flags = cli.flags();
    let mut settings = if cli.no_config {
        Settings::None
    } else if let Some(path) = &cli.config {
        match std::path::absolute(path)
            .map_err(|e| format!("{}: {e}", path.display()))
            .and_then(|path| ConfigFile::load(&path).map_err(|e| e.to_string()))
        {
            Ok(file) => Settings::Fixed(Rc::new(file)),
            Err(err) => {
                eprintln!("sember: {err}");
                return ExitCode::from(2);
            }
        }
    } else {
        Settings::Discover(Discovery::default())
    };
    let changes = match cli.changed.as_deref().map(Changes::load).transpose() {
        Ok(changes) => changes,
        Err(err) => {
            eprintln!("sember: {err}");
            return ExitCode::from(2);
        }
    };
    let explicit = !cli.paths.is_empty();
    let paths = match &changes {
        Some(changes) if !explicit => changes.files().to_vec(),
        _ if !explicit => vec![PathBuf::from("-")],
        _ => cli.paths,
    };
    if changes.is_some() && paths.iter().any(|path| path == Path::new("-")) {
        eprintln!("sember: --changed can't read stdin; name files or directories");
        return ExitCode::from(2);
    }

    let mut files = Vec::new();
    let mut failed = false;
    for path in &paths {
        if let Err(err) = collect(path, explicit, &mut settings, &mut files) {
            eprintln!("sember: {}: {err}", path.display());
            failed = true;
        }
    }

    let mut would_change = Vec::new();
    for file in &files {
        let config = match settings.for_path(file, &flags) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("sember: {err}");
                failed = true;
                continue;
            }
        };
        let lines = match changes
            .as_ref()
            .map(|changes| changes.lines(file))
            .transpose()
        {
            Ok(Some(None)) => continue,
            Ok(lines) => lines.flatten(),
            Err(err) => {
                eprintln!("sember: {}: {err}", file.display());
                failed = true;
                continue;
            }
        };
        match run(file, cli.check, &config, lines) {
            Ok(true) => would_change.push(file),
            Ok(false) => {}
            Err(err) => {
                eprintln!("sember: {}: {err}", file.display());
                failed = true;
            }
        }
    }

    if cli.check && !would_change.is_empty() {
        for file in &would_change {
            eprintln!("would reformat {}", display(file));
        }
        eprintln!(
            "{} file{} would be reformatted",
            would_change.len(),
            if would_change.len() == 1 { "" } else { "s" }
        );
        return ExitCode::from(1);
    }
    if failed {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn display(path: &Path) -> String {
    if path == Path::new("-") {
        "<stdin>".to_owned()
    } else {
        path.display().to_string()
    }
}

/// Expands directories into the files beneath them that sember formats,
/// sorted so the output order is stable. Excluded files are skipped, even
/// when named explicitly, so a pre-commit hook passing every changed file
/// honours `exclude`.
fn collect(
    path: &Path,
    explicit: bool,
    settings: &mut Settings,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if path == Path::new("-") {
        files.push(path.to_owned());
        return Ok(());
    }
    if !path.is_dir() {
        if settings.wants(path, explicit)? {
            files.push(path.to_owned());
        }
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(path)
        .and_then(|dir| dir.collect::<Result<_, _>>())
        .map_err(|e| e.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let child = entry.path();
        let is_dir = entry.file_type().map_err(|e| e.to_string())?.is_dir();
        if is_dir {
            if !SKIP_DIRS.iter().any(|skip| entry.file_name() == *skip) {
                collect(&child, false, settings, files)?;
            }
        } else {
            collect(&child, false, settings, files)?;
        }
    }
    Ok(())
}

/// Formats one file, or with `lines` only the paragraphs on those lines.
/// Returns whether its contents changed (or would, under `--check`).
fn run(path: &Path, check: bool, config: &Config, lines: Option<Lines>) -> io::Result<bool> {
    let stdin = path == Path::new("-");
    let src = if stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        fs::read_to_string(path)?
    };

    let formatted = match lines {
        None | Some(Lines::All) => sember::format_markdown(&src, config),
        Some(Lines::Ranges(ranges)) => sember::format_markdown_lines(&src, config, &ranges),
    };
    for line in &formatted.kept {
        eprintln!(
            "{}:{line}: left as is: reformatting this paragraph would change how it renders",
            display(path)
        );
    }
    let changed = formatted.output != src;

    if stdin && !check {
        io::stdout().write_all(formatted.output.as_bytes())?;
    } else if changed && !check {
        fs::write(path, &formatted.output)?;
    }
    Ok(changed)
}
