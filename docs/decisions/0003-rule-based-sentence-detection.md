# 3. Detect sentences with conservative rules

Status: accepted.
Recorded after the fact; the decision landed in commit `1084e1f` (#2), with a refinement for code spans in `e88ff7a` (#10).

## Context

Putting each sentence on its own line needs a way to find where a sentence ends.
The text is Markdown prose full of ambiguity: "Dr. Smith", "U.S. Army", "e.g. Rust", initials, and code spans that may or may not end a sentence.

## Decision

A hand-written, rule-based detector (`is_boundary` in `src/sentence.rs`) with a deliberate lean toward not breaking.

Documented in the module comment: "when a boundary is ambiguous ("U.S. Army", "Dr. Smith"), sember keeps the two pieces on one line.
A missed break leaves two sentences on a line; a wrong break splits one."
The first commit says the same: "ambiguous boundaries (initials, "Dr.", "e.g.") stay joined."

The rules are:

- A list of abbreviations that never end a sentence, even before a capital letter (`NEVER_FINAL`).
- Character sets for closing and opening punctuation, a special case for "No.", and a check for initials.
- A code span that ends in a period ends a sentence, so "Run `make test.`" followed by "Then deploy." breaks.
  A span such as `e.g.` does not.
  That rule was added in `e88ff7a`.

## Alternatives

None documented.
No statistical or language model is mentioned anywhere in the code, README or history.
A reason for avoiding one is easy to infer, since a heuristic adds no dependency and gives the same answer every time, but that is a reading and not something recorded.

## Consequences

- The cost of a wrong guess is small and visible: two sentences stay on one line, and nothing is broken in the middle.
- The abbreviation list is finite, so an unlisted abbreviation can split a sentence wrongly.
- The safety net in [record 2](0002-verify-by-reparsing.md) covers rendering changes but not this kind of mistake, which is only a matter of where a line ends.
