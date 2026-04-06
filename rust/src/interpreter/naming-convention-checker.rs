/// Naming convention checker for custom word definitions.
/// Validates that user-defined word names follow the AI-first naming convention (§DEV-NAMING-INDEX).
/// Produces warnings (not errors) for violations, guiding users toward clearer names.

const AMBIGUOUS_PREFIXES: &[&str] = &["DO-", "HANDLE-", "PROCESS-", "MANAGE-", "UTIL-", "HELPER-"];

const AMBIGUOUS_NAMES: &[&str] = &[
    "CALC", "RUN", "EXEC2", "TEMP", "MAIN", "TEST", "STUFF", "THING",
];

/// Maximum length for short, unstructured names that are considered acceptable
/// without requiring an action-object pattern (e.g., GREET, DOUBLE).
const SHORT_NAME_MAX_LENGTH: usize = 6;

/// Checks if a custom word name follows the AI-first naming convention.
/// Returns Some(warning_message) if a violation is detected, None if compliant.
pub(crate) fn check_word_name_convention(name: &str) -> Option<String> {
    let upper = name.to_uppercase();

    // Check ambiguous verb prefixes
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

    // Check ambiguous standalone names
    if AMBIGUOUS_NAMES.contains(&upper.as_str()) {
        return Some(format!(
            "Warning: '{}': naming convention violation: ambiguous word name. \
             Consider a more specific action_object name.\n  \
             See: §DEV-NAMING-INDEX",
            upper
        ));
    }

    // Short, clear names without hyphens are acceptable
    if !upper.contains('-') && upper.len() <= SHORT_NAME_MAX_LENGTH {
        return None;
    }

    // Accepted structured patterns: ACTION-OBJECT, IS-*, HAS-*
    if upper.starts_with("IS-") || upper.starts_with("HAS-") {
        return None;
    }

    // Any hyphenated name that passed the ambiguous prefix check is accepted
    if upper.contains('-') {
        return None;
    }

    // Long single-word names (> SHORT_NAME_MAX_LENGTH) without structure
    // are not flagged — user may have a valid reason
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
