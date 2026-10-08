//! Cases other semantic-line-break formatters get wrong. Each one checks the
//! exact output and that a second pass changes nothing.

use sember::format_markdown;

fn assert_formats(src: &str, expected: &str) {
    let formatted = format_markdown(src);
    assert_eq!(formatted.output, expected);
    assert!(formatted.kept.is_empty(), "kept: {:?}", formatted.kept);
    assert_eq!(format_markdown(expected).output, expected, "not idempotent");
}

/// A code block followed by more text inside the same list item must not
/// stop the rest of the file from being formatted.
#[test]
fn list_item_with_code_block_then_text() {
    assert_formats(
        "Para is here. Some\nwrapped text.\n\n1. Item one.\n\n   ```sh\n   cmd \\\n     > out.json\n   ```\n\n   Text after. More\n   here.\n",
        "Para is here.\nSome wrapped text.\n\n1. Item one.\n\n   ```sh\n   cmd \\\n     > out.json\n   ```\n\n   Text after.\n   More here.\n",
    );
}

/// The second sentence of a footnote must stay inside the footnote.
#[test]
fn footnote_continuation_stays_indented() {
    assert_formats(
        "Text[^1].\n\n[^1]: The note. Has two sentences.\n",
        "Text[^1].\n\n[^1]: The note.\n      Has two sentences.\n",
    );
}

/// Front matter, link reference definitions, tables and HTML blocks pass
/// through byte for byte.
#[test]
fn non_prose_is_untouched() {
    let src = "---\ntitle: A. B\n---\n\nOne line.\n\n| a. B | c. D |\n|---|---|\n| e. F | g. H |\n\n<div>Raw. HTML.</div>\n\n[ref]: https://example.com/x.y \"Title. Here\"\n";
    assert_formats(src, src);
}

/// A GitHub alert marker must stay on its own line.
#[test]
fn github_alert() {
    assert_formats(
        "> [!NOTE]\n> First sentence. Second\n> sentence.\n",
        "> [!NOTE]\n> First sentence.\n> Second sentence.\n",
    );
}

/// Breaks that would turn text into a list item, heading or quote are
/// skipped instead of escaped.
#[test]
fn no_escapes_added() {
    let src = "We shipped 2 builds; 1. the fast one and 2. the slow one.\n";
    assert_formats(src, src);
}

/// Series commas and "e.g.," are not sentence boundaries.
#[test]
fn commas_and_abbreviations() {
    let src = "The answer is A, B, or C, e.g., depending on the input. Then\nit holds.\n";
    assert_formats(
        src,
        "The answer is A, B, or C, e.g., depending on the input.\nThen it holds.\n",
    );
}

/// Hard-wrapped text in nested containers is unwrapped with the right prefix.
#[test]
fn nested_containers() {
    assert_formats(
        "> - Make sure the number of groups is under the limit. Some\n>   IdPs return an error. A\n>   common fix is a filter.\n",
        "> - Make sure the number of groups is under the limit.\n>   Some IdPs return an error.\n>   A common fix is a filter.\n",
    );
}

/// A reference link whose label wraps across lines still resolves after the
/// lines are joined.
#[test]
fn wrapped_reference_label() {
    assert_formats(
        "See the *[Using\nderive]* page. Next.\n\n[Using derive]: https://serde.rs/derive.html\n",
        "See the *[Using derive]* page.\nNext.\n\n[Using derive]: https://serde.rs/derive.html\n",
    );
}
