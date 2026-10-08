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
        Some("The build failed, and the logs are kept.\n"),
    );
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "The build failed,\nand the logs are kept.\n"
    );
}

mod config {
    use std::fs;

    use super::sember;

    const SRC: &str = "The build failed, and the logs are kept. Then\nit retries.\n";
    const SENTENCE: &str = "The build failed, and the logs are kept.\nThen it retries.\n";
    const SEMBR: &str = "The build failed,\nand the logs are kept.\nThen it retries.\n";

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        dir
    }

    fn format_file(dir: &std::path::Path, rel: &str, args: &[&str]) -> (Option<i32>, String) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, SRC).unwrap();
        let mut all = args.to_vec();
        all.push(rel);
        let out = sember(&all, dir, None);
        (out.status.code(), fs::read_to_string(&path).unwrap())
    }

    #[test]
    fn uses_the_repo_config() {
        let dir = repo();
        fs::write(dir.path().join(".sember.toml"), "style = \"sembr\"\n").unwrap();
        assert_eq!(
            format_file(dir.path(), "docs/a.md", &[]),
            (Some(0), SEMBR.into())
        );
    }

    #[test]
    fn nearest_config_wins() {
        let dir = repo();
        fs::write(dir.path().join(".sember.toml"), "style = \"sembr\"\n").unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::write(
            dir.path().join("docs/.sember.toml"),
            "style = \"sentence\"\n",
        )
        .unwrap();
        assert_eq!(
            format_file(dir.path(), "docs/a.md", &[]),
            (Some(0), SENTENCE.into())
        );
    }

    #[test]
    fn glob_sections_match_paths_relative_to_the_config() {
        let dir = repo();
        fs::write(
            dir.path().join(".sember.toml"),
            "[\"docs/**\"]\nstyle = \"sembr\"\n",
        )
        .unwrap();
        assert_eq!(
            format_file(dir.path(), "docs/a.md", &[]),
            (Some(0), SEMBR.into())
        );
        assert_eq!(
            format_file(dir.path(), "notes.md", &[]),
            (Some(0), SENTENCE.into())
        );
    }

    #[test]
    fn flags_override_the_config() {
        let dir = repo();
        fs::write(dir.path().join(".sember.toml"), "style = \"sembr\"\n").unwrap();
        assert_eq!(
            format_file(dir.path(), "a.md", &["--style", "sentence"]),
            (Some(0), SENTENCE.into())
        );
    }

    #[test]
    fn no_config_and_explicit_config() {
        let dir = repo();
        fs::write(dir.path().join(".sember.toml"), "style = \"sembr\"\n").unwrap();
        assert_eq!(
            format_file(dir.path(), "a.md", &["--no-config"]),
            (Some(0), SENTENCE.into())
        );
        fs::write(dir.path().join("other.toml"), "style = \"sentence\"\n").unwrap();
        assert_eq!(
            format_file(dir.path(), "a.md", &["--config", "other.toml"]),
            (Some(0), SENTENCE.into())
        );
    }

    #[test]
    fn stops_at_the_repository_root() {
        let outer = tempfile::tempdir().unwrap();
        fs::write(outer.path().join(".sember.toml"), "style = \"sembr\"\n").unwrap();
        let inner = outer.path().join("project");
        fs::create_dir_all(inner.join(".git")).unwrap();
        assert_eq!(format_file(&inner, "a.md", &[]), (Some(0), SENTENCE.into()));
    }

    #[test]
    fn formats_map_extensions_for_directories_and_files() {
        let dir = repo();
        fs::write(
            dir.path().join(".sember.toml"),
            "[formats]\nqmd = \"md\"\nmdx = \"md\"\n",
        )
        .unwrap();
        for name in ["docs/a.qmd", "docs/b.mdx", "docs/c.txt"] {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, SRC).unwrap();
        }
        let out = sember(&["docs"], dir.path(), None);
        assert!(out.status.success(), "{out:?}");
        let read = |name: &str| fs::read_to_string(dir.path().join(name)).unwrap();
        assert_eq!(read("docs/a.qmd"), SENTENCE);
        assert_eq!(read("docs/b.mdx"), SENTENCE);
        assert_eq!(read("docs/c.txt"), SRC);
    }

    #[test]
    fn unmapped_extension_named_explicitly_is_an_error() {
        let dir = repo();
        let (code, content) = format_file(dir.path(), "a.qmd", &[]);
        assert_eq!(code, Some(2));
        assert_eq!(content, SRC);
    }

    #[test]
    fn exclude_skips_files_even_when_named() {
        let dir = repo();
        fs::write(
            dir.path().join(".sember.toml"),
            "exclude = [\"CHANGELOG.md\", \"vendor/**\"]\n",
        )
        .unwrap();
        assert_eq!(
            format_file(dir.path(), "CHANGELOG.md", &[]),
            (Some(0), SRC.into())
        );
        assert_eq!(
            format_file(dir.path(), "vendor/lib/README.md", &[]),
            (Some(0), SRC.into())
        );
        assert_eq!(
            format_file(dir.path(), "docs/a.md", &[]),
            (Some(0), SENTENCE.into())
        );
    }

    #[test]
    fn invalid_config_is_an_error_and_writes_nothing() {
        let dir = repo();
        fs::write(dir.path().join(".sember.toml"), "stlye = \"sembr\"\n").unwrap();
        let (code, content) = format_file(dir.path(), "a.md", &[]);
        assert_eq!(code, Some(2));
        assert_eq!(content, SRC);
    }
}

mod changed {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use super::sember;

    const TWO: &str = "First one. First\ntwo.\n\nSecond one. Second\ntwo.\n";

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    /// A repository with `a.md` and `b.md` committed as `TWO`.
    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q"]);
        fs::write(dir.path().join("a.md"), TWO).unwrap();
        fs::write(dir.path().join("b.md"), TWO).unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-q", "-m", "init"]);
        dir
    }

    fn read(dir: &Path, name: &str) -> String {
        fs::read_to_string(dir.join(name)).unwrap()
    }

    #[test]
    fn formats_only_changed_paragraphs() {
        let dir = repo();
        fs::write(
            dir.path().join("a.md"),
            "First one. First\ntwo.\n\nSecond one. Second\nthree.\n",
        )
        .unwrap();
        let out = sember(&["--changed", "HEAD", "a.md", "b.md"], dir.path(), None);
        assert!(out.status.success(), "{out:?}");
        assert_eq!(
            read(dir.path(), "a.md"),
            "First one. First\ntwo.\n\nSecond one.\nSecond three.\n"
        );
        assert_eq!(read(dir.path(), "b.md"), TWO);
    }

    #[test]
    fn a_deletion_counts_as_a_change_to_its_paragraph() {
        let dir = repo();
        fs::write(
            dir.path().join("a.md"),
            "First one. First\ntwo.\n\nSecond one. Second\n",
        )
        .unwrap();
        let out = sember(&["--changed", "HEAD", "a.md"], dir.path(), None);
        assert!(out.status.success(), "{out:?}");
        assert_eq!(
            read(dir.path(), "a.md"),
            "First one. First\ntwo.\n\nSecond one.\nSecond\n"
        );
    }

    #[test]
    fn without_paths_formats_changed_and_untracked_files() {
        let dir = repo();
        fs::write(
            dir.path().join("a.md"),
            "First one. First\nthree.\n\nSecond one. Second\ntwo.\n",
        )
        .unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        fs::write(dir.path().join("docs/new.md"), TWO).unwrap();
        fs::write(dir.path().join("notes.txt"), TWO).unwrap();

        let out = sember(&["--check", "--changed", "HEAD"], dir.path(), None);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.contains("would reformat a.md"), "{stderr}");
        assert!(stderr.contains("would reformat docs/new.md"), "{stderr}");
        assert!(stderr.contains("2 files would be reformatted"), "{stderr}");

        let out = sember(&["--changed", "HEAD"], dir.path(), None);
        assert!(out.status.success(), "{out:?}");
        assert_eq!(
            read(dir.path(), "a.md"),
            "First one.\nFirst three.\n\nSecond one. Second\ntwo.\n"
        );
        assert_eq!(
            read(dir.path(), "docs/new.md"),
            "First one.\nFirst two.\n\nSecond one.\nSecond two.\n"
        );
        assert_eq!(read(dir.path(), "b.md"), TWO);
        assert_eq!(read(dir.path(), "notes.txt"), TWO);
    }

    #[test]
    fn unknown_ref_is_an_error() {
        let dir = repo();
        let out = sember(&["--changed", "no-such-branch"], dir.path(), None);
        assert_eq!(out.status.code(), Some(2));
        assert!(
            String::from_utf8(out.stderr)
                .unwrap()
                .contains("`no-such-branch` is not a commit")
        );
    }

    #[test]
    fn stdin_is_an_error() {
        let dir = repo();
        let out = sember(&["--changed", "HEAD", "-"], dir.path(), Some(TWO));
        assert_eq!(out.status.code(), Some(2));
    }
}
