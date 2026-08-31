use crate::coreword_registry::{ExecutionForm, Partiality};

/// Runtime view of a canonical Core Word.
///
/// Documentation and presentation are generated from `spec/words.json`; safety,
/// partiality, stability, and execution form are projected from the generated
/// contract. This type assembles those projections for existing GUI and reference
/// consumers and owns no parallel source of language facts.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub category: &'static str,
    pub summary: &'static str,
    #[allow(dead_code)]
    pub hover_summary: &'static str,
    pub hover_syntax: &'static str,
    pub role: &'static str,
    pub stack_effect: &'static str,
    pub stability: &'static str,
    pub partiality: Partiality,
    pub execution_form: ExecutionForm,
}

/// Complete projected Core Word view.
///
/// It is assembled once from generated documentation and generated contracts for
/// runtime consumers and invariant tests.
#[cfg_attr(not(test), allow(dead_code))]
pub fn builtin_specs() -> &'static [BuiltinSpec] {
    static SPECS: std::sync::OnceLock<Vec<BuiltinSpec>> = std::sync::OnceLock::new();
    SPECS.get_or_init(|| {
        super::generated_core_word_docs::GENERATED_CORE_WORD_DOCS
            .iter()
            .map(|doc| {
                let word = crate::kernel::generated::generated_word(doc.name)
                    .expect("generated documentation must name a canonical Word");
                BuiltinSpec {
                    name: doc.name,
                    category: doc.category,
                    summary: doc.summary,
                    hover_summary: doc.hover_summary,
                    hover_syntax: doc.hover_syntax,
                    role: doc.role,
                    stack_effect: doc.stack_effect,
                    stability: crate::coreword_registry::stability_from_contract(word),
                    partiality: crate::coreword_registry::partiality_from_contract(word),
                    execution_form: crate::coreword_registry::execution_form_from_contract(word),
                }
            })
            .collect()
    })
}

pub fn lookup_builtin_spec(name: &str) -> Option<&'static BuiltinSpec> {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    builtin_specs().iter().find(|spec| spec.name == canonical)
}

/// WASM/GUI tuple shape: `(name, hover_summary, hover_syntax)`.
/// Position 1 (`hover_summary`) is the native button-title text;
/// position 2 (`hover_syntax`) is the inline word-info preview.
/// See three-layer-documentation-model.md §4.
///
/// Consumed only by the wasm bindings (feature = "wasm").
#[cfg_attr(not(feature = "wasm"), allow(dead_code))]
pub fn collect_core_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str)> {
    super::generated_core_word_docs::GENERATED_CORE_WORD_DOCS
        .iter()
        .map(|doc| (doc.name, doc.hover_summary, doc.hover_syntax))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn builtin_specs_do_not_contain_symbol_aliases_or_input_helpers() {
        let forbidden = [
            "+", "-", "*", "/", "%", "=", "<", "<=", ">", ">=", "<>", ".", "..", ",", ",,", "~",
            "!", "'", "|", "?", "^",
        ];

        for spec in super::builtin_specs() {
            assert!(
                !forbidden.contains(&spec.name),
                "builtin spec must not contain symbol/helper word: {}",
                spec.name
            );
        }
    }

    #[test]
    fn builtin_specs_contain_canonical_core_words() {
        let required = [
            "ADD", "SUB", "MUL", "DIV", "MOD", "EQ", "NEQ", "LT", "LTE", "GT", "GTE", "KEEP",
            "OR-NIL", "SQRT", "SORT",
        ];

        for name in required {
            assert!(
                super::lookup_builtin_spec(name).is_some(),
                "missing canonical core word: {}",
                name
            );
        }
    }

    /// Every prose field of a spec must be the generated value, verbatim.
    ///
    /// `check:runtime-metadata` proves the same thing syntactically — that each
    /// field is written as `doc.<field>` — but only this asserts it of the
    /// values actually observed at runtime, so a projection that compiled while
    /// transforming or substituting prose would still be caught here.
    #[test]
    fn generated_core_docs_preserve_the_legacy_observation() {
        let generated = super::super::generated_core_word_docs::GENERATED_CORE_WORD_DOCS;
        assert_eq!(generated.len(), super::builtin_specs().len());

        for (doc, spec) in generated.iter().zip(super::builtin_specs()) {
            assert_eq!(doc.name, spec.name);
            assert_eq!(doc.category, spec.category, "{} category", doc.name);
            assert_eq!(doc.summary, spec.summary, "{} summary", doc.name);
            assert_eq!(doc.role, spec.role, "{} role", doc.name);
            assert_eq!(
                doc.stack_effect, spec.stack_effect,
                "{} stack_effect",
                doc.name
            );
            assert_eq!(doc.hover_summary, spec.hover_summary);
            assert_eq!(doc.hover_syntax, spec.hover_syntax);
        }
    }

    /// The contract-projected fields must equal the canonical contract's own
    /// projection for the same Word, so `BuiltinSpec` cannot drift from
    /// `spec/words.json` even if someone rewires the assembly.
    #[test]
    fn contract_projected_fields_match_the_canonical_contract() {
        for spec in super::builtin_specs() {
            let word = crate::kernel::generated::generated_word(spec.name)
                .expect("every spec name must be a canonical Word");
            assert_eq!(
                spec.stability,
                crate::coreword_registry::stability_from_contract(word),
                "{} stability",
                spec.name
            );
            assert_eq!(
                spec.partiality,
                crate::coreword_registry::partiality_from_contract(word),
                "{} partiality",
                spec.name
            );
            assert_eq!(
                spec.execution_form,
                crate::coreword_registry::execution_form_from_contract(word),
                "{} execution_form",
                spec.name
            );
        }
    }

    #[test]
    fn builtin_specs_have_required_lookup_content() {
        for spec in super::builtin_specs() {
            assert!(!spec.summary.is_empty(), "{} missing summary", spec.name);
            assert!(!spec.role.is_empty(), "{} missing role", spec.name);
            assert!(!spec.category.is_empty(), "{} missing category", spec.name);
            assert!(
                !spec.stack_effect.is_empty(),
                "{} missing stack_effect",
                spec.name
            );
            assert!(
                spec.stability == "stable" || spec.stability == "experimental",
                "{} has invalid stability {}",
                spec.name,
                spec.stability
            );
        }
    }

    #[test]
    fn builtin_specs_stack_effect_grammar() {
        for spec in super::builtin_specs() {
            // Control directives (SPEC §6.4) act positionally on the source
            // stream, not as a stack `X -> Y` transformation, so the arrow
            // grammar does not apply to them; their contract is carried by
            // `execution_form` and a prose stack-effect note.
            if spec.execution_form != crate::coreword_registry::ExecutionForm::RuntimeWord {
                continue;
            }
            let s = spec.stack_effect;
            let is_literal_no_op =
                s == "no values popped or pushed" || s == "operands preserved; result pushed";
            if is_literal_no_op {
                continue;
            }
            assert!(
                s.contains("->"),
                "{} stack_effect missing '->' arrow: {:?}",
                spec.name,
                s
            );
        }
    }

    #[test]
    fn builtin_specs_lookup_text_is_utf8_plain_text() {
        let check = |label: &str, name: &str, text: &str| {
            assert!(
                !text.chars().any(|c| c.is_control() && c != '\n'),
                "{} field of {} must be UTF-8 plain text without control characters; got: {:?}",
                label,
                name,
                text
            );
        };
        for spec in super::builtin_specs() {
            check("summary", spec.name, spec.summary);
            check("role", spec.name, spec.role);
            check("stack_effect", spec.name, spec.stack_effect);
            check("category", spec.name, spec.category);
        }
    }
}
