# sember

Semantic line breaks for cleaner text and smaller, more manageable diffs.

sember rewrites Markdown so that each sentence sits on its own source line.
Hard-wrapped paragraphs are unwrapped, and long lines holding several sentences are split.
The rendered page doesn't change, because Markdown joins the lines of a paragraph back together, but a diff now shows exactly which sentence changed.

## Status

Early.
Today sember does one-sentence-per-line for CommonMark and GitHub-flavored Markdown.
Clause-level breaks, a width limit, per-path configuration and formatting only the paragraphs changed since a git ref are planned.

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
