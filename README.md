# sember

Semantic line breaks for cleaner text and smaller, more manageable diffs.

sember rewrites Markdown so that each sentence sits on its own source line.
Hard-wrapped paragraphs are unwrapped, and long lines holding several sentences are split.
The rendered page doesn't change, because Markdown joins the lines of a paragraph back together, but a diff now shows exactly which sentence changed.

## Styles

- **`--style sentence`** (the default) puts one sentence per line and never splits a sentence.
- **`--style sembr`** also breaks at clause boundaries: after `;`, `:` and dashes, before a conjunction or a word like "which" or "because" that follows a comma, and after an opening "If…," or "When…" clause.
  Series ("A, B, or C"), abbreviations ("e.g.,") and asides in parentheses stay on one line, and a break never strands a single word.

In the `sembr` style:

- `--width N` adds clause breaks only to lines longer than N characters, at the last clause boundary that fits.
  A line with no clause boundary stays long rather than breaking mid-phrase.
- `--existing-breaks` decides what happens to line breaks already in a paragraph.
  `auto` (the default) keeps breaks that follow punctuation, so a person's deliberate breaks survive, but reflows any paragraph that breaks mid-phrase, since it was hard-wrapped to a width.
  `keep` always keeps breaks that follow punctuation, and `reflow` ignores existing breaks.

## Configuration

Put a `.sember.toml` in your repository so every run, pre-commit hook and CI job uses the same standard.
The layout follows Vale's `.vale.ini`: project settings at the top, then sections named by glob.

```toml
# Top level: project settings, and formatting defaults for every file.
style = "sentence"                       # "sentence" (default) or "sembr"
exclude = ["CHANGELOG.md", "vendor/**"]  # never formatted, even when named

# Treat other extensions as a format sember knows.
[formats]
qmd = "md"
mdx = "md"

# Sections named by glob. Every matching section applies, in file order,
# so later sections win.
["docs/**"]
style = "sembr"
width = 80                 # sembr: clause breaks only on longer lines (0 = always)

["docs/legacy/**"]
existing-breaks = "reflow" # sembr: "auto" (default), "keep" or "reflow"
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

## Status

Early.
Formatting only the paragraphs changed since a git ref is planned.

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

# Clause breaks too, only where a line is over 80 characters.
sember --style sembr --width 80 docs/
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
