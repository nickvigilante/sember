use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const WRAPPED: &str = "One sentence. Another\nsentence.\n";
const FORMATTED: &str = "One sentence.\nAnother sentence.\n";

fn sember(args: &[&str], dir: &Path, stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sember"))
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    child.wait_with_output().unwrap()
}

#[test]
fn stdin_to_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let out = sember(&[], dir.path(), Some(WRAPPED));
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap(), FORMATTED);
}

#[test]
fn rewrites_files_in_place_and_walks_directories() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("docs/nested")).unwrap();
    fs::create_dir_all(dir.path().join("docs/node_modules")).unwrap();
    for name in [
        "docs/a.md",
        "docs/nested/b.markdown",
        "docs/node_modules/c.md",
    ] {
        fs::write(dir.path().join(name), WRAPPED).unwrap();
    }
    fs::write(dir.path().join("docs/notes.txt"), WRAPPED).unwrap();

    let out = sember(&["docs"], dir.path(), None);
    assert!(out.status.success(), "{out:?}");

    let read = |name: &str| fs::read_to_string(dir.path().join(name)).unwrap();
    assert_eq!(read("docs/a.md"), FORMATTED);
    assert_eq!(read("docs/nested/b.markdown"), FORMATTED);
    assert_eq!(read("docs/node_modules/c.md"), WRAPPED);
    assert_eq!(read("docs/notes.txt"), WRAPPED);
}

#[test]
fn check_fails_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.md"), WRAPPED).unwrap();

    let out = sember(&["--check", "a.md"], dir.path(), None);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("would reformat a.md")
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("a.md")).unwrap(),
        WRAPPED
    );
}

#[test]
fn check_passes_on_formatted_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.md"), FORMATTED).unwrap();

    let out = sember(&["--check", "a.md"], dir.path(), None);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn missing_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = sember(&["nope.md"], dir.path(), None);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn sembr_style_flag() {
    let dir = tempfile::tempdir().unwrap();
    let out = sember(
        &["--style", "sembr"],
        dir.path(),
        Some("If the build fails, the logs are kept.\n"),
    );
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "If the build fails,\nthe logs are kept.\n"
    );
}
