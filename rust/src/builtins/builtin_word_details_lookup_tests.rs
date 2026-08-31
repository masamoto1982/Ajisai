//! LOOKUP rendering checks for `builtin_word_details.rs`. Kept in a sibling
//! file so the renderer stays within the §14.1 file-size budget.

use super::builtin_word_definitions::{builtin_specs, lookup_builtin_spec};
use super::builtin_word_details::lookup_builtin_detail;
use super::builtin_word_lookup_docs::builtin_lookup_docs;

const REQUIRED_SECTIONS: &[&str] = &["Category:", "Summary:", "Role:", "Stack Effect:"];

/// Sections every builtin now renders, authored entry or not: the
/// derived template (three-layer model §3.4) on top of the four base
/// sections.
const DERIVED_SECTIONS: &[&str] = &["Examples:", "Failure:", "Side Effects:", "Stability:"];

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

#[test]
fn every_authored_doc_entry_names_a_real_builtin() {
    for doc in builtin_lookup_docs() {
        assert!(
            lookup_builtin_spec(doc.word).is_some(),
            "authored LOOKUP doc for `{}` has no matching BuiltinSpec",
            doc.word
        );
        for related in doc.related {
            assert!(
                lookup_builtin_spec(related).is_some(),
                "`{}` lists unknown related word `{}`",
                doc.word,
                related
            );
        }
    }
}

#[test]
fn authored_doc_entries_are_editor_safe_plain_text() {
    // §3.3: UTF-8 English plain text, ≤ 80 columns, no control
    // characters, no trailing whitespace — the LOOKUP body is loaded
    // into the code editor verbatim.
    for doc in builtin_lookup_docs() {
        assert!(
            !doc.behavior.is_empty(),
            "`{}` has an empty behavior",
            doc.word
        );
        let mut texts: Vec<&str> = vec![doc.behavior, doc.failure_note];
        for example in doc.examples {
            assert!(
                !example.code.is_empty(),
                "`{}` has an example with empty code",
                doc.word
            );
            texts.push(example.code);
            texts.push(example.result);
        }
        for text in texts {
            for line in text.split('\n') {
                assert!(
                    line.len() <= 80,
                    "`{}` has a line over 80 columns: {}",
                    doc.word,
                    line
                );
                assert!(
                    !line.chars().any(|c| c.is_control()),
                    "`{}` has a control character in: {}",
                    doc.word,
                    line
                );
                assert_eq!(
                    line,
                    line.trim_end(),
                    "`{}` has trailing whitespace in: {}",
                    doc.word,
                    line
                );
            }
        }
    }
}

#[test]
fn authored_entry_renders_behavior_examples_and_related() {
    let body = lookup_builtin_detail("GET");
    for section in ["Behavior:", "Related:", "Result:"] {
        assert!(
            body.contains(section),
            "GET LOOKUP body missing {}: full body =\n{}",
            section,
            body
        );
    }
    assert!(
        body.contains("indexOutOfBounds"),
        "GET Failure must name the NIL-projection reason:\n{}",
        body
    );
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
    let standard = lookup_builtin_detail("MAP");
    assert!(
        standard.contains("Standard vocabulary (operational)"),
        "MAP LOOKUP body must name its Standard kind:\n{}",
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
    // The three-layer model (§2.3) requires GET / DIV / NUM / CHR to
    // describe their Bubble/NIL cases (the specification's term for a
    // reasoned NIL) separately from contract errors.
    for word in ["GET", "DIV", "NUM"] {
        let body = lookup_builtin_detail(word);
        assert!(
            body.contains("Bubble/NIL"),
            "{} LOOKUP body must describe its Bubble/NIL case:\n{}",
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
fn experimental_word_header_shows_stability() {
    // PRINT is marked experimental in BUILTIN_SPECS.
    let body = lookup_builtin_detail("PRINT");
    assert!(
        body.contains("# PRINT  (experimental)"),
        "PRINT header must show '(experimental)':\n{}",
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
    for name in ["ADD", "MAP", "LOOKUP", "DEF", "OR-NIL", "TOP", "PRINT"] {
        let body = lookup_builtin_detail(name);
        assert!(
            !body.chars().any(|c| c.is_control() && c != '\n'),
            "LOOKUP body for {} must be UTF-8 plain text without control characters:\n{}",
            name,
            body
        );
    }
}
