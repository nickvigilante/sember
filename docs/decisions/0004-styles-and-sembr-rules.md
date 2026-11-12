# 4. Three styles, and SemBr rules with a Never, Width or Always level

Status: accepted.
Recorded after the fact; the decision landed across `1084e1f` (#2), `2ec9214` (#3), `3d56abe` (#5) and `53d4ac2` (#8).
Commits `3d56abe` and `53d4ac2` have no message body, so their reasoning comes only from the code and the README.

## Context

People want different layouts from the same Markdown.
Some want one sentence per line.
Some want the fuller Semantic Line Breaks specification (sembr.org), whose rules range from MUST to MAY.
Some want long lines again.

## Decision

Three styles, set with `style`:

| Style | Behaviour |
| --- | --- |
| `sentence` (default) | One sentence per line. A sentence is never split. |
| `sembr` | Follows the Semantic Line Breaks specification. |
| `paragraph` | Joins every soft break so each paragraph is one line. |

`sentence` came first, `sembr` next (`2ec9214`), and `paragraph` with the configurable rules after (`3d56abe`).

For `sembr`, the specification's rules are individually settable.
Rules 5, 6, 7, 8, 10 and 11 take one of three levels, which `Level` in `src/markdown.rs` orders from weakest to strongest: `never`, `width` (break only where a line would exceed the width), and `always`.
"The strongest rule that applies to a gap decides" (code comment).
Rules 2, 4, 9 and 13 always hold and have no setting.
The clause punctuation is a configurable list.

Documented in the README: "The defaults follow the specification's wording: SHOULD and RECOMMENDED rules break `always`, and MAY rules break for `width` or `never`."

Related settings:

- **Width.**
  The default is 80, from rule 12 of the specification, and 0 means no limit.
  With `width`, a break is added "at the last clause boundary that fits, and never breaks mid-phrase" (`2ec9214`).
- **Existing breaks.**
  `auto` (default) keeps breaks that follow punctuation, "so a person's deliberate breaks survive", and reflows a paragraph that also breaks mid-phrase, "since it was hard-wrapped".
  `keep` and `reflow` are the documented alternatives.
- **Lead-in** (`53d4ac2`).
  A bold label that opens a paragraph or list item, such as `**Note.**`, stays on its own line with `own-line` (default) or shares the line with `same-line`.
  No reason is recorded; treating the label as its own sentence is a plausible reading (inferred).

## Alternatives

For existing breaks, `keep` and `reflow` are documented.
For the styles and the levels, none are documented.
The reason `paragraph` exists is not stated; it reads as the inverse transform (inferred).

## Consequences

- The default is the simplest, safest layout, and `sembr` is opt-in.
- There are many settings.
- `auto` can override a person's partial layout when a paragraph mixes punctuation breaks with mid-phrase ones.
- `paragraph` ignores `lead-in`.
