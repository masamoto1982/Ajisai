//! "Did you mean" for an unrecognized Word name.
//!
//! The unknown-name diagnosis has always told a reader to check the spelling
//! without saying what the spelling might have been — the one next-check an
//! agent cannot act on, even though the entire vocabulary it would need is
//! compiled into the same binary. This module answers it: the Corewords, their
//! aliases, and whatever names the failing interpreter additionally knows,
//! ranked by edit distance from the name that did not resolve.
//!
//! Deliberately conservative. A suggestion that is not a plausible typo is
//! worse than none — it sends a repair attempt at a Word the author never
//! meant — so the distance ceiling scales with the name's length and only the
//! closest few survive.

use crate::core_word_aliases::CORE_WORD_ALIASES;
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
    let needle = name.trim().to_uppercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let ceiling = distance_ceiling(needle.chars().count());

    let vocabulary = get_builtin_word_registry()
        .iter()
        .map(|entry| entry.name.to_string())
        .chain(
            CORE_WORD_ALIASES
                .iter()
                .map(|alias| alias.alias.to_string()),
        )
        .chain(extra.map(|word| word.to_string()));

    let mut scored: Vec<(usize, String)> = Vec::new();
    for candidate in vocabulary {
        let upper = candidate.to_uppercase();
        if upper == needle {
            // The name resolves after all — a caller asking about it wants a
            // different diagnosis than a spelling hint.
            continue;
        }
        // A symbolic alias (`+`, `^`) is never a typo of an alphabetic name;
        // its edit distance is small only because it is short.
        if !upper.chars().any(|c| c.is_alphanumeric()) {
            continue;
        }
        let distance = edit_distance(&needle, &upper);
        if distance <= ceiling && !scored.iter().any(|(_, existing)| existing == &candidate) {
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

/// Levenshtein distance over `char`s, two rows at a time.
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
        let user = ["EXAMPLE@DOUBLE".to_string()];
        let candidates = suggest_words("EXAMPLE@DOUBEL", user.iter().map(String::as_str));
        assert_eq!(candidates, vec!["EXAMPLE@DOUBLE".to_string()]);
    }

    #[test]
    fn the_name_itself_is_never_offered_as_its_own_correction() {
        assert!(!suggest_words("MAP", std::iter::empty()).contains(&"MAP".to_string()));
    }

    #[test]
    fn suggestions_are_capped_and_ordered_by_distance() {
        let candidates = suggest_words("MAPP", std::iter::empty());
        assert!(candidates.len() <= MAX_CANDIDATES);
        assert_eq!(candidates.first().map(String::as_str), Some("MAP"));
    }
}
