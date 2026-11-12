# 6. `--check` and the exit-code contract

Status: accepted.
Recorded after the fact; `--check` landed in the first working version, commit `1084e1f` (#2).

## Context

Formatters run in two situations: a person rewriting files, and CI or a pre-commit hook asking "is this already formatted?".
The second needs an answer a script can act on, and it must never change files.

## Decision

`--check` reports without writing, so it is "safe to run anywhere" (README).
For each file that would change it prints `would reformat FILE`, and it prints a count on stderr.

Exit status:

| Code | Meaning |
| --- | --- |
| 0 | Success. In `--check` mode, nothing needs reformatting. |
| 1 | `--check` found files that need reformatting. |
| 2 | An error: bad configuration, an I/O failure, a git failure or an unsupported file. |

Other behaviour in `src/main.rs`:

- With no paths, or with `-`, sember reads stdin and writes stdout.
  In `--check` mode it writes nothing.
- `--changed` with stdin is rejected with exit status 2.
- Errors are collected and processing continues, so one bad file does not stop the others (inferred).
- When both conditions occur, exit status 1 wins, so an error cannot hide that files need reformatting (the order of the checks suggests this is deliberate; inferred).

## Alternatives

None documented.

## Consequences

- CI can use the exit status alone: 0 is clean, 1 is "run sember", 2 is "something is broken".
- The repository's own CI relies on this: it runs `sember --check README.md` as a test that sember formats its own documentation.
- A script cannot tell "needs reformatting" from "failed" if both happen in one run, because 1 is reported.
