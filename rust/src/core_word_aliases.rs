#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreWordAliasKind {
    SymbolAlias,
    SyntaxSugar,
    InputHelper,
    Deprecated,
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
        alias: "~",
        canonical: Some("SAFE"),
        kind: CoreWordAliasKind::SyntaxSugar,
        summary: "Enable safe mode",
    },
    CoreWordAlias {
        alias: "'",
        canonical: None,
        kind: CoreWordAliasKind::InputHelper,
        summary: "Insert quoted word marker",
    },
];

pub fn lookup_core_word_alias(alias: &str) -> Option<&'static CoreWordAlias> {
    CORE_WORD_ALIASES.iter().find(|entry| entry.alias == alias)
}

pub fn canonicalize_core_word_name(name: &str) -> String {
    if let Some(alias) = lookup_core_word_alias(name) {
        if let Some(canonical) = alias.canonical {
            return canonical.to_string();
        }
    }

    name.to_uppercase()
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
                CoreWordAliasKind::Deprecated => "deprecated",
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
