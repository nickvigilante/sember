//! Sentence-boundary rules.
//!
//! The rules are deliberately conservative: when a boundary is ambiguous
//! ("U.S. Army", "Dr. Smith"), sember keeps the two pieces on one line.
//! A missed break leaves two sentences on a line; a wrong break splits one.

/// Abbreviations that never end a sentence, even before a capital letter
/// ("Dr. Smith", "Fig. 3", "e.g. Rust").
const NEVER_FINAL: &[&str] = &[
    "e.g", "i.e", "vs", "cf", "viz", "approx", "dr", "mr", "mrs", "ms", "prof", "sr", "jr", "st",
    "mt", "fig", "figs", "eq", "eqs", "nos", "vol", "vols", "p", "pp", "ch", "sec", "ref", "refs",
    "ed", "eds", "rev", "gen", "gov", "lt", "col", "sgt", "capt", "jan", "feb", "mar", "apr",
    "jun", "jul", "aug", "sep", "sept", "oct", "nov", "dec",
];

/// Characters that can close a sentence after its final punctuation, as in
/// `word.)`, `"Quoted."` or `*emphasised.*`.
fn is_closer(c: char) -> bool {
    matches!(c, '"' | '\'' | ')' | ']' | '*' | '_' | '”' | '’' | '»')
}

/// Characters that can open a sentence before its first letter.
fn is_opener(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | '(' | '[' | '*' | '_' | '“' | '‘' | '«' | '`' | '!'
    )
}

/// Reports whether `before` ends a sentence and `after` starts a new one,
/// where `before` and `after` are the Markdown source on either side of a
/// whitespace gap.
pub fn is_boundary(before: &str, after: &str) -> bool {
    // "No." is the abbreviation in "No. 5" but an answer in "No. The server…".
    if last_word(before).eq_ignore_ascii_case("no.") {
        return after.starts_with(|c: char| !c.is_ascii_digit()) && starts_sentence(after);
    }
    ends_sentence(before) && starts_sentence(after)
}

fn last_word(text: &str) -> &str {
    text.rsplit(char::is_whitespace)
        .next()
        .unwrap_or("")
        .trim_start_matches(is_opener)
}

fn ends_sentence(before: &str) -> bool {
    let trimmed = before.trim_end_matches(is_closer);
    let Some(last) = trimmed.chars().next_back() else {
        return false;
    };
    match last {
        '!' | '?' => true,
        '.' => !is_abbreviation(trimmed),
        _ => false,
    }
}

/// `text` ends with a period; decide whether its last word is an
/// abbreviation or an initial rather than a sentence end.
fn is_abbreviation(text: &str) -> bool {
    let word = last_word(text);
    let stem = &word[..word.len() - 1];
    if stem.is_empty() {
        // A lone "." after a space or an opener, as in "foo ." or "(.".
        return true;
    }
    if stem.ends_with('.') {
        // An ellipsis ("agreed... Then") ends a sentence when a capital follows.
        return false;
    }
    // Initials and dotted acronyms: "J.", "U.S.", "e.g".
    if stem.split('.').all(|part| {
        let mut chars = part.chars();
        chars.next().is_some_and(char::is_alphabetic) && chars.next().is_none()
    }) {
        return true;
    }
    // "1." or "12." mid-sentence is usually an inline enumerator
    // ("in 2 steps: 1. build 2. test"), while "in 2024." ends a sentence.
    if stem.len() <= 2 && stem.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    let lower = stem.to_lowercase();
    NEVER_FINAL.contains(&lower.as_str())
}

fn starts_sentence(after: &str) -> bool {
    let rest = after.trim_start_matches(|c: char| is_opener(c) && c != '`');
    if after.starts_with('`') {
        // A sentence that opens with inline code: "`coder` is the CLI."
        return true;
    }
    rest.chars()
        .next()
        .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::is_boundary;

    #[test]
    fn plain_sentences() {
        assert!(is_boundary("It works.", "Then it stops."));
        assert!(is_boundary("Really?", "Yes."));
        assert!(is_boundary("Wait!", "Stop."));
        assert!(is_boundary("It shipped in 2024.", "2025 was quieter."));
    }

    #[test]
    fn closers_and_openers() {
        assert!(is_boundary("He said \"done.\"", "\"Next,\" she said."));
        assert!(is_boundary("(Parenthetical.)", "After."));
        assert!(is_boundary("This is *bold.*", "Next."));
        assert!(is_boundary("Run it.", "`coder` is the CLI."));
        assert!(is_boundary("Read the guide.", "[Learn more](x)."));
    }

    #[test]
    fn not_a_boundary() {
        assert!(!is_boundary("Use e.g.", "Rust."));
        assert!(!is_boundary("Ask Dr.", "Smith."));
        assert!(!is_boundary("Made in the U.S.", "Army."));
        assert!(!is_boundary("By J.", "Smith."));
        assert!(!is_boundary("See Fig.", "3 for details."));
        assert!(!is_boundary("Two steps: 1.", "Build it."));
        assert!(!is_boundary("See item No.", "5 below."));
        assert!(!is_boundary("It ends.", "but lowercase continues."));
        assert!(!is_boundary("A list:", "Next."));
        assert!(!is_boundary("No period", "Next."));
    }

    #[test]
    fn no_as_an_answer() {
        assert!(is_boundary(
            "Does it block? No.",
            "The server adds little latency."
        ));
    }

    #[test]
    fn ellipsis_ends_before_capital() {
        assert!(is_boundary("Dr. Smith agreed...", "Then left."));
    }
}
