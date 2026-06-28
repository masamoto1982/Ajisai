use super::builtin_word_definitions::lookup_builtin_spec;
use crate::core_word_aliases::{lookup_core_word_alias, CoreWordAliasKind};

/// Render the LOOKUP body for a built-in word using the four-section
/// template (Category / Summary / Role / Stack Effect). Stability is shown
/// in parentheses next to the header.
pub fn lookup_builtin_detail(name: &str) -> String {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    let alias_lead = build_alias_lead(name);

    let Some(spec) = lookup_builtin_spec(&canonical) else {
        if let Some(body) = crate::interpreter::modules::lookup_module_word_detail(&canonical) {
            return body;
        }
        return format!(
            "{}# {}\n\nNo documentation found for this word.\n",
            alias_lead, canonical
        );
    };

    render_four_section(
        &alias_lead,
        spec.name,
        spec.stability,
        spec.category,
        spec.summary,
        spec.role,
        spec.stack_effect,
    )
}

pub fn render_four_section(
    alias_lead: &str,
    name: &str,
    stability: &str,
    category: &str,
    summary: &str,
    role: &str,
    stack_effect: &str,
) -> String {
    let mut out = String::new();
    out.push_str(alias_lead);

    if stability.is_empty() || stability == "stable" {
        out.push_str(&format!("# {}\n\n", name));
    } else {
        out.push_str(&format!("# {}  ({})\n\n", name, stability));
    }

    out.push_str("Category:\n");
    push_indented(&mut out, category, "  ");
    out.push('\n');

    out.push_str("Summary:\n");
    push_indented(&mut out, summary, "  ");
    out.push('\n');

    out.push_str("Role:\n");
    push_indented(&mut out, role, "  ");
    out.push('\n');

    out.push_str("Stack Effect:\n");
    push_indented(&mut out, stack_effect, "  ");

    out
}

fn build_alias_lead(name: &str) -> String {
    lookup_core_word_alias(name)
        .and_then(|alias| {
            alias.canonical.map(|canonical_name| match alias.kind {
                CoreWordAliasKind::SymbolAlias => {
                    format!("{} is an alias of {}.\n\n", alias.alias, canonical_name)
                }
                CoreWordAliasKind::SyntaxSugar => {
                    format!(
                        "{} is syntax sugar for {}.\n\n",
                        alias.alias, canonical_name
                    )
                }
                CoreWordAliasKind::InputHelper => {
                    format!("{} is an input helper.\n\n", alias.alias)
                }
            })
        })
        .unwrap_or_default()
}

fn push_indented(out: &mut String, body: &str, indent: &str) {
    for line in body.split('\n') {
        out.push_str(indent);
        out.push_str(line);
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::lookup_builtin_detail;

    const REQUIRED_SECTIONS: &[&str] = &["Category:", "Summary:", "Role:", "Stack Effect:"];

    #[test]
    fn lookup_for_add_contains_four_required_sections() {
        let body = lookup_builtin_detail("ADD");
        assert!(body.contains("# ADD"), "ADD header missing:\n{}", body);
        for section in REQUIRED_SECTIONS {
            assert!(
                body.contains(section),
                "ADD LOOKUP body missing section {}: full body =\n{}",
                section,
                body
            );
        }
    }

    #[test]
    fn lookup_for_alias_includes_alias_lead() {
        let body = lookup_builtin_detail("+");
        assert!(
            body.starts_with("+ is syntax sugar for ADD")
                || body.starts_with("+ is an alias of ADD"),
            "alias lead missing for '+'; got:\n{}",
            body
        );
        assert!(body.contains("# ADD"));
    }

    #[test]
    fn every_builtin_lookup_contains_all_four_sections() {
        for spec in crate::builtins::builtin_specs() {
            let body = lookup_builtin_detail(spec.name);
            for section in REQUIRED_SECTIONS {
                assert!(
                    body.contains(section),
                    "{} LOOKUP body missing section {}:\n{}",
                    spec.name,
                    section,
                    body
                );
            }
        }
    }

    #[test]
    fn experimental_word_header_shows_stability() {
        // SPAWN is marked experimental in BUILTIN_SPECS.
        let body = lookup_builtin_detail("SPAWN");
        assert!(
            body.contains("# SPAWN  (experimental)"),
            "SPAWN header must show '(experimental)':\n{}",
            body
        );
    }

    #[test]
    fn stable_word_header_omits_stability() {
        let body = lookup_builtin_detail("ADD");
        assert!(
            body.contains("# ADD\n"),
            "ADD (stable) header must be bare:\n{}",
            body
        );
        assert!(
            !body.contains("# ADD  (stable)"),
            "stable stability must NOT be shown in header:\n{}",
            body
        );
    }

    #[test]
    fn comparison_words_have_uniform_stack_effect() {
        // All six comparison primitives must use the same stack-effect
        // notation so the four-section template is consistent across the
        // comparison category.
        const EXPECTED: &str = "[ a ] [ b ] -> [ TRUE | FALSE ]";
        for name in &["EQ", "NEQ", "LT", "LTE", "GT", "GTE"] {
            let spec = crate::builtins::builtin_word_definitions::lookup_builtin_spec(name)
                .unwrap_or_else(|| panic!("{} must have a BuiltinSpec", name));
            assert_eq!(
                spec.stack_effect, EXPECTED,
                "{} stack_effect deviates from the comparison-word standard",
                name
            );
        }
    }

    #[test]
    fn lookup_output_is_utf8_plain_text() {
        for name in ["ADD", "MAP", "LOOKUP", "DEF", "VENT", "TOP", "PRINT"] {
            let body = lookup_builtin_detail(name);
            assert!(
                !body.chars().any(|c| c.is_control() && c != '\n'),
                "LOOKUP body for {} must be UTF-8 plain text without control characters:\n{}",
                name,
                body
            );
        }
    }
}
