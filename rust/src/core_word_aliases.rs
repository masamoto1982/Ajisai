#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreWordAliasKind {
    SymbolAlias,
    SyntaxSugar,
    InputHelper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoreWordAlias {
    pub alias: &'static str,
    pub canonical: Option<&'static str>,
    pub kind: CoreWordAliasKind,
    pub summary: &'static str,
}

pub const CORE_WORD_ALIASES: &[CoreWordAlias] = &[
    CoreWordAlias {
        alias: "+",
        canonical: Some("ADD"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Add values",
    },
    CoreWordAlias {
        alias: "-",
        canonical: Some("SUB"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Subtract values",
    },
    CoreWordAlias {
        alias: "*",
        canonical: Some("MUL"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Multiply values",
    },
    CoreWordAlias {
        alias: "/",
        canonical: Some("DIV"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Divide values",
    },
    CoreWordAlias {
        alias: "%",
        canonical: Some("MOD"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Modulo",
    },
    CoreWordAlias {
        alias: "=",
        canonical: Some("EQ"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Compare equality",
    },
    CoreWordAlias {
        alias: "<",
        canonical: Some("LT"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Compare less-than",
    },
    CoreWordAlias {
        alias: "<=",
        canonical: Some("LTE"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Compare less-than-or-equal",
    },
    CoreWordAlias {
        alias: ">",
        canonical: Some("GT"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Compare greater-than",
    },
    CoreWordAlias {
        alias: ">=",
        canonical: Some("GTE"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Compare greater-than-or-equal",
    },
    CoreWordAlias {
        alias: "<>",
        canonical: Some("NEQ"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Compare inequality",
    },
    CoreWordAlias {
        alias: "!",
        canonical: Some("FORC"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Force destructive dictionary operations",
    },
    CoreWordAlias {
        alias: "&",
        canonical: Some("AND"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Logical AND alias",
    },
    CoreWordAlias {
        alias: ".",
        canonical: Some("TOP"),
        kind: CoreWordAliasKind::SyntaxSugar,
        summary: "Target stack top",
    },
    CoreWordAlias {
        alias: "..",
        canonical: Some("STAK"),
        kind: CoreWordAliasKind::SyntaxSugar,
        summary: "Target whole stack",
    },
    CoreWordAlias {
        alias: ",",
        canonical: Some("EAT"),
        kind: CoreWordAliasKind::SyntaxSugar,
        summary: "Consume operands",
    },
    CoreWordAlias {
        alias: ",,",
        canonical: Some("KEEP"),
        kind: CoreWordAliasKind::SyntaxSugar,
        summary: "Keep operands",
    },
    CoreWordAlias {
        alias: "'",
        canonical: None,
        kind: CoreWordAliasKind::InputHelper,
        summary: "Insert quoted word marker",
    },
    CoreWordAlias {
        alias: "?",
        canonical: Some("LOOKUP"),
        kind: CoreWordAliasKind::SymbolAlias,
        summary: "Look up and display word definition",
    },
    CoreWordAlias {
        alias: "~",
        canonical: Some("FLOW"),
        kind: CoreWordAliasKind::SyntaxSugar,
        summary: "Pipeline visual marker (no-op)",
    },
    CoreWordAlias {
        alias: "^",
        canonical: Some("VENT"),
        kind: CoreWordAliasKind::SyntaxSugar,
        summary: "NIL coalescing",
    },
];

pub fn lookup_core_word_alias(alias: &str) -> Option<&'static CoreWordAlias> {
    CORE_WORD_ALIASES.iter().find(|entry| entry.alias == alias)
}

/// Canonicalize a surface word name to its dictionary key, allocating only when
/// it is actually required (handoff 手3 — dispatch de-allocation).
///
/// This is called on every word dispatch, so the previous unconditional
/// `String` allocation was pure overhead for the two dominant cases:
/// * a symbol alias (`+`, `<=`, `,,`) maps to a `&'static str` canonical name —
///   returned as `Cow::Borrowed` with zero allocation;
/// * an already-uppercase ASCII word (`MAP`, `LENGTH`, most user words) is its
///   own canonical form, so the input slice is borrowed unchanged.
///
/// Only a name that genuinely needs case folding (contains an ASCII lowercase
/// letter, or any non-ASCII char where Unicode upcasing may not be identity)
/// takes the owned `to_uppercase()` path — identical to the old behavior. The
/// borrow fast-path is gated on `is_ascii()` precisely so it never diverges from
/// Unicode `to_uppercase` for exotic input.
pub fn canonicalize_core_word_name(name: &str) -> std::borrow::Cow<'_, str> {
    if let Some(alias) = lookup_core_word_alias(name) {
        if let Some(canonical) = alias.canonical {
            return std::borrow::Cow::Borrowed(canonical);
        }
    }

    if name.is_ascii() && !name.bytes().any(|b| b.is_ascii_lowercase()) {
        return std::borrow::Cow::Borrowed(name);
    }

    std::borrow::Cow::Owned(name.to_uppercase())
}

pub fn is_reserved_core_word_alias(name: &str) -> bool {
    CORE_WORD_ALIASES.iter().any(|entry| entry.alias == name)
}

pub fn collect_core_word_aliases() -> Vec<(&'static str, &'static str, &'static str, &'static str)>
{
    CORE_WORD_ALIASES
        .iter()
        .filter_map(|a| {
            let canonical = a.canonical?;
            let kind = match a.kind {
                CoreWordAliasKind::SymbolAlias => "symbol_alias",
                CoreWordAliasKind::SyntaxSugar => "syntax_sugar",
                CoreWordAliasKind::InputHelper => "input_helper",
            };
            Some((a.alias, canonical, kind, a.summary))
        })
        .collect()
}

pub fn collect_input_helper_words() -> Vec<(&'static str, &'static str)> {
    CORE_WORD_ALIASES
        .iter()
        .filter(|a| matches!(a.kind, CoreWordAliasKind::InputHelper))
        .map(|a| (a.alias, a.summary))
        .collect()
}
