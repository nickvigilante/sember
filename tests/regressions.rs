//! Cases other semantic-line-break formatters get wrong. Each one checks the
//! exact output and that a second pass changes nothing.

use sember::{Config, format_markdown};

fn assert_formats(src: &str, expected: &str) {
    let formatted = format_markdown(src, &Config::default());
    assert_eq!(formatted.output, expected);
    assert!(formatted.kept.is_empty(), "kept: {:?}", formatted.kept);
    assert_eq!(
        format_markdown(expected, &Config::default()).output,
        expected,
        "not idempotent"
    );
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

/// Fenced divs (Quarto, Pandoc) and admonitions (Docusaurus, MDX) look
/// like paragraph text to CommonMark; their `:::` lines must stay put.
#[test]
fn fenced_divs_and_admonitions() {
    assert_formats(
        "::: {.callout-note}\nSome text. More\ntext.\n:::\n\n:::tip\nA tip. Another.\n:::\n",
        "::: {.callout-note}\nSome text.\nMore text.\n:::\n\n:::tip\nA tip.\nAnother.\n:::\n",
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

mod sembr {
    use sember::{Config, ExistingBreaks, Style, format_markdown};

    fn sembr(width: usize, existing_breaks: ExistingBreaks) -> Config {
        Config {
            style: Style::Sembr,
            width,
            existing_breaks,
        }
    }

    fn assert_formats(config: &Config, src: &str, expected: &str) {
        let formatted = format_markdown(src, config);
        assert_eq!(formatted.output, expected);
        assert!(formatted.kept.is_empty(), "kept: {:?}", formatted.kept);
        assert_eq!(
            format_markdown(expected, config).output,
            expected,
            "not idempotent"
        );
    }

    /// Breaks a person placed after punctuation survive, even ones sember
    /// wouldn't have added itself.
    #[test]
    fn keeps_deliberate_breaks() {
        let src = "Run the job nightly,\nthen prune old snapshots,\nand alert on failure.\n";
        assert_formats(&sembr(0, ExistingBreaks::Auto), src, src);
    }

    /// A hard-wrapped paragraph is unwrapped and re-broken at clauses.
    #[test]
    fn reflows_hard_wrapped_paragraphs() {
        assert_formats(
            &sembr(0, ExistingBreaks::Auto),
            "If the build fails, the logs are kept so you can see which\nresource caused the problem, and the next build starts clean.\n",
            "If the build fails,\nthe logs are kept so you can see which resource caused the problem,\nand the next build starts clean.\n",
        );
    }

    /// `reflow` ignores even punctuated breaks.
    #[test]
    fn reflow_ignores_existing_breaks() {
        assert_formats(
            &sembr(0, ExistingBreaks::Reflow),
            "Run the job nightly,\nthen prune old snapshots.\n",
            "Run the job nightly, then prune old snapshots.\n",
        );
    }

    /// With a width, clause breaks are added only where a line is too long,
    /// at the last clause boundary that fits, and never mid-phrase.
    #[test]
    fn width_picks_the_last_fitting_clause() {
        let config = sembr(50, ExistingBreaks::Auto);
        assert_formats(
            &config,
            "Short line, and it stays.\n",
            "Short line, and it stays.\n",
        );
        assert_formats(
            &config,
            "The logs are kept, which helps, and the next build starts clean.\n",
            "The logs are kept, which helps,\nand the next build starts clean.\n",
        );
        let unbreakable =
            "This sentence has no clause boundary anywhere along its considerable length.\n";
        assert_formats(&config, unbreakable, unbreakable);
    }

    /// Series and abbreviations stay together in the sembr style too.
    #[test]
    fn series_stay_together() {
        let src = "The answer is A, B, or C, e.g., depending on the input.\n";
        assert_formats(&sembr(0, ExistingBreaks::Auto), src, src);
    }

    /// Words ending in multi-byte characters before a comma.
    #[test]
    fn non_ascii_before_comma() {
        assert_formats(
            &sembr(0, ExistingBreaks::Auto),
            "We met in Kyōto, and the trip went well. Café, then bistrō.\n",
            "We met in Kyōto,\nand the trip went well.\nCafé, then bistrō.\n",
        );
    }

    /// Clause breaks in containers get the right prefix.
    #[test]
    fn clause_breaks_in_lists() {
        assert_formats(
            &sembr(0, ExistingBreaks::Auto),
            "- If the build fails, the logs are kept.\n",
            "- If the build fails,\n  the logs are kept.\n",
        );
    }
}
