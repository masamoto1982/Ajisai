use super::builtin_word_definitions::{
    lookup_builtin_spec, BuiltinExampleDoc, BuiltinSyntaxDoc,
};
use crate::core_word_aliases::{
    lookup_core_word_alias, CoreWordAliasKind, CORE_WORD_ALIASES,
};

/// Render the LOOKUP body for a built-in word.
///
/// Output is the §3.4 template from `docs/dev/three-layer-documentation-model.md`:
/// ASCII English plain text, sectioned by capitalized headings, two-space
/// indentation for nested blocks. Suitable to be loaded into the editor
/// textarea verbatim.
pub fn lookup_builtin_detail(name: &str) -> String {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    let alias_lead = build_alias_lead(name);

    let Some(spec) = lookup_builtin_spec(&canonical) else {
        // Module-imported built-ins (e.g. MUSIC@PLAY) are intentionally
        // out of Phase 2 scope. They fall back to a placeholder until
        // module words are extended to the three-layer model
        // (handover Phase 4).
        return format!(
            "{}# {}\n\nDocumentation for this word is a placeholder pending\nthe Phase 4 extension of the three-layer documentation\nmodel to module words.\n",
            alias_lead, canonical
        );
    };

    let mut out = String::new();
    out.push_str(&alias_lead);
    out.push_str(&format!("# {}\n\n", spec.name));

    if let Some(sugar) = primary_sugar_for(spec.name) {
        out.push_str("Sugar:\n");
        out.push_str(&format!("  {} = {}\n\n", sugar, spec.name));
    }

    out.push_str("Category:\n");
    out.push_str(&format!("  {}\n\n", spec.category));

    out.push_str("Summary:\n");
    push_indented(&mut out, spec.summary, "  ");
    out.push('\n');

    if let Some(role) = spec.role {
        out.push_str("Role:\n");
        push_indented(&mut out, role, "  ");
        out.push('\n');
    }

    out.push_str("Syntax:\n");
    render_syntax_forms(&mut out, spec.syntax_forms);
    out.push('\n');

    out.push_str("Stack Effect:\n");
    push_indented(&mut out, spec.stack_effect, "  ");
    out.push('\n');

    out.push_str("Behavior:\n");
    push_indented(&mut out, spec.behavior, "  ");
    out.push('\n');

    if !spec.examples.is_empty() {
        out.push_str("Examples:\n");
        render_examples(&mut out, spec.examples);
        out.push('\n');
    }

    if let Some(failure) = spec.failure {
        out.push_str("Failure:\n");
        push_indented(&mut out, failure, "  ");
        out.push('\n');
    }

    if !spec.side_effects.is_empty() {
        out.push_str("Side Effects:\n");
        for se in spec.side_effects {
            out.push_str(&format!("  {}\n", se));
        }
        out.push('\n');
    }

    if let Some(mi) = spec.modifier_interaction {
        out.push_str("Modifier Interaction:\n");
        push_indented(&mut out, mi, "  ");
        out.push('\n');
    }

    if !spec.related.is_empty() {
        out.push_str("Related:\n");
        out.push_str(&format!("  {}\n\n", spec.related.join(", ")));
    }

    out.push_str("Stability:\n");
    out.push_str(&format!("  {}\n", spec.stability));

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

fn primary_sugar_for(canonical: &str) -> Option<&'static str> {
    CORE_WORD_ALIASES
        .iter()
        .find(|a| {
            a.canonical == Some(canonical)
                && matches!(
                    a.kind,
                    CoreWordAliasKind::SymbolAlias | CoreWordAliasKind::SyntaxSugar
                )
        })
        .map(|a| a.alias)
}

fn render_syntax_forms(out: &mut String, forms: &'static [BuiltinSyntaxDoc]) {
    for form in forms {
        out.push_str("  Canonical:\n");
        push_indented(out, form.canonical, "    ");
        if let Some(short) = form.shorthand {
            out.push_str("  Shorthand:\n");
            push_indented(out, short, "    ");
        }
        if let Some(desc) = form.description {
            push_indented(out, desc, "  ");
        }
    }
}

fn render_examples(out: &mut String, examples: &'static [BuiltinExampleDoc]) {
    let multiple = examples.len() > 1;
    for (i, ex) in examples.iter().enumerate() {
        if multiple && i > 0 {
            out.push('\n');
        }
        out.push_str("  Canonical:\n");
        push_indented(out, ex.canonical, "    ");
        if let Some(short) = ex.shorthand {
            out.push_str("  Shorthand:\n");
            push_indented(out, short, "    ");
        }
        if let Some(result) = ex.result {
            out.push_str("\n  Result:\n");
            push_indented(out, result, "    ");
        }
    }
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

    #[test]
    fn lookup_for_add_contains_template_sections() {
        let body = lookup_builtin_detail("ADD");
        for section in [
            "# ADD",
            "Sugar:",
            "Category:",
            "Summary:",
            "Role:",
            "Syntax:",
            "Stack Effect:",
            "Behavior:",
            "Examples:",
            "Failure:",
            "Related:",
            "Stability:",
        ] {
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
            body.starts_with("+ is an alias of ADD"),
            "alias lead missing for '+'; got:\n{}",
            body
        );
        assert!(body.contains("# ADD"));
    }

    #[test]
    fn lookup_for_def_omits_sugar_section() {
        // DEF has no sugar; the Sugar: heading must not appear.
        let body = lookup_builtin_detail("DEF");
        assert!(
            !body.contains("Sugar:"),
            "DEF has no sugar but Sugar: section was emitted:\n{}",
            body
        );
    }

    #[test]
    fn lookup_for_lookup_includes_sugar_section() {
        let body = lookup_builtin_detail("LOOKUP");
        assert!(
            body.contains("Sugar:\n  ? = LOOKUP"),
            "LOOKUP must show '? = LOOKUP' sugar:\n{}",
            body
        );
    }

    #[test]
    fn lookup_output_is_ascii() {
        for name in ["ADD", "MAP", "LOOKUP", "DEF", "OR-NIL", "TOP", "PRINT"] {
            let body = lookup_builtin_detail(name);
            assert!(
                body.is_ascii(),
                "LOOKUP body for {} contains non-ASCII characters:\n{}",
                name,
                body
            );
        }
    }
}
