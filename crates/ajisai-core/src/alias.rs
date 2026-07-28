//! The alias table.
//!
//! Symbol notation is a first-class surface of Ajisai, not a compatibility
//! shim and not a deprecated spelling. `1 2 +` and `1 2 ADD` are the same
//! program: the parser normalizes the symbol to the canonical word before any
//! other layer sees it, so the evaluator, the contract registry, the lint, the
//! formatter, and the error messages all deal in one name. There is exactly
//! one table, here, and every other layer reads it.
//!
//! Because normalization happens in the parser, semantic equivalence between a
//! symbol and its canonical word is structural rather than a property that has
//! to be re-established word by word. `tests/aliases.rs` still checks it end
//! to end for every entry.

/// Every alias in the language, symbol to canonical word.
pub const ALIASES: &[(&str, &str)] = &[
    // Arithmetic
    ("+", "ADD"),
    ("-", "SUB"),
    ("*", "MUL"),
    ("/", "DIV"),
    // Comparison
    ("=", "EQ"),
    ("<", "LT"),
    (">", "GT"),
    // Flow selection: where in the basin the next word draws from.
    (".", "TOP"),
    (":", "STAK"),
    // Flow retention: whether the next word swallows what it drew.
    ("!", "EAT"),
    ("&", "KEEP"),
    // The vent.
    ("^", "VENT"),
];

/// Normalize a source token to its canonical word name.
///
/// Aliases match the whole token exactly, so `-` is `SUB` while `-3` is a
/// literal and `>TEXT` is its own word rather than `GT` followed by `TEXT`.
///
/// **Case folding is ASCII-only, and no other normalization is applied.**
/// `add`, `Add`, and `ADD` are one word; `やる` is itself. This is deliberate.
/// Full Unicode case conversion is defined against a particular Unicode
/// version, is locale-sensitive in places, and can change a string's length —
/// so it would make word identity depend on which Unicode table an
/// implementation was built with, and would silently differ across
/// implementations of the same specification. ASCII case mapping has been
/// fixed forever.
///
/// Source is required to be in Normalization Form C (`SPECIFICATION.md` §2.4);
/// Ajisai does not normalize it, so two names that differ only in
/// decomposition are two names.
pub fn canonical(token: &str) -> String {
    for (symbol, word) in ALIASES {
        if token == *symbol {
            return (*word).to_string();
        }
    }
    token.to_ascii_uppercase()
}

/// The aliases that point at `word`, in table order.
pub fn aliases_for(word: &str) -> Vec<&'static str> {
    ALIASES
        .iter()
        .filter(|(_, canonical)| *canonical == word)
        .map(|(symbol, _)| *symbol)
        .collect()
}
