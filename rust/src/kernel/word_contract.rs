//! The spine's typed view of a Word's contract, derived from the generated
//! registry (migration plan §12). The execution wrapper reads this — arity,
//! nil policy — so the shared concerns of every Word are applied from one
//! machine-checked source (`spec/words.json`) rather than re-encoded per Word.
//!
//! The contract vocabularies themselves live in the generated registry, where
//! they are projected from the `enum` lists in `spec/words.schema.json`. This
//! module deliberately does not restate them: a second hand-written copy is
//! exactly how the implementation vocabulary drifted narrower than the
//! specification's in the first place.

use super::generated::{Arity as SpecArity, GeneratedWord, NilPolicy};

/// Fixed stack arity: how many operands a Word consumes and how many results it
/// produces under the default consume mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arity {
    pub consumes: u8,
    pub produces: u8,
}

/// Whether a NIL operand short-circuits the Word to a NIL result without
/// running its primitive.
pub fn passes_nil_through(policy: NilPolicy) -> bool {
    matches!(
        policy,
        NilPolicy::Passthrough | NilPolicy::PassthroughThenProject
    )
}

/// A Word's static contract as the execution wrapper consumes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordContract {
    pub arity: Arity,
    pub nil_policy: NilPolicy,
}

impl WordContract {
    /// Build a fixed-arity contract from a generated Word. Returns `None` when
    /// the Word's stack arity is data-dependent (`variable`/`control`) — such
    /// Words do not flow through the fixed-arity wrapper.
    pub fn from_generated(word: &GeneratedWord) -> Option<Self> {
        let consumes = arity_count(word.stack_inputs)?;
        let produces = arity_count(word.stack_outputs)?;
        Some(WordContract {
            arity: Arity { consumes, produces },
            nil_policy: word.nil_policy,
        })
    }
}

fn arity_count(arity: SpecArity) -> Option<u8> {
    arity.fixed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::generated::GENERATED_WORDS;

    fn word(name: &str) -> &'static GeneratedWord {
        GENERATED_WORDS
            .iter()
            .find(|word| word.name == name)
            .expect("word present in generated registry")
    }

    #[test]
    fn fixed_arity_word_yields_a_contract() {
        let contract = WordContract::from_generated(word("ADD")).expect("ADD is fixed-arity");
        assert_eq!(
            contract.arity,
            Arity {
                consumes: 2,
                produces: 1
            }
        );
        assert!(passes_nil_through(contract.nil_policy));
    }

    #[test]
    fn and_is_kleene_absorbing_not_a_blanket_passthrough() {
        let contract = WordContract::from_generated(word("AND")).expect("AND is fixed-arity");
        assert_eq!(contract.nil_policy, NilPolicy::KleeneAbsorbing);
        // A NIL operand does not blanket-short-circuit AND/OR the way it does
        // for the arithmetic family: FALSE can still absorb it into FALSE, so
        // the shared wrapper must not pre-empt the primitive here.
        assert!(!passes_nil_through(contract.nil_policy));
    }

    #[test]
    fn div_passes_nil_through_then_projects() {
        let contract = WordContract::from_generated(word("DIV")).expect("DIV is fixed-arity");
        assert_eq!(contract.nil_policy, NilPolicy::PassthroughThenProject);
        assert!(passes_nil_through(contract.nil_policy));
    }

    #[test]
    fn data_dependent_arity_has_no_fixed_contract() {
        // COLLECT's stack arity is "variable" in spec/words.json (it gathers
        // however many values a preceding count names), so it does not flow
        // through the fixed-arity wrapper. COND used to be this module's
        // example — its clauses are a single Vector operand now (CodeBlock/
        // Vector unification, docs/dev/type-unification-work-order-2026-08.md),
        // so its own arity is fixed at 2 like any other higher-order Word.
        assert!(WordContract::from_generated(word("COLLECT")).is_none());
    }

    /// The regression this whole projection exists to prevent: every value the
    /// specification admits must be reachable as a variant. A hand-written
    /// vocabulary silently collapsed `passthroughThenProject` and `inspectNil`
    /// into neighbouring values, which is how 11 Words came to be mislabelled.
    #[test]
    fn every_spec_nil_policy_is_representable() {
        let declared: Vec<&str> = GENERATED_WORDS
            .iter()
            .map(|word| word.nil_policy.as_spec_str())
            .collect();
        for value in [
            "preserveReason",
            "consumeNil",
            "rejectNil",
            "passthrough",
            "createsNil",
            "passthroughThenProject",
        ] {
            assert!(
                declared.contains(&value),
                "no Word carries nilPolicy `{value}`; the projection lost a specification value"
            );
        }
    }
}
