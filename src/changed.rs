//! `--changed <ref>`: which files, and which lines in them, changed since a
//! git ref.

use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The lines a file changed since the ref.
#[derive(Debug, Clone)]
pub enum Lines {
    /// Untracked, so every line is new.
    All,
    Ranges(Vec<RangeInclusive<usize>>),
}

/// The files changed between a ref and the working tree, from the current
/// directory down.
pub struct Changes {
    reference: String,
    /// Changed files, relative to the current directory.
    files: Vec<PathBuf>,
    /// The same files, canonicalized, for matching paths given on the
    /// command line; `true` when untracked.
    canonical: HashMap<PathBuf, (PathBuf, bool)>,
}

impl Changes {
    pub fn load(reference: &str) -> Result<Self, String> {
        let commit = format!("{reference}^{{commit}}");
        git(&["rev-parse", "--verify", "--quiet", &commit])
            .map_err(|_| format!("--changed: `{reference}` is not a commit in this repository"))?;

        let tracked = git(&[
            "diff",
            "--name-only",
            "-z",
            "--relative",
            "--no-renames",
            "--diff-filter=d",
            reference,
            "--",
        ])?;
        let untracked = git(&["ls-files", "-z", "--others", "--exclude-standard"])?;

        let mut files = Vec::new();
        let mut canonical = HashMap::new();
        for (list, is_untracked) in [(tracked, false), (untracked, true)] {
            for name in list.split('\0').filter(|name| !name.is_empty()) {
                let path = PathBuf::from(name);
                // A path that's gone (a type change, a racing delete) has no
                // lines to format.
                if let Ok(real) = path.canonicalize() {
                    canonical.insert(real, (path.clone(), is_untracked));
                    files.push(path);
                }
            }
        }
        files.sort();
        files.dedup();
        Ok(Self {
            reference: reference.to_owned(),
            files,
            canonical,
        })
    }

    /// Every changed file, relative to the current directory.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// The lines of `path` that changed, or `None` when it didn't change.
    pub fn lines(&self, path: &Path) -> Result<Option<Lines>, String> {
        let Some((relative, untracked)) = path
            .canonicalize()
            .ok()
            .and_then(|real| self.canonical.get(&real))
        else {
            return Ok(None);
        };
        if *untracked {
            return Ok(Some(Lines::All));
        }
        let pathspec = format!(":(literal){}", relative.display());
        let diff = git(&[
            "diff",
            "--unified=0",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--no-renames",
            &self.reference,
            "--",
            &pathspec,
        ])?;
        Ok(Some(Lines::Ranges(hunks(&diff))))
    }
}

/// The new-side line ranges of a `--unified=0` diff's hunks. A hunk that
/// only deletes touches the lines on either side of the deletion, since
/// the paragraph around it changed.
fn hunks(diff: &str) -> Vec<RangeInclusive<usize>> {
    diff.lines()
        .filter_map(|line| {
            let header = line.strip_prefix("@@ -")?;
            let new = header.split(' ').nth(1)?.strip_prefix('+')?;
            let (start, count): (usize, usize) = match new.split_once(',') {
                Some((start, count)) => (start.parse().ok()?, count.parse().ok()?),
                None => (new.parse().ok()?, 1),
            };
            Some(match count {
                0 => start.max(1)..=start + 1,
                _ => start..=start + count - 1,
            })
        })
        .collect()
}

fn git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("--changed needs git: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {}: {}", args[0], stderr.trim()));
    }
    String::from_utf8(output.stdout).map_err(|_| format!("git {}: output is not UTF-8", args[0]))
}

#[cfg(test)]
mod tests {
    use super::hunks;

    #[test]
    fn parses_hunk_headers() {
        let diff = "\
diff --git a/a.md b/a.md
--- a/a.md
+++ b/a.md
@@ -3 +3 @@ heading
-old
+new
@@ -10,0 +11,2 @@
+one
+two
@@ -20,2 +21,0 @@
-gone
-gone
@@ -1,1 +0,0 @@
";
        assert_eq!(hunks(diff), vec![3..=3, 11..=12, 21..=22, 1..=1]);
    }
}
