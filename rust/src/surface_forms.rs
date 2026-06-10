//! Surface-form metadata: the named, English-based concept behind every visible
//! *symbolic* form in Ajisai source.
//!
//! Ajisai source is word-based. Visible symbols are **surface forms** — aliases
//! or sugar for named, English-based canonical concepts. Crucially, not every
//! surface form is a runtime word: some are purely lexical (resolved by the
//! tokenizer), some are parser-level structural delimiters, and a few are
//! reserved markers that never appear as runtime tokens at all.
//!
//! This module classifies the lexical / structural / reserved surface forms that
//! are **not** runtime-canonicalizable words. The runtime *word* aliases
//! (`+` -> `ADD`, `.` -> `TOP`, `~` -> `FLOW`, ...) live in
//! [`crate::core_word_aliases`], which remains the single source of truth for
//! runtime name canonicalization. The two tables are deliberately kept separate:
//! [`crate::core_word_aliases::canonicalize_core_word_name`] must never map `#`,
//! `[`, `{`, `'`, `;`, ... onto the concept names defined here, because these
//! concepts are not runtime words.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceFormKind {
    /// Parser-level structural delimiter, e.g. `[` `]` `{` `}`.
    DelimiterSugar,
    /// String-literal delimiter, e.g. `'`.
    LiteralSugar,
    /// Compound stack-modifier shorthand, e.g. `;` (`. ,`) and `;;` (`.. ,,`).
    ModifierSugar,
    /// Source-level directive consumed by the tokenizer, e.g. `#`.
    SourceDirective,
    /// Control-flow directive meaningful only inside a construct, e.g. `$`.
    ControlDirective,
    /// Reserved marker that is never a runtime token, e.g. `(` `)`.
    ReservedMarker,
    /// Conversion-word lexical pattern: `>` immediately followed by letters,
    /// e.g. `>CF`. This is a *pattern*, not a fixed table entry, because the
    /// canonical home is a runtime word of the same name (see
    /// [`is_conversion_word_token`]).
    ConversionWord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceForm {
    /// The visible symbol as written in source.
    pub surface: &'static str,
    /// The named, English-based concept this surface form denotes.
    pub concept: &'static str,
    pub kind: SurfaceFormKind,
    /// Whether the concept is a runtime word. Every entry in [`SURFACE_FORMS`]
    /// is `false`: these are lexical / structural / reserved forms only.
    pub runtime_word: bool,
    pub summary: &'static str,
}

/// The lexical / structural / reserved surface forms.
///
/// Runtime word aliases are intentionally absent here; see
/// [`crate::core_word_aliases::CORE_WORD_ALIASES`]. The `>NAME` conversion-word
/// form is also absent because it is a pattern (see [`is_conversion_word_token`])
/// whose canonical home is a runtime word, not a fixed sugar entry.
pub const SURFACE_FORMS: &[SurfaceForm] = &[
    SurfaceForm {
        surface: "#",
        concept: "COMMENT-LINE",
        kind: SurfaceFormKind::SourceDirective,
        runtime_word: false,
        summary: "Line comment: characters from `#` to end of line are ignored",
    },
    SurfaceForm {
        surface: "$",
        concept: "COND-CLAUSE",
        kind: SurfaceFormKind::ControlDirective,
        runtime_word: false,
        summary: "COND clause separator (guard $ body)",
    },
    SurfaceForm {
        surface: "[",
        concept: "BEGIN-VECTOR",
        kind: SurfaceFormKind::DelimiterSugar,
        runtime_word: false,
        summary: "Vector start",
    },
    SurfaceForm {
        surface: "]",
        concept: "END-VECTOR",
        kind: SurfaceFormKind::DelimiterSugar,
        runtime_word: false,
        summary: "Vector end",
    },
    SurfaceForm {
        surface: "{",
        concept: "BEGIN-BLOCK",
        kind: SurfaceFormKind::DelimiterSugar,
        runtime_word: false,
        summary: "Code block start",
    },
    SurfaceForm {
        surface: "}",
        concept: "END-BLOCK",
        kind: SurfaceFormKind::DelimiterSugar,
        runtime_word: false,
        summary: "Code block end",
    },
    SurfaceForm {
        surface: "'",
        concept: "STRING-QUOTE",
        kind: SurfaceFormKind::LiteralSugar,
        runtime_word: false,
        summary: "String literal delimiter (serves as both open and close)",
    },
    SurfaceForm {
        surface: ";",
        concept: "TOP-EAT",
        kind: SurfaceFormKind::ModifierSugar,
        runtime_word: false,
        summary: "Shorthand for `. ,` (TOP EAT)",
    },
    SurfaceForm {
        surface: ";;",
        concept: "STAK-KEEP",
        kind: SurfaceFormKind::ModifierSugar,
        runtime_word: false,
        summary: "Shorthand for `.. ,,` (STAK KEEP)",
    },
    SurfaceForm {
        surface: "(",
        concept: "RESERVED-BEGIN",
        kind: SurfaceFormKind::ReservedMarker,
        runtime_word: false,
        summary: "Reserved (continued-fraction serialization); not valid in source",
    },
    SurfaceForm {
        surface: ")",
        concept: "RESERVED-END",
        kind: SurfaceFormKind::ReservedMarker,
        runtime_word: false,
        summary: "Reserved (continued-fraction serialization); not valid in source",
    },
];

/// Look up the surface-form metadata for a symbol.
pub fn lookup_surface_form(surface: &str) -> Option<&'static SurfaceForm> {
    SURFACE_FORMS.iter().find(|f| f.surface == surface)
}

/// Classify a `>NAME` conversion-word token (e.g. `>CF`).
///
/// `>` and `>=` are the `GT` / `GTE` comparison aliases (see
/// [`crate::core_word_aliases`]); `>` immediately followed by an ASCII letter is
/// instead a single conversion-word token whose canonical home is the runtime
/// word of the same name (e.g. the `>CF` continued-fraction conversion word).
pub fn is_conversion_word_token(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next() == Some('>') && matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
}

/// Human-readable label for a surface-form kind, for diagnostics.
pub fn surface_form_kind_label(kind: SurfaceFormKind) -> &'static str {
    match kind {
        SurfaceFormKind::DelimiterSugar => "delimiter sugar",
        SurfaceFormKind::LiteralSugar => "literal sugar",
        SurfaceFormKind::ModifierSugar => "modifier sugar",
        SurfaceFormKind::SourceDirective => "a source directive",
        SurfaceFormKind::ControlDirective => "control directive sugar",
        SurfaceFormKind::ReservedMarker => "a reserved marker",
        SurfaceFormKind::ConversionWord => "a conversion word",
    }
}

/// One-line diagnostic describing a surface form, e.g.
/// `'$' is control directive sugar for COND-CLAUSE.`
pub fn describe_surface_form(surface: &str) -> Option<String> {
    lookup_surface_form(surface).map(|f| {
        format!(
            "'{}' is {} for {}.",
            f.surface,
            surface_form_kind_label(f.kind),
            f.concept
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_word_aliases::canonicalize_core_word_name;

    #[test]
    fn lookup_returns_named_concepts() {
        assert_eq!(lookup_surface_form("#").unwrap().concept, "COMMENT-LINE");
        assert_eq!(lookup_surface_form("$").unwrap().concept, "COND-CLAUSE");
        assert_eq!(lookup_surface_form("[").unwrap().concept, "BEGIN-VECTOR");
        assert_eq!(lookup_surface_form("]").unwrap().concept, "END-VECTOR");
        assert_eq!(lookup_surface_form("{").unwrap().concept, "BEGIN-BLOCK");
        assert_eq!(lookup_surface_form("}").unwrap().concept, "END-BLOCK");
        assert_eq!(lookup_surface_form("'").unwrap().concept, "STRING-QUOTE");
        assert_eq!(lookup_surface_form(";").unwrap().concept, "TOP-EAT");
        assert_eq!(lookup_surface_form(";;").unwrap().concept, "STAK-KEEP");
        assert_eq!(lookup_surface_form("(").unwrap().concept, "RESERVED-BEGIN");
        assert_eq!(lookup_surface_form(")").unwrap().concept, "RESERVED-END");
    }

    #[test]
    fn unknown_surface_form_is_none() {
        assert!(lookup_surface_form("+").is_none());
        assert!(lookup_surface_form("ADD").is_none());
        assert!(lookup_surface_form("COND").is_none());
    }

    #[test]
    fn surface_forms_are_never_runtime_words() {
        assert!(SURFACE_FORMS.iter().all(|f| !f.runtime_word));
    }

    /// The two tables must stay disjoint: a lexical/structural surface form must
    /// never be canonicalized onto its concept name as if it were a runtime word.
    #[test]
    fn canonicalize_does_not_leak_surface_concepts() {
        assert_ne!(canonicalize_core_word_name("#"), "COMMENT-LINE");
        assert_ne!(canonicalize_core_word_name("["), "BEGIN-VECTOR");
        assert_ne!(canonicalize_core_word_name("{"), "BEGIN-BLOCK");
        assert_ne!(canonicalize_core_word_name("'"), "STRING-QUOTE");
        assert_ne!(canonicalize_core_word_name(";"), "TOP-EAT");
        assert_ne!(canonicalize_core_word_name(";;"), "STAK-KEEP");
        assert_ne!(canonicalize_core_word_name("$"), "COND-CLAUSE");
    }

    #[test]
    fn conversion_word_pattern() {
        assert!(is_conversion_word_token(">CF"));
        assert!(is_conversion_word_token(">Cf"));
        // `>` and `>=` are comparison aliases, not conversion words.
        assert!(!is_conversion_word_token(">"));
        assert!(!is_conversion_word_token(">="));
        assert!(!is_conversion_word_token(">2"));
    }

    #[test]
    fn describe_is_diagnostic_friendly() {
        assert_eq!(
            describe_surface_form("$").unwrap(),
            "'$' is control directive sugar for COND-CLAUSE."
        );
        assert_eq!(
            describe_surface_form("]").unwrap(),
            "']' is delimiter sugar for END-VECTOR."
        );
    }
}
