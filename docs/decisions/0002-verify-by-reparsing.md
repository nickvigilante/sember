# 2. Re-parse the result and leave back any paragraph that would change rendering

Status: accepted.
Recorded after the fact; the decision landed in commit `1084e1f` (#2).

## Context

Changing only whitespace ([record 1](0001-edit-whitespace-only.md)) is not always safe.
A line break placed before something that looks like a list marker or a heading can change what the document means.
The sentence rules ([record 3](0003-rule-based-sentence-detection.md)) are heuristics, so they cannot be trusted to avoid every such case.

## Decision

After editing, sember parses the result again and compares it with the original.
The comparison uses a signature of the event stream, with soft breaks counted as spaces and runs of whitespace collapsed (`signature`, `src/markdown.rs`).
If the two differ, it bisects to find the paragraphs responsible, leaves those exactly as they were, and reports their line numbers.
Bisection costs "a few dozen re-parses instead of one per paragraph" (code comment).

Documented in the module comment: a paragraph whose edits would change how the document renders "is left as it was and reported, instead of being written or silently skipped."
The commit message adds: "one bad paragraph never blocks the rest of the file and never goes unnoticed."

## Alternatives

The commit message names two and rejects both:

- Escaping the offending marker.
- Skipping the paragraph silently.

Re-parsing after every paragraph is implicitly rejected for its cost.

## Consequences

- The formatter's safety does not depend on the sentence heuristics being perfect.
- It costs at least one extra parse of every file.
- A paragraph that is left back stays unformatted until someone rewords it.
- Idempotence is protected by tests rather than by this mechanism: `tests/regressions.rs` checks that a second pass changes nothing, and the README is formatted by sember with CI checking that it stays so (commit `1084e1f`).
