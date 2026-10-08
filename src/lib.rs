//! sember: semantic line breaks for Markdown.
//!
//! The library entry point is [`format_markdown`]; the `sember` binary wraps
//! it with file handling and a `--check` mode.

mod markdown;
mod sentence;

pub use markdown::{Config, ExistingBreaks, Formatted, Style};

/// Reformats the line breaks in a Markdown document.
///
/// Only the whitespace between words inside paragraphs and list items
/// changes. A paragraph whose rewrite would alter the rendered document is
/// left as it was, and its line number is returned in [`Formatted::kept`].
pub fn format_markdown(src: &str, config: &Config) -> Formatted {
    markdown::format(src, config)
}
