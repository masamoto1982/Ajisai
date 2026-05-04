use super::builtin_word_definitions::lookup_builtin_spec;

pub fn lookup_builtin_detail(name: &str) -> String {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    let canonical_label = lookup_builtin_spec(&canonical)
        .map(|spec| spec.name.to_string())
        .unwrap_or_else(|| canonical.clone());

    let alias_lead = crate::core_word_aliases::lookup_core_word_alias(name)
        .and_then(|alias| {
            alias.canonical.map(|canonical_name| match alias.kind {
                crate::core_word_aliases::CoreWordAliasKind::SymbolAlias => {
                    format!("{} is an alias of {}.\n\n", alias.alias, canonical_name)
                }
                crate::core_word_aliases::CoreWordAliasKind::SyntaxSugar => {
                    format!(
                        "{} is syntax sugar for {}.\n\n",
                        alias.alias, canonical_name
                    )
                }
                crate::core_word_aliases::CoreWordAliasKind::InputHelper => {
                    format!("{} is an input helper.\n\n", alias.alias)
                }
            })
        })
        .unwrap_or_default();

    format!(
        "{}# {}\n\nThis is where the description of the specified built-in word will be provided. The current text is a placeholder pending a full rewrite of the built-in word documentation.\n",
        alias_lead, canonical_label
    )
}
