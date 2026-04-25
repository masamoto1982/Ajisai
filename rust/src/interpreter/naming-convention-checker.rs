pub(crate) fn check_reserved_word_name(name: &str) -> Option<String> {
    if let Some(alias) = crate::core_word_aliases::lookup_core_word_alias(name) {
        return match alias.kind {
            crate::core_word_aliases::CoreWordAliasKind::SymbolAlias => Some(format!(
                "Cannot define '{}': '{}' is reserved as an alias of {}.",
                name,
                name,
                alias.canonical.unwrap_or("")
            )),
            crate::core_word_aliases::CoreWordAliasKind::SyntaxSugar => Some(format!(
                "Cannot define '{}': '{}' is reserved as syntax sugar for {}.",
                name,
                name,
                alias.canonical.unwrap_or("")
            )),
            crate::core_word_aliases::CoreWordAliasKind::InputHelper => Some(format!(
                "Cannot define '{}': '{}' is an input helper, not a user word name.",
                name, name
            )),
            crate::core_word_aliases::CoreWordAliasKind::Deprecated => {
                Some(format!("Cannot define '{}': '{}' is reserved.", name, name))
            }
        };
    }

    if name == "$" {
        return Some("Cannot define '$': '$' is tokenizer-level syntax.".to_string());
    }

    None
}

const AMBIGUOUS_PREFIXES: &[&str] = &["DO-", "HANDLE-", "PROCESS-", "MANAGE-", "UTIL-", "HELPER-"];

const AMBIGUOUS_NAMES: &[&str] = &[
    "CALC", "RUN", "EXEC2", "TEMP", "MAIN", "TEST", "STUFF", "THING",
];

const SHORT_NAME_MAX_LENGTH: usize = 6;

pub(crate) fn check_word_name_convention(name: &str) -> Option<String> {
    let upper = name.to_uppercase();

    for prefix in AMBIGUOUS_PREFIXES {
        if upper.starts_with(prefix) {
            let verb = prefix.trim_end_matches('-');
            return Some(format!(
                "Warning: '{}': naming convention violation: ambiguous verb '{}'. \
                 Consider: 'APPLY-...' or 'RESOLVE-...'\n  \
                 See: §DEV-NAMING-INDEX",
                upper, verb
            ));
        }
    }

    if AMBIGUOUS_NAMES.contains(&upper.as_str()) {
        return Some(format!(
            "Warning: '{}': naming convention violation: ambiguous word name. \
             Consider a more specific action_object name.\n  \
             See: §DEV-NAMING-INDEX",
            upper
        ));
    }

    if !upper.contains('-') && upper.len() <= SHORT_NAME_MAX_LENGTH {
        return None;
    }

    if upper.starts_with("IS-") || upper.starts_with("HAS-") {
        return None;
    }

    if upper.contains('-') {
        return None;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accept_action_object_pattern() {
        assert!(check_word_name_convention("PARSE-TOKEN").is_none());
        assert!(check_word_name_convention("SORT-VALUES").is_none());
        assert!(check_word_name_convention("COMPUTE-TOTAL").is_none());
    }

    #[test]
    fn test_accept_action_object_in_context() {
        assert!(check_word_name_convention("LOOKUP-HINT-IN-DICT").is_none());
    }

    #[test]
    fn test_accept_action_source_to_target() {
        assert!(check_word_name_convention("CONVERT-VALUE-TO-STR").is_none());
    }

    #[test]
    fn test_accept_is_condition() {
        assert!(check_word_name_convention("IS-EVEN").is_none());
        assert!(check_word_name_convention("IS-NIL").is_none());
        assert!(check_word_name_convention("IS-EMPTY").is_none());
    }

    #[test]
    fn test_accept_has_property() {
        assert!(check_word_name_convention("HAS-ITEMS").is_none());
    }

    #[test]
    fn test_accept_short_clear_names() {
        assert!(check_word_name_convention("GREET").is_none());
        assert!(check_word_name_convention("DOUBLE").is_none());
        assert!(check_word_name_convention("TRIPLE").is_none());
    }

    #[test]
    fn test_warn_ambiguous_verb_prefix() {
        let result = check_word_name_convention("DO-CALC");
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(msg.contains("ambiguous verb 'DO'"));
        assert!(msg.contains("§DEV-NAMING-INDEX"));

        assert!(check_word_name_convention("HANDLE-INPUT").is_some());
        assert!(check_word_name_convention("PROCESS-DATA").is_some());
        assert!(check_word_name_convention("MANAGE-STATE").is_some());
        assert!(check_word_name_convention("UTIL-FORMAT").is_some());
        assert!(check_word_name_convention("HELPER-SORT").is_some());
    }

    #[test]
    fn test_warn_ambiguous_standalone_names() {
        let result = check_word_name_convention("CALC");
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(msg.contains("ambiguous word name"));

        assert!(check_word_name_convention("RUN").is_some());
        assert!(check_word_name_convention("EXEC2").is_some());
        assert!(check_word_name_convention("TEMP").is_some());
        assert!(check_word_name_convention("MAIN").is_some());
        assert!(check_word_name_convention("TEST").is_some());
        assert!(check_word_name_convention("STUFF").is_some());
        assert!(check_word_name_convention("THING").is_some());
    }

    #[test]
    fn test_case_insensitive() {
        assert!(check_word_name_convention("do-calc").is_some());
        assert!(check_word_name_convention("calc").is_some());
        assert!(check_word_name_convention("is-even").is_none());
    }
}
