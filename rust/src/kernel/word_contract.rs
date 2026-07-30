//! The spine's typed view of a Word's contract, derived from the generated
//! registry (migration plan §12). The execution wrapper reads this — arity,
//! nil policy — so the shared concerns of every Word are applied from one
//! machine-checked source (`spec/words.json`) rather than re-encoded per Word.

use super::generated::GeneratedWord;

/// Fixed stack arity: how many operands a Word consumes and how many results it
/// produces under the default consume mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arity {
    pub consumes: u8,
    pub produces: u8,
}

/// How a Word treats a NIL operand, projected from the spec `nilPolicy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NilPolicy {
    /// A NIL operand passes through: the Word yields NIL without running.
    Passthrough,
    /// A NIL operand passes through, and the Word may itself project a result
    /// to NIL (e.g. `DIV` by zero).
    PassthroughThenProject,
    /// The Word may create a NIL from non-NIL operands.
    CreatesNil,
    /// The Word preserves an operand's NIL reason unchanged.
    PreservesReason,
    /// The Word rejects a NIL operand as an error rather than passing it.
    RejectsNil,
    /// The Word consumes a NIL operand as ordinary input.
    ConsumeNil,
    /// The Word inspects NIL-ness without propagating it.
    InspectNil,
}

impl NilPolicy {
    fn from_spec(spec: &str) -> Option<Self> {
        Some(match spec {
            "passthrough" => NilPolicy::Passthrough,
            "passthroughThenProject" => NilPolicy::PassthroughThenProject,
            "createsNil" => NilPolicy::CreatesNil,
            "preserveReason" => NilPolicy::PreservesReason,
            "rejectNil" => NilPolicy::RejectsNil,
            "consumeNil" => NilPolicy::ConsumeNil,
            "inspectNil" => NilPolicy::InspectNil,
            _ => return None,
        })
    }

    /// Whether a NIL operand short-circuits the Word to a NIL result without
    /// running its primitive.
    pub fn passes_nil_through(self) -> bool {
        matches!(
            self,
            NilPolicy::Passthrough | NilPolicy::PassthroughThenProject
        )
    }
}

/// A Word's static contract as the execution wrapper consumes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordContract {
    pub arity: Arity,
    pub nil_policy: NilPolicy,
}

impl WordContract {
    /// Build a fixed-arity contract from a generated Word. Returns `None` when
    /// the Word's stack arity is data-dependent (`"variable"`/`"control"`) or
    /// its nil policy is unrecognized — such Words do not flow through the
    /// fixed-arity wrapper (yet).
    pub fn from_generated(word: &GeneratedWord) -> Option<Self> {
        let consumes = word.stack_inputs.parse::<u8>().ok()?;
        let produces = word.stack_outputs.parse::<u8>().ok()?;
        let nil_policy = NilPolicy::from_spec(word.nil_policy)?;
        Some(WordContract {
            arity: Arity { consumes, produces },
            nil_policy,
        })
    }
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
        assert!(contract.nil_policy.passes_nil_through());
    }

    #[test]
    fn div_passes_nil_through_then_projects() {
        let contract = WordContract::from_generated(word("DIV")).expect("DIV is fixed-arity");
        assert_eq!(contract.nil_policy, NilPolicy::PassthroughThenProject);
        assert!(contract.nil_policy.passes_nil_through());
    }

    #[test]
    fn data_dependent_arity_has_no_fixed_contract() {
        // COND's stack arity is "variable" in spec/words.json, so it does not
        // flow through the fixed-arity wrapper.
        assert!(WordContract::from_generated(word("COND")).is_none());
    }
}
