# sember

Semantic line breaks for cleaner text and smaller, more manageable diffs.

sember rewrites Markdown so that each sentence sits on its own source line.
Hard-wrapped paragraphs are unwrapped, and long lines holding several sentences are split.
The rendered page doesn't change, because Markdown joins the lines of a paragraph back together, but a diff now shows exactly which sentence changed.

## Styles

- **`sentence`** (the default) puts one sentence per line and never splits a sentence.
- **`sembr`** follows the [Semantic Line Breaks specification](https://sembr.org): a break after every sentence, plus the optional rules below.
- **`paragraph`** puts each paragraph on one line, joining every soft line break.

Choose one with `--style` or `style` in `.sember.toml`.

### SemBr rules

Each optional rule of the specification has a level: `always` breaks wherever the rule applies, `width` breaks only where a line would otherwise be longer than `width` characters, and `never` turns the rule off.
The defaults follow the specification's wording: SHOULD and RECOMMENDED rules break `always`, and MAY rules break for `width` or `never`.

```toml
style = "sembr"
width = 80                        # rule 12 recommends 80; 0 means no limit

[sembr]
independent-clauses = "always"    # rule 5: after an independent clause
clause-punctuation = [",", ";", ":", "—", "–"]   # which marks count for rule 5
dependent-clauses = "width"       # rule 6: "If the build fails, / the logs…"
before-lists = "always"           # rule 7: before an inline (1) … (2) … list
list-items = "never"              # rule 8: between items of "A, B, or C"
links = "never"                   # rule 10: before or after a link
inline-markup = "never"           # rule 11: before **bold**, `code` and so on
```

The other rules always hold and have no setting.
A break never changes the rendered document (rule 2), every sentence ends a line (rule 4), sember only breaks at spaces, so never inside a hyphenated word (rule 9), and a line runs past the width when nothing allows a break (rule 13).
A break also never leaves a single word on a line, and never splits an abbreviation such as "e.g.," or a parenthetical aside.

### Existing line breaks

In the `sembr` style, `existing-breaks` decides what happens to line breaks already in a paragraph.
`auto` (the default) keeps breaks that follow punctuation, so a person's deliberate breaks survive, but reflows any paragraph that breaks mid-phrase, since it was hard-wrapped to a width.
`keep` always keeps breaks that follow punctuation, and `reflow` ignores existing breaks.

## Configuration

Put a `.sember.toml` in your repository so every run, pre-commit hook and CI job uses the same standard.
The layout follows Vale's `.vale.ini`: project settings at the top, then sections named by glob.

```toml
# Top level: project settings, and formatting defaults for every file.
style = "sentence"                       # "sentence" (default), "sembr" or "paragraph"
exclude = ["CHANGELOG.md", "vendor/**"]  # never formatted, even when named

# Treat other extensions as a format sember knows.
[formats]
qmd = "md"
mdx = "md"

# Sections named by glob. Every matching section applies, in file order,
# so later sections win.
["docs/**"]
style = "sembr"

["docs/legacy/**"]
existing-breaks = "reflow"

["docs/api/**".sembr]      # SemBr rules can differ per section too
links = "width"
```

- **Globs** work like `.gitignore` patterns.
  One containing `/` matches the path relative to the config file, where `*` stays within a directory and `**` crosses directories.
  One without `/` matches the file name at any depth, so `["*.mdx"]` covers every MDX file.
- **Formats:** directory runs pick up `.md` and `.markdown` files plus every extension listed under `[formats]`.
  Naming a file with an unmapped extension is an error that suggests the mapping.
  Lines next to a `:::` fence (Quarto and Pandoc divs, Docusaurus admonitions) are never joined or split.
- **Which file applies:** for each file, sember uses the nearest `.sember.toml` in its directory or a parent, stopping at the repository root (the directory with `.git`).
- **Precedence:** the defaults, then the top-level settings, then each matching section, then command-line flags.
  `--config PATH` uses one file for every input instead, and `--no-config` ignores config files.
- **Mistakes are errors:** unknown keys, values and sections, and invalid globs, stop sember with exit status 2 before it writes anything.

## Adopting it gradually

`--changed REF` formats only the paragraphs that changed since a git ref (a branch, tag or commit), so a repository can adopt sember one edit at a time instead of in one huge diff.
Uncommitted and untracked changes count, and an untracked file is formatted whole.
With no paths, it formats every changed file under the current directory; with paths, it skips the files among them that didn't change.

```sh
# In CI: fail if any paragraph this branch touched isn't formatted.
sember --check --changed origin/main

# Before committing: format what you've changed since the last commit.
sember --changed HEAD
```

In CI, fetch enough history for the ref to exist, for example `fetch-depth: 0` with `actions/checkout`.

## Status

Early.

## What it guarantees

- **Only whitespace inside paragraphs and list items changes.**
  Front matter, headings, tables, code blocks, HTML, link reference definitions and every other byte pass through untouched.
- **The rendering stays the same.**
  After rewriting, sember parses the result again and compares it with the original.
  A paragraph whose rewrite would change the rendered document is left as it was and reported with its line number.
- **No escapes are added.**
  A break that would start a line with something Markdown reads as a list item, heading, quote or fence (`1.`, `-`, `#`, `>`) is skipped instead of escaped.
- **Containers keep their prefixes.**
  New lines inside blockquotes, list items and footnotes get the right `>` markers and indentation.
- **Running it twice changes nothing.**

When a sentence boundary is ambiguous, sember keeps the text on one line.
"Dr. Smith", "e.g. Rust", "U.S. Army" and "Fig. 3" are never split.

## Usage

```sh
# Rewrite files in place; directories are searched for .md and .markdown files.
sember README.md docs/

# Exit with status 1 if anything would change, without writing (for CI).
sember --check docs/

# Read stdin, write stdout.
sember < draft.md > draft.sembr.md

# Semantic line breaks, with the SemBr rules from .sember.toml.
sember --style sembr docs/

# Only the paragraphs changed since main.
sember --changed main
```

Exit status is 0 on success, 1 when `--check` finds files to reformat, and 2 on errors such as an unreadable file.

## Install

```sh
cargo install --git https://github.com/nickvigilante/sember
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
