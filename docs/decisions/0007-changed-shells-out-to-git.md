# 7. `--changed` shells out to `git` and formats only changed paragraphs

Status: accepted.
Recorded after the fact; the decision landed in commit `cd53888` (#6), which has no message body.

## Context

Running a formatter over an existing project produces a huge diff.
The README gives the motivation: adopt sember "one edit at a time instead of in one huge diff".

## Decision

`--changed <ref>` formats only the paragraphs that changed since `<ref>` (`src/changed.rs`).

It runs `git` by name, with three commands:

1. `git rev-parse --verify <ref>^{commit}` to check the ref exists.
2. `git diff --name-only -z --relative --no-renames --diff-filter=d <ref>` for the changed files.
3. `git ls-files -z --others --exclude-standard` for untracked files.

Then it reads a zero-context diff and turns the hunk headers into line ranges on the new side.
A hunk that only deletes lines marks the lines on either side as changed, "since the paragraph around it changed" (documented).
Untracked files are formatted whole.
Uncommitted changes count.
With no paths it formats the changed files under the current directory; with paths it skips any that are unchanged.
If `git` is missing, the error is "--changed needs git: ...".

## Alternatives

None documented.
No git library appears in `Cargo.toml`.
Avoiding a heavy native dependency, and matching the user's own git behaviour such as `--exclude-standard`, are plausible reasons (inferred).

## Consequences

- `git` must be on PATH, and the ref must exist locally.
- In CI the ref has to be fetched first, because `actions/checkout` fetches only the checked-out commit (README).
  The GitHub Action handles this ([record 10](0010-action-prebuilt-binaries.md)).
- Output is read as UTF-8 and separated by NUL bytes, which survives unusual file names (inferred).
- The line-range filter lives in the formatter itself, so only paragraphs touching a changed line are rewritten.
