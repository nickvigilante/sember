# 5. `.sember.toml` laid out like `.vale.ini`, strict about mistakes

Status: accepted.
Recorded after the fact; the decision landed in commit `8d0ed14` (#4), which has no message body.
The reasoning below comes from code comments and the README, or is marked as inferred.

## Context

A project wants one standard for every run, pre-commit hook and CI job (README), and different folders may need different settings.

## Decision

The configuration file is `.sember.toml`.
Top-level keys are the defaults, and a table named by a glob, such as `["docs/**"]`, overrides them for matching files.
Every section that matches a file applies in file order, so later sections win (`src/config.rs`, lines 1 to 26).
Each layer is a struct of optional fields, so a later layer replaces only what it names.
The overall precedence is: built-in defaults, then the top level, then matching sections, then command-line flags (README, "Precedence").

**Matching.**
Globs are matched with `globset` using `literal_separator(true)`.
A pattern containing `/` matches the path relative to the config file's directory.
A pattern without `/` matches the file name at any depth, "as in .gitignore" (documented).

**Discovery.**
For each file, sember uses the nearest `.sember.toml` in its directory or a parent, stopping after the directory that holds `.git`.
`--config PATH` applies one file to every input, and `--no-config` ignores config files.

**Strictness.**
Unknown keys, values, sections and invalid globs exit with status 2 before anything is written, "so a typo can't silently fall back to the defaults" (documented, `src/config.rs`).
A bare table name that is not a glob is rejected as "far more likely a misspelt `[formats]`" (documented).

## Alternatives

None documented.
The only comparison in the sources is the layout of Vale's `.vale.ini`; no reason for choosing it is given.
Familiarity for people who write documentation tooling is a plausible reason (inferred).

## Consequences

- "Last match wins" is easy to reason about.
- Glob semantics resemble `.gitignore` but are not identical, since matching uses `globset` alone.
- The `toml` crate is used with `preserve_order`; that keeps file order available for precedence (inferred, no comment says so).
- Nested config files do not inherit from their parents: the nearest one wins entirely (a consequence of the code, not a stated choice).
- Stopping at `.git` keeps a stray config above the repository from leaking in (inferred).
