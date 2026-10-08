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
/// Whether `text` ends with an abbreviation or initial ("e.g.", "Dr.",
/// "J."). Text that doesn't end with a period never does.
fn is_abbreviation(text: &str) -> bool {
    let Some(stem) = last_word(text).strip_suffix('.') else {
        return false;
    };
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

/// Coordinating conjunctions that start a new clause after a comma.
const COORDINATORS: &[&str] = &["and", "but", "or", "nor", "so", "yet"];

/// Words that open a subordinate or relative clause after a comma.
const SUBORDINATORS: &[&str] = &[
    "which", "who", "whom", "whose", "where", "when", "while", "whereas", "because", "although",
    "though", "unless", "since", "if", "until", "after", "before", "once",
];

/// Words that open an introductory clause at the start of a sentence; the
/// comma that closes it ("If the build fails, the logs are kept") is a
/// clause boundary.
const INTRODUCERS: &[&str] = &[
    "if", "when", "whenever", "while", "although", "though", "because", "since", "after", "before",
    "once", "unless", "until", "whether",
];

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
}

fn bare(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// Clause punctuation that can end an independent clause (SemBr rule 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Punct {
    Comma,
    Semicolon,
    Colon,
    EmDash,
    EnDash,
}

/// The kind of clause boundary a gap sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clause {
    /// After an independent clause (SemBr rule 5).
    Independent(Punct),
    /// After a dependent clause (rule 6): "If the build fails, / the logs".
    Dependent,
    /// Between items of a series (rule 8): "A, / B, or C".
    ListItem,
}

/// Whether both sides of a gap keep at least two words, so a break never
/// strands a single word on a line. `before` is the current clause so far.
pub fn has_room(before: &str, after: &str) -> bool {
    words(before).take(2).count() == 2 && words(after).take(2).count() == 2
}

/// Whether `after` opens an inline enumerated list item such as "(1)",
/// "(b)" or "(iv)" (SemBr rule 7 recommends a break before it).
pub fn starts_inline_list(after: &str) -> bool {
    let Some(rest) = after.strip_prefix('(') else {
        return false;
    };
    let Some((label, tail)) = rest.split_once(')') else {
        return false;
    };
    let enumerator = (1..=2).contains(&label.len()) && label.bytes().all(|b| b.is_ascii_digit())
        || label.len() == 1 && label.bytes().all(|b| b.is_ascii_lowercase())
        || (1..=4).contains(&label.len()) && label.bytes().all(|b| matches!(b, b'i' | b'v' | b'x'));
    enumerator && tail.starts_with([' ', '\t'])
}

/// Classifies the gap between `before` and `after` as a clause boundary,
/// if it is one. `before` is the current clause up to the gap: the text
/// since the last sentence or clause boundary.
pub fn clause(before: &str, after: &str) -> Option<Clause> {
    if !has_room(before, after) {
        return None;
    }
    // Inside parentheses or brackets the aside stays on one line.
    let depth = |open: char, close: char| {
        before.chars().filter(|&c| c == open).count() as isize
            - before.chars().filter(|&c| c == close).count() as isize
    };
    if depth('(', ')') > 0 || depth('[', ']') > 0 {
        return None;
    }

    let trimmed = before.trim_end_matches(['*', '_', '"', '”', ')']);
    let next = bare(after.split_whitespace().next().unwrap_or(""));
    match trimmed.chars().next_back() {
        Some(';') => Some(Clause::Independent(Punct::Semicolon)),
        Some('—') => Some(Clause::Independent(Punct::EmDash)),
        Some('–') => Some(Clause::Independent(Punct::EnDash)),
        Some(':') if !after.starts_with(|c: char| c.is_ascii_digit()) => {
            Some(Clause::Independent(Punct::Colon))
        }
        Some(',') => {
            let before_words: Vec<&str> = words(before).collect();
            let last = before_words.last().copied().unwrap_or("");
            if is_abbreviation(last.trim_end_matches(',')) {
                return None;
            }
            // Another comma in the same stretch, before or after the gap,
            // means a series: "A, B, or C".
            let earlier = &trimmed[..trimmed.len() - 1];
            let in_series = earlier
                .rsplit([';', ':'])
                .next()
                .is_some_and(|stretch| stretch.contains(','));
            let series_ahead = after
                .split(['.', ';', ':', '!', '?'])
                .next()
                .is_some_and(|stretch| stretch.contains(','));
            if COORDINATORS.contains(&next.as_str()) {
                return Some(if in_series {
                    Clause::ListItem
                } else {
                    Clause::Independent(Punct::Comma)
                });
            }
            if SUBORDINATORS.contains(&next.as_str()) {
                return Some(Clause::Dependent);
            }
            // The first comma of a clause that opens with "If", "When"...,
            // possibly after a conjunction ("and when a user logs in, the").
            let mut opening = before_words.iter().map(|w| bare(w));
            let first = opening
                .find(|w| !COORDINATORS.contains(&w.as_str()))
                .unwrap_or_default();
            if INTRODUCERS.contains(&first.as_str())
                && !earlier.contains(',')
                && before_words.len() >= 3
            {
                return Some(Clause::Dependent);
            }
            (in_series || series_ahead).then_some(Clause::ListItem)
        }
        _ if before.ends_with(" --") => Some(Clause::Independent(Punct::EmDash)),
        _ => None,
    }
}

/// Whether the gap is a clause boundary of a kind `sember` breaks at by
/// default: an independent or dependent clause, not a list item.
#[cfg(test)]
fn is_clause_boundary(before: &str, after: &str) -> bool {
    matches!(
        clause(before, after),
        Some(Clause::Independent(_) | Clause::Dependent)
    )
}

#[cfg(test)]
mod tests {
    use super::{Clause, Punct, clause, is_boundary, is_clause_boundary, starts_inline_list};

    #[test]
    fn clause_kinds() {
        assert_eq!(
            clause("Run the job nightly;", "then prune old snapshots."),
            Some(Clause::Independent(Punct::Semicolon))
        );
        assert_eq!(
            clause("Coder runs the template,", "and the build starts."),
            Some(Clause::Independent(Punct::Comma))
        );
        assert_eq!(
            clause("If the build fails,", "the logs are kept."),
            Some(Clause::Dependent)
        );
        assert_eq!(
            clause("Logs are kept,", "which helps debugging."),
            Some(Clause::Dependent)
        );
        assert_eq!(
            clause("The answer is A, B,", "or C today."),
            Some(Clause::ListItem)
        );
        assert_eq!(
            clause("Pick the red one,", "the blue one, or both."),
            Some(Clause::ListItem)
        );
    }

    #[test]
    fn inline_lists() {
        assert!(starts_inline_list("(1) build it"));
        assert!(starts_inline_list("(b) test it"));
        assert!(starts_inline_list("(iv) ship it"));
        assert!(!starts_inline_list("(see below) for more"));
        assert!(!starts_inline_list("(1)build"));
    }

    #[test]
    fn clause_boundaries() {
        assert!(is_clause_boundary(
            "Run the job nightly;",
            "then prune old snapshots."
        ));
        assert!(is_clause_boundary(
            "Coder runs the template,",
            "and the build starts."
        ));
        assert!(is_clause_boundary(
            "Logs are kept,",
            "which helps debugging."
        ));
        assert!(is_clause_boundary(
            "If the build fails,",
            "the logs are kept."
        ));
        assert!(is_clause_boundary(
            "There are two options:",
            "keep them or drop them."
        ));
        assert!(is_clause_boundary("It is fast —", "very fast indeed."));
        assert!(is_clause_boundary(
            "and when a developer creates a workspace,",
            "the control plane runs it."
        ));
    }

    #[test]
    fn not_clause_boundaries() {
        // Series.
        assert!(!is_clause_boundary("The answer is A, B,", "or C today."));
        // Plain commas, abbreviations, asides, and stranded single words.
        assert!(!is_clause_boundary("However,", "the logs are kept."));
        assert!(!is_clause_boundary(
            "Use a provider, e.g.,",
            "Okta or Keycloak."
        ));
        assert!(!is_clause_boundary("Pick one (fast,", "or slow) now."));
        assert!(!is_clause_boundary("It is red,", "and."));
        assert!(!is_clause_boundary("Logs are kept, the", "build fails."));
        assert!(!is_clause_boundary("Start at 10:", "30 tomorrow."));
        // "As" usually opens a phrase, not a clause.
        assert!(!is_clause_boundary(
            "As an Owner or Admin,",
            "go to the settings."
        ));
    }

    #[test]
    fn non_ascii_words() {
        assert!(is_clause_boundary(
            "Visit Kyōto and Ōsaka,",
            "and then Nagoya."
        ));
        assert!(is_boundary("Visit Kyōto.", "Then Ōsaka."));
        assert!(!is_boundary("Ask Dr.", "Ōtsuka today."));
    }

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
