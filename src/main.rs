use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use sember::Config;
use sember::config::{ConfigFile, Discovery, ExistingName, Partial, StyleName};

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

    /// With `--style sembr`, add clause breaks only to lines longer than
    /// this many characters. 0 breaks at every clause boundary [default: 0].
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
}

#[derive(Clone, Copy, ValueEnum)]
enum StyleArg {
    /// One sentence per line; sentences are never split.
    Sentence,
    /// Semantic line breaks: sentences plus clause boundaries.
    Sembr,
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
            }),
            width: self.width,
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
    Fixed(ConfigFile),
    Discover(Discovery),
}

impl Settings {
    /// The config for one input: defaults, then the config file (top-level
    /// settings and matching overrides), then the command-line flags.
    fn for_path(&mut self, path: &Path, flags: &Partial) -> Result<Config, String> {
        let mut config = Config::default();
        let absolute = if path == Path::new("-") {
            std::env::current_dir().map_err(|e| e.to_string())?
        } else {
            std::path::absolute(path).map_err(|e| e.to_string())?
        };
        match self {
            Settings::None => {}
            Settings::Fixed(file) => file.apply(&absolute, &mut config),
            Settings::Discover(discovery) => {
                let dir = if path == Path::new("-") {
                    absolute.as_path()
                } else {
                    absolute.parent().unwrap_or(Path::new("/"))
                };
                if let Some(file) = discovery.find(dir).map_err(|e| e.to_string())? {
                    file.apply(&absolute, &mut config);
                }
            }
        }
        flags.apply(&mut config);
        Ok(config)
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
            Ok(file) => Settings::Fixed(file),
            Err(err) => {
                eprintln!("sember: {err}");
                return ExitCode::from(2);
            }
        }
    } else {
        Settings::Discover(Discovery::default())
    };
    let paths = if cli.paths.is_empty() {
        vec![PathBuf::from("-")]
    } else {
        cli.paths
    };

    let mut files = Vec::new();
    let mut failed = false;
    for path in &paths {
        if let Err(err) = collect(path, &mut files) {
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
        match run(file, cli.check, &config) {
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

/// Expands directories into the Markdown files beneath them, sorted so the
/// output order is stable.
fn collect(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if path == Path::new("-") || !path.is_dir() {
        files.push(path.to_owned());
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let child = entry.path();
        let name = entry.file_name();
        if entry.file_type()?.is_dir() {
            if !SKIP_DIRS.iter().any(|skip| name == *skip) {
                collect(&child, files)?;
            }
        } else if child
            .extension()
            .is_some_and(|ext| ext == "md" || ext == "markdown")
        {
            files.push(child);
        }
    }
    Ok(())
}

/// Formats one file. Returns whether its contents changed (or would, under
/// `--check`).
fn run(path: &Path, check: bool, config: &Config) -> io::Result<bool> {
    let stdin = path == Path::new("-");
    let src = if stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        fs::read_to_string(path)?
    };

    let formatted = sember::format_markdown(&src, config);
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
