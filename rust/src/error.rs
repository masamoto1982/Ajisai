//! Three-layer error model.
//!
//! Errors carry three distinct messages:
//!  * `summary`   — single short line ("WHEN ... WHY").
//!  * `detail`   — explanation suitable for an experienced human reader.
//!  * `diagnosis` — structured next-check guidance for an AI agent.
//!
//! Layered presentation lets the GUI surface a one-liner while still exposing
//! the deeper context for tooling and AI assistance.

#[derive(Clone, Debug)]
pub struct AjisaiError {
    pub summary: String,
    pub detail: String,
    pub diagnosis: String,
}

impl AjisaiError {
    pub fn new(
        summary: impl Into<String>,
        detail: impl Into<String>,
        diagnosis: impl Into<String>,
    ) -> Self {
        Self {
            summary: summary.into(),
            detail: detail.into(),
            diagnosis: diagnosis.into(),
        }
    }

    pub fn stack_underflow(word: &str, need: usize, have: usize) -> Self {
        Self::new(
            format!("Stack underflow at {}", word),
            format!(
                "Word {} requires {} value(s) on the stack but only {} was/were available.",
                word, need, have
            ),
            format!(
                "Check the operands feeding {}: push {} more value(s) before invoking it, or remove the invocation.",
                word,
                need.saturating_sub(have)
            ),
        )
    }

    pub fn unknown_word(name: &str) -> Self {
        Self::new(
            format!("Unknown word `{}`", name),
            format!(
                "No core word, user word, or numeric literal matches `{}` in the current scope.",
                name
            ),
            format!(
                "Verify spelling and capitalisation of `{}`. Core words are upper-case; user words are defined via DEF.",
                name
            ),
        )
    }

    pub fn parse_error(token: &str) -> Self {
        Self::new(
            format!("Cannot parse token `{}`", token),
            format!(
                "The lexer recognised `{}` as a candidate value but it does not match any supported numeric or symbolic form.",
                token
            ),
            "Confirm the token matches one of: integer (`42`), fraction (`3/4`), decimal (`3.14`), or a defined word.".to_string(),
        )
    }
}
