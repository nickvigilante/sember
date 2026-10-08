use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

/// Put each sentence of your Markdown on its own line.
///
/// With no paths, reads stdin and writes the result to stdout. With paths,
/// rewrites each file in place; directories are searched for `.md` and
/// `.markdown` files.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Files or directories to format. Use `-` for stdin.
    paths: Vec<PathBuf>,

    /// Don't write anything; exit with status 1 if any file would change.
    #[arg(long)]
    check: bool,
}

const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target"];

fn main() -> ExitCode {
    let cli = Cli::parse();
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
        match run(file, cli.check) {
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
fn run(path: &Path, check: bool) -> io::Result<bool> {
    let stdin = path == Path::new("-");
    let src = if stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        fs::read_to_string(path)?
    };

    let formatted = sember::format_markdown(&src);
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
