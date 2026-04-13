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

pub fn can_hedge_word(word: &str) -> bool {
    let upper = word.trim().to_ascii_uppercase();
    if DENY_WORDS.contains(&upper.as_str()) {
        return false;
    }
    true
}

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
