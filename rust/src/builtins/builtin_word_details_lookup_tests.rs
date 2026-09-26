//! LOOKUP rendering checks for `builtin_word_details.rs`. Kept in a sibling
//! file so the renderer stays within the the file-size budget in docs/dev/specification-implementation-rules.md file-size budget.

use super::builtin_word_definitions::{builtin_specs, lookup_builtin_spec};
use super::builtin_word_details::lookup_builtin_detail;

const REQUIRED_SECTIONS: &[&str] = &["Family:", "Summary:", "Stack Effect:"];

/// Sections every builtin now renders, authored entry or not: the
/// derived template (three-layer model §3.4) on top of the four base
/// sections.
const DERIVED_SECTIONS: &[&str] = &["Examples:", "Failure:", "Side Effects:", "Vocabulary:"];

#[test]
fn every_builtin_renders_the_derived_sections() {
    for spec in builtin_specs() {
        let body = lookup_builtin_detail(spec.name);
        for section in REQUIRED_SECTIONS.iter().chain(DERIVED_SECTIONS) {
            assert!(
                body.contains(section),
                "{} LOOKUP body missing section {}: full body =\n{}",
                spec.name,
                section,
                body
            );
        }
    }
}

/// `LOOKUP` is a reading surface for the vocabulary, so it says which half
/// of the public Core a Word belongs to — and says it in terms that keep
/// Core one flat dictionary.
#[test]
fn lookup_reports_the_vocabulary_tier() {
    let kernel = lookup_builtin_detail("FOLD");
    assert!(
        kernel.contains("Vocabulary:") && kernel.contains("Semantic Kernel"),
        "FOLD LOOKUP body must name the Semantic Kernel:\n{}",
        kernel
    );
    let standard = lookup_builtin_detail("FILTER");
    assert!(
        standard.contains("Standard vocabulary (operational)"),
        "FILTER LOOKUP body must name its Standard kind:\n{}",
        standard
    );
    for word in builtin_specs() {
        let body = lookup_builtin_detail(word.name);
        assert!(
            body.contains("Vocabulary:"),
            "{} LOOKUP body has no Vocabulary section",
            word.name
        );
    }
}

#[test]
fn nil_projection_rule_words_describe_nil_not_only_errors() {
    // GET / DIV / NUM describe their NIL cases separately from contract
    // errors.
    for word in ["GET", "DIV", "NUM"] {
        let body = lookup_builtin_detail(word);
        assert!(
            body.contains("NIL"),
            "{} LOOKUP body must describe its NIL case:\n{}",
            word,
            body
        );
    }
}

#[test]
fn word_without_authored_entry_falls_back_to_hover_example() {
    let body = lookup_builtin_detail("ROUND");
    let spec = lookup_builtin_spec("ROUND").expect("ROUND spec");
    assert!(
        body.contains(spec.hover_syntax),
        "ROUND Examples should reuse hover_syntax until authored:\n{}",
        body
    );
}

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
        body.starts_with("+ is syntax sugar for ADD") || body.starts_with("+ is an alias of ADD"),
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
fn word_header_is_the_bare_name() {
    // The header carries the name alone: no label the registry does not
    // declare. PRINT used to read `(experimental)` for having an effect.
    for name in ["ADD", "PRINT", "DEF"] {
        let body = lookup_builtin_detail(name);
        assert!(
            body.contains(&format!("# {name}\n")),
            "{name} header must be bare:\n{body}"
        );
    }
}

#[test]
fn comparison_words_have_uniform_stack_effect() {
    // All six comparison primitives must use the same stack-effect
    // notation so the four-section template is consistent across the
    // comparison category.
    const EXPECTED: &str = "[ a ] [ b ] -> [ TRUE | FALSE ]";
    for name in &["EQ", "LT", "GT"] {
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
    for name in ["ADD", "MAP", "LOOKUP", "DEF", "TOP", "PRINT"] {
        let body = lookup_builtin_detail(name);
        assert!(
            !body.chars().any(|c| c.is_control() && c != '\n'),
            "LOOKUP body for {} must be UTF-8 plain text without control characters:\n{}",
            name,
            body
        );
    }
}
