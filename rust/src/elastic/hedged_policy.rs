use crate::elastic::purity_table::{purity_by_name, Purity};

const DENY_WORDS: &[&str] = &[
    "PRINT",
    "INPUT",
    "IMPORT",
    "RESTORE-MODULE",
    "RESTORE-IMPORTS",
    "NOW",
    "RAND",
];

const HOF_ALLOWLIST: &[&str] = &["MAP", "FILTER", "ANY", "ALL", "COUNT", "FOLD", "SCAN"];

/// Returns `true` only when `word` is a known-pure builtin.
///
/// Conservatively denies unknown words and words with `Unknown` or `Impure`
/// purity.  This is an allowlist policy: anything not explicitly proven pure
/// is rejected.
pub fn can_hedge_word(word: &str) -> bool {
    let upper = word.trim().to_ascii_uppercase();
    if DENY_WORDS.contains(&upper.as_str()) {
        return false;
    }
    match purity_by_name(&upper) {
        Some(info) => info.purity == Purity::Pure,
        None => false,
    }
}

/// Returns `true` when every token in `tokens` is hedgeable.
/// An empty block is not hedgeable (vacuous truth is not safe here).
pub fn can_hedge_code_block(tokens: &[String]) -> bool {
    !tokens.is_empty() && tokens.iter().all(|token| can_hedge_word(token))
}

pub fn can_hedge_cond_guard(tokens: &[String]) -> bool {
    can_hedge_code_block(tokens)
}

pub fn can_hedge_hof_kernel(word: &str) -> bool {
    let upper = word.trim().to_ascii_uppercase();
    HOF_ALLOWLIST.contains(&upper.as_str())
}
