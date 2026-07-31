use crate::coreword_registry::{ExecutionForm, Partiality, SafetyLevel};

use super::builtin_specs as authored_specs;

/// The hand-written half of a Core Word's description.
///
/// What used to live here was the Word's *contract* — stack arity, NIL policy,
/// purity, determinism, executor key — written a second time in Rust and
/// reconciled with `spec/words.json` by after-the-fact checking scripts. That
/// arrangement is what let eleven Words carry a policy label contradicting the
/// one the specification declared. Those fields are gone: the contract is read
/// from `kernel::generated`, projected from `spec/words.json`, and there is no
/// second place to write it down.
///
/// What remains is what the specification does not declare — the prose a reader
/// sees, and three runtime-local classifications (`partiality`,
/// `safety_level`, `safe_preview`) that describe how the *implementation*
/// treats a Word rather than what the language says it means. Nothing here is
/// a second copy of a canonical fact.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub category: &'static str,
    /// Layer 3 (hover): one-line "WORD — short verb phrase" shown in the
    /// native button title attribute. See three-layer-documentation-model.md
    /// §4.2. Retained during Phase 5 only as a migration-validation oracle.
    #[allow(dead_code)]
    pub hover_summary: &'static str,
    /// Layer 3 (hover): shortest useful invocation (operands included, sugar
    /// preferred when shorter) shown in the inline word-info strip. See
    /// three-layer-documentation-model.md §4.3.
    pub hover_syntax: &'static str,

    // Layer 2 (LOOKUP) fields. Four-section template:
    //   Category / Summary / Role / Stack Effect
    // Stability is shown in the header (e.g. `# ADD  (experimental)`).
    pub summary: &'static str,
    pub role: &'static str,
    pub stack_effect: &'static str,
    /// Must agree with the `safety_level` field below. The mapping is:
    ///   safety_level A or B  -> "stable"
    ///   safety_level C or D  -> "experimental"
    ///   safety_level Quarantined -> "experimental"
    /// A consistency test asserts this invariant.
    pub stability: &'static str,

    pub safe_preview: bool,
    pub partiality: Partiality,
    pub safety_level: SafetyLevel,
    /// How the word takes effect (SPEC §6.4). Defaults to `RuntimeWord`; the
    /// lazy/no-op control directives (`VENT`, `FLOW`) set this so the
    /// classification is machine-checkable rather than inferred from prose.
    pub execution_form: ExecutionForm,
}

pub(in crate::builtins) const SPEC_DEFAULT: BuiltinSpec = BuiltinSpec {
    name: "",
    category: "",
    hover_summary: "",
    hover_syntax: "",
    summary: "",
    role: "",
    stack_effect: "",
    stability: "stable",
    safe_preview: true,
    partiality: Partiality::Total,
    safety_level: SafetyLevel::A,
    execution_form: ExecutionForm::RuntimeWord,
};

const BUILTIN_SPECS: &[BuiltinSpec] = &[
    authored_specs::execution::EAT,
    authored_specs::execution::KEEP,
    authored_specs::collections::GET,
    authored_specs::collections::INSERT,
    authored_specs::collections::REPLACE,
    authored_specs::collections::REMOVE,
    authored_specs::collections::LENGTH,
    authored_specs::collections::TAKE,
    authored_specs::collections::SPLIT,
    authored_specs::collections::CONCAT,
    authored_specs::collections::REVERSE,
    authored_specs::collections::RANGE,
    authored_specs::collections::REORDER,
    authored_specs::collections::COLLECT,
    authored_specs::scalar::TRUE,
    authored_specs::scalar::FALSE,
    authored_specs::scalar::NIL,
    authored_specs::scalar::NIL_PREDICATE,
    authored_specs::scalar::NIL_REASON,
    authored_specs::scalar::CHARS,
    authored_specs::scalar::JOIN,
    authored_specs::scalar::TRIM,
    authored_specs::scalar::TOKENIZE,
    authored_specs::scalar::SUBSTITUTE,
    authored_specs::scalar::STARTS_WITH,
    authored_specs::scalar::ENDS_WITH,
    authored_specs::scalar::NUM,
    authored_specs::scalar::STR,
    authored_specs::scalar::CHR,
    authored_specs::numeric::ADD,
    authored_specs::numeric::SUB,
    authored_specs::numeric::MUL,
    authored_specs::numeric::DIV,
    authored_specs::numeric::EQ,
    authored_specs::numeric::LT,
    authored_specs::numeric::LTE,
    authored_specs::numeric::GT,
    authored_specs::numeric::GTE,
    authored_specs::numeric::NEQ,
    authored_specs::scalar::AND,
    authored_specs::scalar::OR,
    authored_specs::scalar::NOT,
    authored_specs::execution::COND,
    authored_specs::execution::VENT,
    authored_specs::execution::MAP,
    authored_specs::execution::FILTER,
    authored_specs::execution::FOLD,
    authored_specs::execution::ANY,
    authored_specs::execution::ALL,
    authored_specs::execution::PRINT,
    authored_specs::execution::DEF,
    authored_specs::execution::DEL,
    authored_specs::execution::LOOKUP,
    authored_specs::collections::FILL,
    authored_specs::numeric::MOD,
    authored_specs::numeric::FLOOR,
    authored_specs::numeric::CEIL,
    authored_specs::numeric::ROUND,
    authored_specs::execution::EXEC,
    authored_specs::numeric::SQRT,
    authored_specs::numeric::ABS,
    authored_specs::numeric::NEG,
    authored_specs::numeric::SIGN,
    authored_specs::numeric::MIN,
    authored_specs::numeric::MAX,
    authored_specs::collections::SORT,
    authored_specs::collections::UNIQUE,
    authored_specs::collections::CONTAINS,
    authored_specs::collections::INDEX_OF,
];

/// The whole prose table.
///
/// Every runtime consumer now reaches a Word through the generated registry
/// and looks its prose up by name, so the only readers of the table as a whole
/// are the tests that assert the two halves agree — but it stays public so
/// that assertion can be written from anywhere.
#[cfg_attr(not(test), allow(dead_code))]
pub fn builtin_specs() -> &'static [BuiltinSpec] {
    BUILTIN_SPECS
}

pub fn lookup_builtin_spec(name: &str) -> Option<&'static BuiltinSpec> {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    BUILTIN_SPECS.iter().find(|spec| spec.name == canonical)
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
            "ADD", "SUB", "MUL", "DIV", "MOD", "EQ", "NEQ", "LT", "LTE", "GT", "GTE", "EAT",
            "KEEP", "LOOKUP", "VENT", "SQRT", "SORT",
        ];

        for name in required {
            assert!(
                super::lookup_builtin_spec(name).is_some(),
                "missing canonical core word: {}",
                name
            );
        }
    }

    #[test]
    fn generated_core_docs_preserve_the_legacy_observation() {
        let generated = super::super::generated_core_word_docs::GENERATED_CORE_WORD_DOCS;
        assert_eq!(generated.len(), super::builtin_specs().len());

        for (doc, spec) in generated.iter().zip(super::builtin_specs()) {
            assert_eq!(doc.name, spec.name);
            assert_eq!(doc.hover_summary, spec.hover_summary);
            assert_eq!(doc.hover_syntax, spec.hover_syntax);
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
