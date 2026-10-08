//! "Did you mean" for an unrecognized Word name.
//!
//! The unknown-name diagnosis has always told a reader to check the spelling
//! without saying what the spelling might have been — the one next-check an
//! agent cannot act on, even though the entire vocabulary it would need is
//! compiled into the same binary. This module answers it: the Corewords
//! and whatever names the failing interpreter additionally knows,
//! ranked by edit distance from the name that did not resolve.
//!
//! Deliberately conservative. A suggestion that is not a plausible typo is
//! worse than none — it sends a repair attempt at a Word the author never
//! meant — so the distance ceiling scales with the name's length and only the
//! closest few survive.

use crate::coreword_registry::get_builtin_word_registry;

/// How many suggestions a diagnosis carries at most.
const MAX_CANDIDATES: usize = 3;

/// Largest edit distance still considered a typo, by name length. One
/// substitution in a three-letter name is a different Word (`ADD` / `AND`);
/// one in a longer name almost never is.
fn distance_ceiling(len: usize) -> usize {
    match len {
        0..=3 => 1,
        4..=7 => 2,
        _ => 3,
    }
}

/// Known Words within a plausible typo distance of `name`, best match first.
///
/// `extra` supplies names the compiled-in registry cannot know — user Words
/// and live bindings from the interpreter that raised the failure. Matching is
/// case-insensitive because Ajisai resolves names that way.
pub(crate) fn suggest_words<'a>(name: &str, extra: impl Iterator<Item = &'a str>) -> Vec<String> {
    // What was written, and what it folds to: `ＡＤＤ` folds to `ADD`, which
    // is then the suggestion rather than the name that resolved.
    let written = name.trim().to_uppercase();
    let needle = fold_full_width(&written);
    if needle.is_empty() {
        return Vec::new();
    }
    let needle: Vec<char> = needle.chars().collect();
    let ceiling = distance_ceiling(needle.len());

    let vocabulary = get_builtin_word_registry()
        .iter()
        .map(|entry| entry.name.to_string())
        .chain(extra.map(|word| word.to_string()));

    let mut scored: Vec<(usize, String)> = Vec::new();
    for candidate in vocabulary {
        let upper = candidate.to_uppercase();
        if upper == written {
            // The name resolves after all — a caller asking about it wants a
            // different diagnosis than a spelling hint.
            continue;
        }
        // A symbol (`+`, `^`) is never a typo of an alphabetic name;
        // its edit distance is small only because it is short.
        if !upper.chars().any(|c| c.is_alphanumeric()) {
            continue;
        }
        let upper: Vec<char> = upper.chars().collect();
        let Some(distance) = edit_distance_within(&needle, &upper, ceiling) else {
            continue;
        };
        if !scored.iter().any(|(_, existing)| existing == &candidate) {
            scored.push((distance, candidate));
        }
    }

    // Distance first, then name, so the same misspelling always produces the
    // same list: a suggestion an agent can cache is worth more than one that
    // depends on registry iteration order.
    scored.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .take(MAX_CANDIDATES)
        .map(|(_, candidate)| candidate)
        .collect()
}

/// Full-width ASCII (U+FF01–U+FF5E, what a Japanese input method produces
/// for `ＡＤＤ`) folded to its ASCII form, so `ＬＥＮＧＴＨ` is one
/// substitution from nothing and `ＬＥＮＧＨＴ` is a typo of `LENGTH`. The
/// dictionary itself holds no full-width name, so nothing resolves
/// differently; only the suggestions do.
fn fold_full_width(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFF01 + 0x21).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// Levenshtein distance over `char`s when it is at most `ceiling`, else
/// `None`.
///
/// Only the band of cells within `ceiling` of the diagonal can hold a
/// distance that small, so only the band is computed: the work is
/// `len × (2·ceiling + 1)` rather than `len²`. A diagnosis runs outside every
/// meter, and the full table over a 32,000-character name and a User Word of
/// the same length took seconds.
fn edit_distance_within(left: &[char], right: &[char], ceiling: usize) -> Option<usize> {
    if left.len().abs_diff(right.len()) > ceiling {
        return None;
    }
    // Any distance past the ceiling is as good as infinite.
    let past = ceiling + 1;
    let mut previous: Vec<usize> = (0..=right.len()).map(|j| j.min(past)).collect();
    let mut current = vec![past; right.len() + 1];
    for (i, l) in left.iter().enumerate() {
        let row = i + 1;
        let low = row.saturating_sub(ceiling).max(1);
        let high = (row + ceiling).min(right.len());
        current[0] = row.min(past);
        current[low - 1] = if low == 1 { current[0] } else { past };
        let mut best = current[low - 1];
        for j in low..=high {
            let substitution = previous[j - 1] + usize::from(*l != right[j - 1]);
            let cell = substitution
                .min(previous[j] + 1)
                .min(current[j - 1] + 1)
                .min(past);
            current[j] = cell;
            best = best.min(cell);
        }
        if high < right.len() {
            current[high + 1] = past;
        }
        if best > ceiling {
            return None;
        }
        std::mem::swap(&mut previous, &mut current);
    }
    let distance = previous[right.len()];
    (distance <= ceiling).then_some(distance)
}

/// Levenshtein distance over `char`s, two rows at a time: the reference the
/// banded form above is checked against.
#[cfg(test)]
fn edit_distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    if left.is_empty() {
        return right.len();
    }
    if right.is_empty() {
        return left.len();
    }

    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0usize; right.len() + 1];
    for (i, l) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, r) in right.iter().enumerate() {
            let substitution = previous[j] + usize::from(l != r);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_one_character_slip_suggests_the_intended_coreword() {
        let candidates = suggest_words("LENGHT", std::iter::empty());
        assert!(
            candidates.contains(&"LENGTH".to_string()),
            "expected LENGTH among {candidates:?}"
        );
    }

    #[test]
    fn a_name_nothing_resembles_suggests_nothing() {
        assert!(suggest_words("FROBNICATE", std::iter::empty()).is_empty());
    }

    #[test]
    fn user_words_are_matched_beside_the_compiled_in_vocabulary() {
        let user = ["DOUBLE".to_string()];
        let candidates = suggest_words("DOUBEL", user.iter().map(String::as_str));
        assert_eq!(candidates, vec!["DOUBLE".to_string()]);
    }

    #[test]
    fn the_name_itself_is_never_offered_as_its_own_correction() {
        assert!(!suggest_words("MAP", std::iter::empty()).contains(&"MAP".to_string()));
    }

    proptest::proptest! {
        #[test]
        fn the_banded_distance_agrees_with_the_full_table(
            left in "[ABC]{0,9}",
            right in "[ABC]{0,9}",
            ceiling in 0usize..4,
        ) {
            let full = edit_distance(&left, &right);
            let left: Vec<char> = left.chars().collect();
            let right: Vec<char> = right.chars().collect();
            proptest::prop_assert_eq!(
                edit_distance_within(&left, &right, ceiling),
                (full <= ceiling).then_some(full)
            );
        }
    }

    /// The diagnosis runs outside every meter, so its work has to be bounded
    /// by the names, not by their product: two 32,000-character names took
    /// seconds through the full table.
    #[test]
    fn a_long_name_is_matched_in_linear_time() {
        let user = "A".repeat(32_000);
        let typo = format!("{}B", "A".repeat(31_999));
        let started = std::time::Instant::now();
        let candidates = suggest_words(&typo, std::iter::once(user.as_str()));
        assert_eq!(candidates, vec![user]);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn suggestions_are_capped_and_ordered_by_distance() {
        let candidates = suggest_words("MAPP", std::iter::empty());
        assert!(candidates.len() <= MAX_CANDIDATES);
        assert_eq!(candidates.first().map(String::as_str), Some("MAP"));
    }
}
