//! Surface-form metadata: the named, English-based concept behind every visible
//! source form that is *not* a runtime word — the symbolic ones, and the one
//! word-shaped keyword (`IDLE`) that is read positionally rather than looked
//! up.
//!
//! Ajisai source is word-based. Visible symbols are **surface forms** — aliases
//! or sugar for named, English-based canonical concepts. Crucially, not every
//! surface form is a runtime word: some are purely lexical (resolved by the
//! tokenizer), some are parser-level structural delimiters, and a few are
//! reserved markers that never appear as runtime tokens at all.
//!
//! This module classifies the lexical / structural / reserved surface forms that
//! are **not** runtime-canonicalizable words. The runtime *word* aliases
//! (`+` -> `ADD`, `^` -> `OR-NIL`, ...) live in [`crate::core_word_aliases`], which
//! remains the single source of truth for runtime name canonicalization. The two
//! tables are deliberately kept separate:
//! [`crate::core_word_aliases::canonicalize_core_word_name`] must never map `#`,
//! `[`, `{`, `'`, ... onto the concept names defined here, because these
//! concepts are not runtime words.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceFormKind {
    /// Parser-level structural delimiter, e.g. `[` `]` `{` `}`.
    DelimiterSugar,
    /// String-literal delimiter, e.g. `'`.
    LiteralSugar,
    /// Source-level directive consumed by the tokenizer, e.g. `#`.
    SourceDirective,
    /// Control-flow directive meaningful only inside a construct, e.g. `|`.
    ControlDirective,
    /// Reserved marker that is never a runtime token, e.g. `(` `)`.
    ReservedMarker,
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
/// [`crate::core_word_aliases::CORE_WORD_ALIASES`].
pub const SURFACE_FORMS: &[SurfaceForm] = &[
    SurfaceForm {
        surface: "#",
        concept: "COMMENT-LINE",
        kind: SurfaceFormKind::SourceDirective,
        runtime_word: false,
        summary: "Line comment: characters from `#` to end of line are ignored",
    },
    SurfaceForm {
        surface: "|",
        concept: "COND-CLAUSE",
        kind: SurfaceFormKind::ControlDirective,
        runtime_word: false,
        summary: "COND clause separator (guard | body)",
    },
    // `IDLE` is the else-guard: a clause whose guard is exactly this one name
    // fires when no earlier clause did. It is matched positionally by
    // `control_cond::is_idle_guard`, never resolved through the dictionary, so
    // it is not one of the Core Words and does not appear in the Dictionary
    // panel. It was also, until it was registered here, in no registry, no
    // reading surface, and no reference entry — a name a reader could only learn
    // from an example, in a language whose whole claim is that a small fixed
    // vocabulary is all there is. Registering it puts it in the manifest and
    // lets the reading surfaces name it.
    SurfaceForm {
        surface: "IDLE",
        concept: "COND-ELSE-GUARD",
        kind: SurfaceFormKind::ControlDirective,
        runtime_word: false,
        summary: "COND else-guard: fires when no earlier clause did",
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
        surface: "(",
        concept: "RESERVED-BEGIN",
        kind: SurfaceFormKind::ReservedMarker,
        runtime_word: false,
        summary: "Reserved; not valid in source ('[' ']' is the sole bracket, including for continued-fraction display)",
    },
    SurfaceForm {
        surface: ")",
        concept: "RESERVED-END",
        kind: SurfaceFormKind::ReservedMarker,
        runtime_word: false,
        summary: "Reserved; not valid in source ('[' ']' is the sole bracket, including for continued-fraction display)",
    },
];

/// Look up the surface-form metadata for a symbol.
pub fn lookup_surface_form(surface: &str) -> Option<&'static SurfaceForm> {
    SURFACE_FORMS.iter().find(|f| f.surface == surface)
}

/// Human-readable label for a surface-form kind, for diagnostics.
pub fn surface_form_kind_label(kind: SurfaceFormKind) -> &'static str {
    match kind {
        SurfaceFormKind::DelimiterSugar => "delimiter sugar",
        SurfaceFormKind::LiteralSugar => "literal sugar",
        SurfaceFormKind::SourceDirective => "a source directive",
        SurfaceFormKind::ControlDirective => "control directive sugar",
        SurfaceFormKind::ReservedMarker => "a reserved marker",
    }
}

/// One-line diagnostic describing a surface form, e.g.
/// `'|' is control directive sugar for COND-CLAUSE.`
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
        assert_eq!(lookup_surface_form("|").unwrap().concept, "COND-CLAUSE");
        assert_eq!(lookup_surface_form("[").unwrap().concept, "BEGIN-VECTOR");
        assert_eq!(lookup_surface_form("]").unwrap().concept, "END-VECTOR");
        assert_eq!(lookup_surface_form("{").unwrap().concept, "BEGIN-BLOCK");
        assert_eq!(lookup_surface_form("}").unwrap().concept, "END-BLOCK");
        assert_eq!(lookup_surface_form("'").unwrap().concept, "STRING-QUOTE");
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
        assert_ne!(canonicalize_core_word_name("|"), "COND-CLAUSE");
    }

    #[test]
    fn describe_is_diagnostic_friendly() {
        assert_eq!(
            describe_surface_form("|").unwrap(),
            "'|' is control directive sugar for COND-CLAUSE."
        );
        assert_eq!(
            describe_surface_form("]").unwrap(),
            "']' is delimiter sugar for END-VECTOR."
        );
    }
}
