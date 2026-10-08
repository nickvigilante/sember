//! Markdown line-break rewriting.
//!
//! sember never re-renders a document. It parses the source with offsets,
//! finds the prose runs inside paragraphs and list items, and edits only the
//! whitespace between words: joining soft line breaks with a space, or
//! replacing a space with a newline plus the container's continuation prefix
//! (`> ` in a blockquote, indentation in a list item). Every other byte of the
//! file is copied through unchanged.
//!
//! After editing, the result is parsed again and compared with the original.
//! A paragraph whose edits would change how the document renders is left as
//! it was and reported, instead of being written or silently skipped.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::sentence;

/// The dialect sember parses: CommonMark plus the GitHub extensions docs
/// commonly use, and YAML/TOML front matter.
fn parser_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_GFM
}

/// The result of formatting one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formatted {
    pub output: String,
    /// 1-based line numbers of paragraphs left unformatted because the
    /// rewrite would have changed the rendered document.
    pub kept: Vec<usize>,
}

/// How sember breaks lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Style {
    /// One sentence per line; a sentence is never split.
    #[default]
    Sentence,
    /// Semantic line breaks: sentences, plus clause boundaries.
    Sembr,
}

/// What to do with line breaks already in a paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExistingBreaks {
    /// Keep clause breaks, unless the paragraph also breaks mid-clause,
    /// which means it was hard-wrapped and is reflowed.
    #[default]
    Auto,
    /// Keep every existing break that falls on a clause boundary.
    Keep,
    /// Ignore existing breaks and break from scratch.
    Reflow,
}

/// Formatting options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Config {
    pub style: Style,
    /// In the `sembr` style, only add clause breaks to lines longer than
    /// this many characters. 0 breaks at every clause boundary.
    pub width: usize,
    /// In the `sembr` style, whether to keep clause breaks already there.
    pub existing_breaks: ExistingBreaks,
}

/// Rewrites the line breaks in a document's paragraph prose and leaves
/// everything else untouched.
pub fn format(src: &str, config: &Config) -> Formatted {
    let runs = collect_runs(src);
    let edits: Vec<Vec<Edit>> = runs.iter().map(|run| run_edits(src, run, config)).collect();
    let all: Vec<&Edit> = edits.iter().flatten().collect();
    if all.is_empty() {
        return Formatted {
            output: src.to_owned(),
            kept: Vec::new(),
        };
    }

    let original = signature(src);
    let candidate = apply(src, all);
    if signature(&candidate) == original {
        return Formatted {
            output: candidate,
            kept: Vec::new(),
        };
    }

    // Some edit changes the rendering. Find the paragraphs responsible by
    // bisection, so a large document with one bad paragraph costs a few
    // dozen re-parses instead of one per paragraph.
    let candidates: Vec<usize> = (0..runs.len()).filter(|&i| !edits[i].is_empty()).collect();
    let renders_same = |chosen: &[usize]| {
        let chosen_edits = chosen.iter().flat_map(|&i| &edits[i]).collect();
        signature(&apply(src, chosen_edits)) == original
    };
    let mut accepted = Vec::new();
    let mut kept = Vec::new();
    bisect(&candidates, &mut accepted, &mut kept, &renders_same);
    kept.sort_unstable();

    // `accepted` only ever grows by sets that were checked together with
    // everything accepted before them, so this output renders the same.
    Formatted {
        output: apply(src, accepted.iter().flat_map(|&i| &edits[i]).collect()),
        kept: kept
            .iter()
            .map(|&i| line_number(src, runs[i].start))
            .collect(),
    }
}

/// Accepts the largest subsets of `group` that keep the rendering the same
/// when combined with what is already accepted; single runs that can't be
/// accepted go to `kept`.
fn bisect(
    group: &[usize],
    accepted: &mut Vec<usize>,
    kept: &mut Vec<usize>,
    renders_same: &dyn Fn(&[usize]) -> bool,
) {
    if group.is_empty() {
        return;
    }
    let mut trial = accepted.clone();
    trial.extend_from_slice(group);
    if renders_same(&trial) {
        accepted.extend_from_slice(group);
        return;
    }
    if let [only] = group {
        kept.push(*only);
        return;
    }
    let (left, right) = group.split_at(group.len() / 2);
    bisect(left, accepted, kept, renders_same);
    bisect(right, accepted, kept, renders_same);
}

/// One stretch of inline content inside a paragraph or a tight list item.
#[derive(Debug)]
struct Run {
    start: usize,
    end: usize,
    pieces: Vec<Piece>,
}

#[derive(Debug)]
enum Piece {
    /// Literal text whose interior spaces may become line breaks.
    Text {
        range: Range<usize>,
        breakable: bool,
    },
    /// A soft line break: from the end of the previous content to the start
    /// of the next, covering trailing spaces, the newline, the next line's
    /// container prefix and its indentation.
    Gap {
        range: Range<usize>,
        breakable: bool,
    },
    /// A hard line break (`\` or two trailing spaces). Always kept; a new
    /// line starts after it.
    Hard { range: Range<usize> },
}

#[derive(Debug, Clone)]
struct Edit {
    range: Range<usize>,
    replacement: String,
}

fn is_inline_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::Superscript
            | Tag::Subscript
            | Tag::Link { .. }
            | Tag::Image { .. }
    )
}

fn is_inline_end(tag: &TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::Link
            | TagEnd::Image
    )
}

/// Which block an inline run sits in decides whether sember may touch it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    Prose,
    Other,
}

fn collect_runs(src: &str) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    let mut current: Option<Run> = None;
    let mut link_depth = 0usize;
    let mut last_end = 0usize;
    let mut pending_gap: Option<(usize, bool)> = None;

    let finish = |current: &mut Option<Run>, runs: &mut Vec<Run>| {
        if let Some(run) = current.take() {
            runs.push(run);
        }
    };

    for (event, range) in Parser::new_ext(src, parser_options()).into_offset_iter() {
        let is_content = match &event {
            Event::Start(tag) if is_inline_tag(tag) => {
                if matches!(tag, Tag::Link { .. } | Tag::Image { .. }) {
                    link_depth += 1;
                }
                true
            }
            Event::End(tag) if is_inline_end(tag) => {
                if matches!(tag, TagEnd::Link | TagEnd::Image) {
                    link_depth = link_depth.saturating_sub(1);
                }
                // An end marker carries the whole element's range; only its
                // end matters for where the previous content stops.
                last_end = last_end.max(range.end);
                if let Some(run) = current.as_mut() {
                    run.end = run.end.max(range.end);
                }
                continue;
            }
            Event::Text(_)
            | Event::Code(_)
            | Event::InlineHtml(_)
            | Event::InlineMath(_)
            | Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::HardBreak => true,
            Event::SoftBreak => {
                pending_gap = Some((last_end, link_depth == 0));
                continue;
            }
            Event::Start(tag) => {
                finish(&mut current, &mut runs);
                blocks.push(match tag {
                    Tag::Paragraph | Tag::Item => Block::Prose,
                    _ => Block::Other,
                });
                continue;
            }
            Event::End(_) => {
                finish(&mut current, &mut runs);
                blocks.pop();
                continue;
            }
            _ => {
                finish(&mut current, &mut runs);
                continue;
            }
        };
        if !is_content {
            continue;
        }

        let run = current.get_or_insert_with(|| Run {
            start: range.start,
            end: range.start,
            pieces: Vec::new(),
        });
        if let Some((gap_start, breakable)) = pending_gap.take() {
            run.pieces.push(Piece::Gap {
                range: gap_start..range.start,
                breakable,
            });
        }
        match event {
            Event::Text(_) => run.pieces.push(Piece::Text {
                range: range.clone(),
                breakable: link_depth == 0,
            }),
            Event::HardBreak => run.pieces.push(Piece::Hard {
                range: range.clone(),
            }),
            _ => {}
        }
        run.end = run.end.max(range.end);
        last_end = range.end;

        // Runs outside prose blocks are collected only to be dropped, so
        // that headings, table cells and front matter are never edited.
        if blocks.last() != Some(&Block::Prose) {
            run.pieces.clear();
        }
    }
    finish(&mut current, &mut runs);
    runs.retain(|run| {
        run.pieces
            .iter()
            .any(|piece| !matches!(piece, Piece::Hard { .. }))
    });
    runs
}

/// The prefix a new line inside this run needs: `>` markers are kept and
/// everything else on the run's first line before it (list markers, footnote
/// labels, indentation) becomes spaces of the same width.
fn continuation_prefix(src: &str, run: &Run) -> String {
    let line_start = src[..run.start].rfind('\n').map_or(0, |i| i + 1);
    src[line_start..run.start]
        .chars()
        .map(|c| match c {
            '>' | '\t' => c,
            _ => ' ',
        })
        .collect()
}

/// A place where the line may break: a soft break in the source, or a run
/// of spaces inside text.
#[derive(Debug)]
struct Slot {
    range: Range<usize>,
    /// The source already breaks the line here.
    existing: bool,
    kind: Kind,
    /// The text before the slot ends with clause punctuation (`,` `;` `:`
    /// or a dash). An existing break here can be a deliberate one even when
    /// sember wouldn't add it.
    punctuated: bool,
    /// Character offset of the slot in the run with every slot as one space,
    /// used for measuring line widths.
    column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Ends a sentence: always a line break.
    Sentence,
    /// Ends a clause: a break in the `sembr` style.
    Clause,
    /// Mid-clause, or inside a link: never a break.
    None,
    /// A hard break in the source: always kept.
    Hard,
}

fn slots(src: &str, run: &Run, config: &Config) -> Vec<Slot> {
    let mut slots = Vec::new();
    let mut clause_start = run.start;
    let mut column = 0;
    let mut cursor = run.start;
    let mut push = |range: Range<usize>, existing: bool, breakable: bool, slots: &mut Vec<Slot>| {
        column += src[cursor..range.start].chars().count();
        cursor = range.end;
        let before = &src[run.start..range.start];
        let after = &src[range.end..run.end];
        let allowed = breakable && !starts_block(after);
        let kind = if !allowed {
            Kind::None
        } else if sentence::is_boundary(before, after) {
            clause_start = range.end;
            Kind::Sentence
        } else if config.style == Style::Sembr
            && sentence::is_clause_boundary(&src[clause_start..range.start], after)
        {
            clause_start = range.end;
            Kind::Clause
        } else {
            Kind::None
        };
        let punctuated = allowed
            && before
                .trim_end_matches(['*', '_', '"', '”', ')'])
                .ends_with([',', ';', ':', '—', '–']);
        slots.push(Slot {
            range,
            existing,
            kind,
            punctuated,
            column,
        });
        column += 1;
    };

    for piece in &run.pieces {
        match piece {
            Piece::Gap { range, breakable } => push(range.clone(), true, *breakable, &mut slots),
            Piece::Hard { range } => {
                push(range.clone(), true, false, &mut slots);
                if let Some(slot) = slots.last_mut() {
                    slot.kind = Kind::Hard;
                }
            }
            Piece::Text { range, breakable } => {
                if !breakable {
                    continue;
                }
                let text = &src[range.clone()];
                let mut offset = 0;
                while let Some(pos) = text[offset..].find([' ', '\t']) {
                    let start = offset + pos;
                    let end = start
                        + text[start..]
                            .find(|c: char| c != ' ' && c != '\t')
                            .unwrap_or(text.len() - start);
                    push(
                        range.start + start..range.start + end,
                        false,
                        true,
                        &mut slots,
                    );
                    offset = end;
                }
            }
        }
    }
    slots
}

/// Decides which slots in a run become line breaks, and returns the edits.
fn run_edits(src: &str, run: &Run, config: &Config) -> Vec<Edit> {
    let prefix = continuation_prefix(src, run);
    let newline = format!("\n{prefix}");
    let slots = slots(src, run, config);
    let total = slots.last().map_or(0, |slot| slot.column + 1)
        + src[slots.last().map_or(run.start, |slot| slot.range.end)..run.end]
            .chars()
            .count();

    // A paragraph with any existing break mid-phrase (no punctuation before
    // it) was wrapped to a width, not by meaning, so none of its existing
    // breaks are worth keeping.
    let keep_existing = config.style == Style::Sembr
        && match config.existing_breaks {
            ExistingBreaks::Keep => true,
            ExistingBreaks::Reflow => false,
            ExistingBreaks::Auto => !slots
                .iter()
                .any(|slot| slot.existing && slot.kind == Kind::None && !slot.punctuated),
        };

    // A line break next to a `:::` line belongs to a fenced div (Quarto,
    // Pandoc) or an admonition (Docusaurus, MDX). Plain CommonMark reads it
    // as paragraph text, so the rendering check can't see the damage that
    // joining it would do; leave those breaks exactly as they are.
    let fenced: Vec<bool> = slots
        .iter()
        .map(|slot| slot.existing && touches_fence_line(src, &slot.range))
        .collect();

    let mut breaks: Vec<bool> = slots
        .iter()
        .zip(&fenced)
        .map(|(slot, &fenced)| match slot.kind {
            _ if fenced => true,
            Kind::Sentence | Kind::Hard => true,
            Kind::Clause | Kind::None => {
                keep_existing && slot.existing && (slot.kind == Kind::Clause || slot.punctuated)
            }
        })
        .collect();

    if config.style == Style::Sembr {
        if config.width == 0 {
            for (slot, brk) in slots.iter().zip(breaks.iter_mut()) {
                *brk |= slot.kind == Kind::Clause;
            }
        } else {
            fit_to_width(
                &slots,
                &mut breaks,
                total,
                prefix.chars().count(),
                config.width,
            );
        }
    }

    let mut edits = Vec::new();
    for ((slot, brk), fenced) in slots.iter().zip(breaks).zip(fenced) {
        if slot.kind == Kind::Hard || fenced {
            continue;
        }
        let replacement = if brk {
            newline.as_str()
        } else if slot.existing {
            " "
        } else {
            continue;
        };
        if src[slot.range.clone()] != *replacement {
            edits.push(Edit {
                range: slot.range.clone(),
                replacement: replacement.to_owned(),
            });
        }
    }
    edits
}

/// Within each line already fixed by sentence or kept breaks, adds clause
/// breaks until the line fits in `width` characters: the latest clause
/// boundary that fits, or the earliest one past the limit if none does.
/// Lines with no clause boundary stay long rather than breaking mid-phrase.
fn fit_to_width(slots: &[Slot], breaks: &mut [bool], total: usize, indent: usize, width: usize) {
    let mut line_start = 0;
    let mut i = 0;
    while i <= slots.len() {
        // The next fixed break (or the end of the run) closes this line.
        let next_fixed = (i..slots.len()).find(|&j| breaks[j]);
        let line_end = next_fixed.map_or(total, |j| slots[j].column);
        let candidates: Vec<usize> = (i..next_fixed.unwrap_or(slots.len()))
            .filter(|&j| slots[j].kind == Kind::Clause)
            .collect();
        let mut start = line_start;
        let mut remaining = candidates.as_slice();
        while indent + line_end - start > width {
            let fits = remaining
                .iter()
                .rposition(|&j| indent + slots[j].column - start <= width);
            let Some(pick) = fits.or(if remaining.is_empty() { None } else { Some(0) }) else {
                break;
            };
            let j = remaining[pick];
            breaks[j] = true;
            start = slots[j].column + 1;
            remaining = &remaining[pick + 1..];
        }
        match next_fixed {
            Some(j) => {
                line_start = slots[j].column + 1;
                i = j + 1;
            }
            None => break,
        }
    }
}

/// Whether the line before or after a line break starts with `:::`, after
/// any container prefix.
fn touches_fence_line(src: &str, gap: &Range<usize>) -> bool {
    let is_fence = |line: &str| line.trim_start_matches(['>', ' ', '\t']).starts_with(":::");
    let previous_start = src[..gap.start].rfind('\n').map_or(0, |i| i + 1);
    let next_end = src[gap.end..].find('\n').map_or(src.len(), |i| gap.end + i);
    is_fence(&src[previous_start..gap.start]) || is_fence(&src[gap.end..next_end])
}

/// Whether a line starting with `text` could be read as the start of a new
/// block (heading, list item, blockquote, fence, HTML, table) instead of a
/// paragraph continuation. Breaks before such text are skipped rather than
/// escaped, so the source stays clean.
fn starts_block(text: &str) -> bool {
    let followed_by_space = |rest: &str| rest.is_empty() || rest.starts_with([' ', '\t', '\n']);
    if text.starts_with(['#', '>', '<', '|', '='])
        || text.starts_with("```")
        || text.starts_with("~~~")
    {
        return true;
    }
    if let Some(rest) = text.strip_prefix(['-', '+', '*']) {
        return followed_by_space(rest);
    }
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if (1..=9).contains(&digits)
        && let Some(rest) = text[digits..].strip_prefix(['.', ')'])
    {
        return followed_by_space(rest);
    }
    false
}

fn apply(src: &str, mut edits: Vec<&Edit>) -> String {
    edits.sort_by_key(|edit| edit.range.start);
    let mut out = String::with_capacity(src.len());
    let mut cursor = 0;
    for edit in edits {
        out.push_str(&src[cursor..edit.range.start]);
        out.push_str(&edit.replacement);
        cursor = edit.range.end;
    }
    out.push_str(&src[cursor..]);
    out
}

/// A rendering-equivalence key: the parsed event stream with soft breaks
/// treated as spaces and whitespace runs in text collapsed, which is what
/// Markdown renderers do with paragraph text.
fn signature(src: &str) -> Vec<String> {
    let mut sig = Vec::new();
    let mut text = String::new();
    let flush = |text: &mut String, sig: &mut Vec<String>| {
        if !text.is_empty() {
            let mut collapsed = String::with_capacity(text.len());
            let mut in_space = false;
            for c in text.chars() {
                if c.is_whitespace() {
                    if !in_space {
                        collapsed.push(' ');
                    }
                    in_space = true;
                } else {
                    collapsed.push(c);
                    in_space = false;
                }
            }
            sig.push(format!("T{collapsed}"));
            text.clear();
        }
    };
    for event in Parser::new_ext(src, parser_options()) {
        match event {
            Event::Text(t) => text.push_str(&t),
            Event::SoftBreak => text.push(' '),
            // A reference label may wrap ("[Using\nderive]"); renderers
            // match it case- and whitespace-insensitively, so only the
            // resolved destination and title affect the output.
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            }) => {
                flush(&mut text, &mut sig);
                sig.push(format!("Link({link_type:?}, {dest_url:?}, {title:?})"));
            }
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                ..
            }) => {
                flush(&mut text, &mut sig);
                sig.push(format!("Image({link_type:?}, {dest_url:?}, {title:?})"));
            }
            other => {
                flush(&mut text, &mut sig);
                sig.push(format!("{other:?}"));
            }
        }
    }
    flush(&mut text, &mut sig);
    sig
}

fn line_number(src: &str, offset: usize) -> usize {
    src[..offset].bytes().filter(|&b| b == b'\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::{Config, format};

    fn fmt(src: &str) -> String {
        format(src, &Config::default()).output
    }

    #[test]
    fn splits_sentences_and_joins_wrapped_lines() {
        let src = "Coder workspaces are provisioned by Terraform templates that you write and\nstore in your repository. If the build fails, the logs are kept.\n";
        assert_eq!(
            fmt(src),
            "Coder workspaces are provisioned by Terraform templates that you write and store in your repository.\nIf the build fails, the logs are kept.\n"
        );
    }

    #[test]
    fn keeps_list_and_blockquote_prefixes() {
        assert_eq!(
            fmt("- First one. Second\n  one.\n"),
            "- First one.\n  Second one.\n"
        );
        assert_eq!(
            fmt("> Quote one. Quote\n> two.\n"),
            "> Quote one.\n> Quote two.\n"
        );
        assert_eq!(
            fmt("1. Item one. Item two.\n"),
            "1. Item one.\n   Item two.\n"
        );
    }

    #[test]
    fn does_not_break_inside_links() {
        let src = "See [the docs. Really](https://example.com/a.b). Done.\n";
        assert_eq!(
            fmt(src),
            "See [the docs. Really](https://example.com/a.b).\nDone.\n"
        );
    }

    #[test]
    fn skips_breaks_that_would_start_a_block() {
        for src in [
            "The cost is 3 apples. 1. This would become a list item.\n",
            "Add one thing. + this starts with a plus.\n",
            "See this. # Not a heading.\n",
            "Quote this. > Not a quote.\n",
        ] {
            assert_eq!(fmt(src), src, "{src:?}");
        }
    }

    #[test]
    fn leaves_non_prose_alone() {
        let src = "---\ntitle: Test. Doc\n---\n\n# Heading. With a period\n\n| a. b | c. D |\n|---|---|\n| x. Y | z. W |\n\n```sh\necho One. Two.\n```\n";
        assert_eq!(fmt(src), src);
    }

    #[test]
    fn keeps_hard_breaks() {
        let src = "This has a hard break  \nafter two spaces. And another.\n";
        assert_eq!(
            fmt(src),
            "This has a hard break  \nafter two spaces.\nAnd another.\n"
        );
    }

    #[test]
    fn idempotent() {
        let src = "One. Two\nthree. Four.\n\n- A. B.\n";
        let once = fmt(src);
        assert_eq!(fmt(&once), once);
    }
}
