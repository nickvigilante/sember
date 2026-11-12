# 1. Edit only the whitespace between words

Status: accepted.
Recorded after the fact; the decision landed in the first working version, commit `1084e1f` (#2).

## Context

A formatter for Markdown can work in two ways.
It can parse the document and write it back out from the parse tree, or it can leave the text alone and change only what it must.
Re-rendering normalizes everything it touches: list markers, emphasis style, table spacing, escapes.
For a tool whose purpose is smaller, more reviewable diffs, that would be self-defeating.
(Inferred.)

## Decision

sember parses the source with byte offsets and edits only the whitespace between words inside paragraphs and list items.
It joins soft line breaks with a space, and turns a space at a sentence boundary into a newline plus the container's continuation prefix, which is `> ` in a blockquote and indentation in a list item.
Every other byte of the file is copied through unchanged.

Documented in the module comment of `src/markdown.rs` (lines 3 to 8): "sember never re-renders a document. [...]
Every other byte of the file is copied through unchanged."

## How it works

- The parser is `pulldown-cmark`, driven through `into_offset_iter()` so each event carries its byte range (`src/markdown.rs`, around line 360).
- Only prose in paragraphs and tight list items is a candidate (`Block::Prose`).
  Code blocks, tables, HTML and link reference definitions are other block kinds, so they are never edited.
  This follows from the block structure; no comment states it as a rule (inferred).
- Hard breaks (a trailing backslash or two spaces) are always kept.
- Front matter is parsed as a metadata block, so it is never rewritten.
  The dialect is set explicitly in `parser_options()`: tables, footnotes, strikethrough, task lists, YAML and TOML front matter.

## Alternatives

None documented.

## Consequences

- Diffs contain only the line breaks that changed.
- Block markers, prefixes and indentation have to be rebuilt by hand for each kind of container.
- A break that would put a block marker at the start of a line is skipped rather than escaped (documented in `1084e1f`).
  That is a limit of the approach, and [record 2](0002-verify-by-reparsing.md) is the safety net behind it.
- `pulldown-cmark` is used with `default-features = false`.
  No reason is recorded; the code never uses the HTML output, so avoiding it is a plausible guess (inferred).
