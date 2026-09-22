//! Surface-form metadata: the named, English-based concept behind every visible
//! source form that is *not* a runtime word.
//!
//! Ajisai source is word-based. Visible symbols are **surface forms** — aliases
//! or sugar for named, English-based canonical concepts. Crucially, not every
//! surface form is a runtime word: some are purely lexical (resolved by the
//! tokenizer) and some are parser-level structural delimiters.
//!
//! Every form listed here is live. There is no entry for a character the
//! tokenizer refuses, because it refuses none: `(`, `)` and a bare `|` were
//! once carried here as reserved markers and retired forms, and are now
//! ordinary name characters with no per-character rule of their own
//! (`spec/grammar.json`, characterClasses.nameCharacter). A form earns a place
//! in this table by *doing* something the word rule does not — which is what
//! `{` and `}` came back for: they were freed with those three and then
//! allocated as the Record literal's delimiters (LANG.RECORDS.STRUCTURE),
//! the second of the grammar's two delimiter pairs.
//!
//! This module classifies the lexical / structural surface forms that
//! are **not** runtime-canonicalizable words. The runtime *word* aliases
//! (`+` -> `ADD`, `<=` -> `LTE`, ...) live in [`crate::core_word_aliases`], which
//! remains the single source of truth for runtime name canonicalization. The two
//! tables are deliberately kept separate:
//! [`crate::core_word_aliases::canonicalize_core_word_name`] must never map `#`,
//! `[`, `{`, `'`, ... onto the concept names defined here, because these
//! concepts are not runtime words.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceFormKind {
    /// Parser-level structural delimiter, e.g. `[` `]`.
    DelimiterSugar,
    /// String-literal delimiter, e.g. `'`.
    LiteralSugar,
    /// Source-level directive consumed by the tokenizer, e.g. `#`.
    SourceDirective,
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
}

/// The lexical / structural surface forms.
///
/// Runtime word aliases are intentionally absent here; see
/// [`crate::core_word_aliases::CORE_WORD_ALIASES`].
pub const SURFACE_FORMS: &[SurfaceForm] = &[
    SurfaceForm {
        surface: "#",
        concept: "COMMENT-LINE",
        kind: SurfaceFormKind::SourceDirective,
        runtime_word: false,
        // Line comment: characters from `#` to end of line are ignored
    },
    SurfaceForm {
        surface: "[",
        concept: "BEGIN-VECTOR",
        kind: SurfaceFormKind::DelimiterSugar,
        runtime_word: false,
        // Vector start
    },
    SurfaceForm {
        surface: "]",
        concept: "END-VECTOR",
        kind: SurfaceFormKind::DelimiterSugar,
        runtime_word: false,
        // Vector end
    },
    SurfaceForm {
        surface: "{",
        concept: "BEGIN-RECORD",
        kind: SurfaceFormKind::DelimiterSugar,
        runtime_word: false,
        // Record literal start
    },
    SurfaceForm {
        surface: "}",
        concept: "END-RECORD",
        kind: SurfaceFormKind::DelimiterSugar,
        runtime_word: false,
        // Record literal end
    },
    SurfaceForm {
        surface: "'",
        concept: "STRING-QUOTE",
        kind: SurfaceFormKind::LiteralSugar,
        runtime_word: false,
        // String literal delimiter (serves as both open and close)
    },
];

/// Look up the surface-form metadata for a symbol.
pub fn lookup_surface_form(surface: &str) -> Option<&'static SurfaceForm> {
    SURFACE_FORMS.iter().find(|f| f.surface == surface)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_word_aliases::canonicalize_core_word_name;

    /// The characters that used to be carried here as reserved markers and
    /// retired forms, and are now ordinary name characters. `{` and `}` were
    /// freed with them and are back in the table as the Record literal's
    /// delimiters, so they are not among them.
    const FREED: [&str; 3] = ["(", ")", "|"];

    #[test]
    fn lookup_returns_named_concepts() {
        assert_eq!(lookup_surface_form("#").unwrap().concept, "COMMENT-LINE");
        assert_eq!(lookup_surface_form("[").unwrap().concept, "BEGIN-VECTOR");
        assert_eq!(lookup_surface_form("]").unwrap().concept, "END-VECTOR");
        assert_eq!(lookup_surface_form("{").unwrap().concept, "BEGIN-RECORD");
        assert_eq!(lookup_surface_form("}").unwrap().concept, "END-RECORD");
        assert_eq!(lookup_surface_form("'").unwrap().concept, "STRING-QUOTE");
    }

    /// A form's classification must agree with what the tokenizer does to it.
    ///
    /// This is the gate for the defect that put `{` and `}` in SKILL.md §9 as
    /// usable "delimiter sugar" while §2 of the same generated document called
    /// them invalid source characters. The manifest and every reading surface
    /// are generated from `SURFACE_FORMS`, so a wrong `kind` here propagates
    /// silently into the documents a learner and an agent both read — and
    /// nothing compared it against the runtime that has the final say.
    ///
    /// Deliberately behavioural rather than a string match between two sections
    /// of the generated prose: comparing §2's wording with §9's table would pin
    /// the sentences, and the thing that must not drift is the classification
    /// against the tokenizer.
    ///
    /// Every entry is now a live form, so the whole table is held to the live
    /// reading. A bare `[` is an unclosed vector and a bare `'` an unterminated
    /// string, so what is pinned is that the refusal is about *context* — the
    /// form is one the tokenizer knows — not that the character is invalid.
    #[test]
    fn a_forms_kind_agrees_with_what_the_tokenizer_accepts() {
        for form in SURFACE_FORMS {
            if let Err(message) = crate::tokenizer::tokenize(form.surface) {
                assert!(
                    !message.contains("not a valid Ajisai source character")
                        && !message.contains("is not a valid token"),
                    "'{}' is classified {:?} — a live form — but the tokenizer \
                     rejects the character itself: {message}",
                    form.surface,
                    form.kind
                );
            }
        }
    }

    /// The replacement for the retired-form gate this table used to carry.
    ///
    /// The table is generated into the word manifest, SKILL.md and the
    /// quickstart, so an entry here is a claim that the character does
    /// something the word rule does not. `(`, `)` and `|` no longer do: they
    /// lex as ordinary Symbols. Re-adding one would put a dead concept name
    /// back into every generated reading surface, which is the same defect as
    /// before with the sign flipped — the test that a form in the table is
    /// live (above) is what tells that apart from allocating a character, as
    /// `{` and `}` were allocated for the Record literal.
    #[test]
    fn a_freed_character_is_an_ordinary_name_and_is_not_listed() {
        for surface in FREED {
            assert!(
                lookup_surface_form(surface).is_none(),
                "'{surface}' is an ordinary name character and must not be \
                 carried as a surface form"
            );

            let tokens = crate::tokenizer::tokenize(surface)
                .unwrap_or_else(|e| panic!("'{surface}' should lex as a name, got: {e}"));
            assert_eq!(
                tokens.len(),
                1,
                "'{surface}' should be exactly one token, got {tokens:?}"
            );
            assert!(
                matches!(&tokens[0], crate::types::Token::Symbol(name) if name.as_ref() == surface),
                "'{surface}' should be a Symbol carrying its own lexeme, got {:?}",
                tokens[0]
            );
        }
    }

    /// A freed character is ordinary *inside* a word too, not merely on its
    /// own: the rule that refused them was a per-character one, so this is the
    /// half of its removal that the single-character cases above cannot see.
    #[test]
    fn a_freed_character_is_ordinary_inside_a_word() {
        for name in ["f(x)", "a;b", "x|y", "(", ")"] {
            let tokens = crate::tokenizer::tokenize(name)
                .unwrap_or_else(|e| panic!("`{name}` should lex as a name, got: {e}"));
            assert!(
                matches!(&tokens[..], [crate::types::Token::Symbol(value)] if value.as_ref() == name),
                "`{name}` should be one Symbol, got {tokens:?}"
            );
        }
    }

    #[test]
    fn unknown_surface_form_is_none() {
        assert!(lookup_surface_form("+").is_none());
        assert!(lookup_surface_form("ADD").is_none());
        assert!(lookup_surface_form("SELECT").is_none());
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
        assert_ne!(canonicalize_core_word_name("]"), "END-VECTOR");
        assert_ne!(canonicalize_core_word_name("'"), "STRING-QUOTE");
    }
}
