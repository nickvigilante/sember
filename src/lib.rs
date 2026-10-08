//! sember: semantic line breaks for Markdown.
//!
//! The library entry point is [`format_markdown`]; the `sember` binary wraps
//! it with file handling and a `--check` mode.

use std::ops::RangeInclusive;

pub mod config;
mod markdown;
mod sentence;

pub use markdown::{
    ClausePunctuation, Config, ExistingBreaks, Formatted, Level, SembrRules, Style,
};

/// Reformats the line breaks in a Markdown document.
///
/// Only the whitespace between words inside paragraphs and list items
/// changes. A paragraph whose rewrite would alter the rendered document is
/// left as it was, and its line number is returned in [`Formatted::kept`].
pub fn format_markdown(src: &str, config: &Config) -> Formatted {
    markdown::format(src, config, None)
}

/// Like [`format_markdown`], but formats only the paragraphs that touch one
/// of `lines`: 1-based, inclusive line ranges of `src`, such as the lines a
/// diff added or changed. Every other paragraph is left exactly as it is.
pub fn format_markdown_lines(
    src: &str,
    config: &Config,
    lines: &[RangeInclusive<usize>],
) -> Formatted {
    markdown::format(src, config, Some(lines))
}
