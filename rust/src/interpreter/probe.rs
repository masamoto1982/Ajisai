//! `PROBE` — the pre-execution contract checker exposed as a Core Word.
//!
//! `ajisai check --contract` and `ajisai contract` already run this same
//! inference (`Interpreter::infer_word_contract`) over a named dictionary
//! Word, from outside the language. `PROBE` is the same operation reached
//! from inside it: narrowing a description (a CodeBlock) into what can be
//! known about it without running it, over `Interpreter::
//! infer_contract_for_block`. Narrowing is total here — a well-formed
//! CodeBlock always yields a knowledge Vector, never NIL — because the
//! trichotomy this check reports (`LANG.CONTRACT.CHECK`: verified / cannot
//! verify / violated) lives inside the returned value, as `confidence` and
//! `gaps`, not in PROBE's own outcome category.

use crate::error::{AjisaiError, Result};
use crate::interpreter::word_contract::WordContract;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::{Interpretation, Value, ValueData};

pub(crate) fn op_probe(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let ValueData::CodeBlock(tokens) = &value.data else {
        if !is_keep_mode {
            interp.stack.push(value);
        }
        return Err(AjisaiError::from("PROBE requires a CodeBlock"));
    };
    let tokens = tokens.clone();

    let contract = interp.infer_contract_for_block(&tokens);
    let result = knowledge_vector(&contract);
    interp
        .stack
        .push_with_role(result, Interpretation::Unassigned);
    Ok(())
}

fn purity_str(purity: crate::interpreter::word_contract::ContractPurity) -> &'static str {
    use crate::interpreter::word_contract::ContractPurity::*;
    match purity {
        Pure => "pure",
        Observable => "observable",
        Effectful => "effectful",
    }
}

fn determinism_str(
    determinism: crate::interpreter::word_contract::ContractDeterminism,
) -> &'static str {
    use crate::interpreter::word_contract::ContractDeterminism::*;
    match determinism {
        Deterministic => "deterministic",
        NonDeterministic => "nonDeterministic",
    }
}

fn nil_str(nil: crate::interpreter::word_contract::NilBehavior) -> &'static str {
    use crate::interpreter::word_contract::NilBehavior::*;
    match nil {
        NeverCreates => "neverCreates",
        Propagates => "propagates",
        MayCreate => "mayCreate",
        RejectsNil => "rejectsNil",
        ConsumesNil => "consumesNil",
    }
}

fn confidence_str(
    confidence: crate::interpreter::word_contract::ContractConfidence,
) -> &'static str {
    use crate::interpreter::word_contract::ContractConfidence::*;
    match confidence {
        Complete => "complete",
        Conservative => "conservative",
    }
}

fn pair(key: &str, value: Value) -> Value {
    Value::from_vector(vec![Value::from_string(key), value])
}

/// Six entries, chosen as the checkable subset `#:contract` declarations
/// verify against (purity, nil behavior) plus what makes "cannot verify"
/// readable as data rather than as an opaque failure (confidence, gaps):
/// arity and cost are deliberately not included in this first surface — see
/// docs/dev/ajisai-single-axis-proposal-2026-08.md §8 for why.
fn knowledge_vector(contract: &WordContract) -> Value {
    Value::from_vector(vec![
        pair("purity", Value::from_string(purity_str(contract.purity))),
        pair(
            "determinism",
            Value::from_string(determinism_str(contract.determinism)),
        ),
        pair("nil", Value::from_string(nil_str(contract.nil_behavior))),
        pair(
            "effects",
            Value::from_vector(
                contract
                    .effects
                    .iter()
                    .map(|effect| Value::from_string(effect))
                    .collect(),
            ),
        ),
        pair(
            "confidence",
            Value::from_string(confidence_str(contract.confidence)),
        ),
        pair(
            "gaps",
            Value::from_vector(
                contract
                    .gaps
                    .iter()
                    .map(|gap| Value::from_string(gap.as_str()))
                    .collect(),
            ),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Token;

    /// Reads PROBE's `[ 'key' value ]`-pair Vector into a lookup by key, so
    /// tests read as assertions about facts rather than about Vector shape.
    fn field<'a>(result: &'a Value, key: &str) -> &'a Value {
        result
            .as_vector()
            .expect("PROBE result is a Vector")
            .iter()
            .find_map(|pair| {
                let pair = pair.as_vector()?;
                (pair.first()?.as_text()? == key).then(|| &pair[1])
            })
            .unwrap_or_else(|| panic!("PROBE result has no `{key}` field"))
    }

    fn strings(value: &Value) -> Vec<&str> {
        value
            .as_vector()
            .expect("expected a Vector field")
            .iter()
            .map(|v| v.as_text().expect("expected a String element"))
            .collect()
    }

    #[tokio::test]
    async fn a_pure_block_reports_pure_and_complete() {
        let mut interp = Interpreter::new();
        interp.execute("{ 1 2 ADD } PROBE").await.unwrap();
        let result = interp.stack.last().cloned().unwrap();
        assert_eq!(field(&result, "purity").as_text(), Some("pure"));
        assert_eq!(field(&result, "confidence").as_text(), Some("complete"));
        assert_eq!(strings(field(&result, "effects")), Vec::<&str>::new());
        assert_eq!(strings(field(&result, "gaps")), Vec::<&str>::new());
    }

    #[tokio::test]
    async fn an_effectful_block_reports_its_effect_without_ever_running() {
        let mut interp = Interpreter::new();
        interp.execute("{ 42 PRINT } PROBE").await.unwrap();
        let result = interp.stack.last().cloned().unwrap();
        assert_eq!(field(&result, "purity").as_text(), Some("effectful"));
        assert_eq!(strings(field(&result, "effects")), vec!["consoleWrite"]);
        // The block was examined, not run: PRINT never fired, so the output
        // stream is empty even though PROBE just reported that this block
        // would write to it.
        assert!(interp.collect_output().is_empty());
    }

    #[tokio::test]
    async fn an_unresolved_dependency_is_a_gap_not_an_error() {
        let mut interp = Interpreter::new();
        interp
            .execute("{ TOTALLY-UNDEFINED-WORD } PROBE")
            .await
            .unwrap();
        let result = interp.stack.last().cloned().unwrap();
        assert_eq!(field(&result, "confidence").as_text(), Some("conservative"));
        assert_eq!(strings(field(&result, "gaps")), vec!["gap.unresolvedWord"]);
    }

    #[tokio::test]
    async fn an_empty_block_probes_to_a_trivial_contract() {
        let mut interp = Interpreter::new();
        interp.execute("{ } PROBE").await.unwrap();
        let result = interp.stack.last().cloned().unwrap();
        assert_eq!(field(&result, "purity").as_text(), Some("pure"));
        assert_eq!(field(&result, "confidence").as_text(), Some("complete"));
    }

    #[tokio::test]
    async fn non_codeblock_and_nil_operands_are_errors_with_the_operand_restored() {
        for source in ["1 PROBE", "1 KEEP PROBE", "NIL PROBE", "NIL KEEP PROBE"] {
            let mut interp = Interpreter::new();
            assert!(interp.execute(source).await.is_err(), "accepted {source}");
            assert_eq!(interp.stack.len(), 1, "operand was not restored: {source}");
        }
    }

    #[tokio::test]
    async fn keep_preserves_the_block_beneath_the_result() {
        let mut interp = Interpreter::new();
        interp.execute("{ 1 } KEEP PROBE").await.unwrap();
        assert_eq!(interp.stack.len(), 2);
        assert!(interp.stack.first().unwrap().is_code_block());
        assert!(interp.stack.last().unwrap().as_vector().is_some());
    }

    #[test]
    fn probing_never_mutates_the_dictionary() {
        let mut interp = Interpreter::new();
        let dictionary_epoch = interp.dictionary_epoch;
        let user_word_count = interp.user_words.len();
        interp
            .stack
            .push(Value::from_code_block(vec![Token::Symbol("ADD".into())]));

        op_probe(&mut interp).unwrap();

        assert_eq!(interp.dictionary_epoch, dictionary_epoch);
        assert_eq!(interp.user_words.len(), user_word_count);
    }
}
