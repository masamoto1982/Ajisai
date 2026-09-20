//! `PROBE` — the pre-execution contract checker exposed as a Core Word.
//!
//! `ajisai check --contract` and `ajisai contract` already run this same
//! inference (`Interpreter::infer_word_contract`) over a named dictionary
//! Word, from outside the language. `PROBE` is the same operation reached
//! from inside it: narrowing a description (any Vector, since the CodeBlock/
//! Vector unification) into what can be known about it without running it,
//! over `Interpreter::infer_contract_for_block`. The answer is a Record in the
//! one contract shape `CONTRACT` also answers (`contract_record`). Narrowing
//! is total here — a well-formed operand always yields that Record, never
//! NIL — because the trichotomy this check reports (`LANG.CONTRACT.CHECK`:
//! verified / cannot verify / violated) lives inside the returned value, as
//! `confidence` and `gaps`, not in PROBE's own outcome category.

use crate::error::{AjisaiError, Result};
use crate::interpreter::contract_record::inferred_contract_record;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::Interpretation;

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

    // `as_vector_view` (Tensor-aware) — see control.rs's EXEC for why.
    let Some(elements) = value.as_vector_view() else {
        if !is_keep_mode {
            interp.stack.push(value);
        }
        return Err(AjisaiError::declared(
            "notExecutable",
            "PROBE requires a CodeBlock",
        ));
    };
    let tokens = match crate::interpreter::value_as_code::value_elements_to_tokens(&elements) {
        Ok(t) => t,
        Err(e) => {
            if !is_keep_mode {
                interp.stack.push(value);
            }
            return Err(e);
        }
    };

    let contract = interp.infer_contract_for_block(&tokens);
    let result = inferred_contract_record(&contract);
    interp
        .stack
        .push_with_role(result, Interpretation::Unassigned);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    /// Reads one field of PROBE's contract Record, so tests read as
    /// assertions about facts rather than about Record shape.
    fn field<'a>(result: &'a Value, key: &str) -> &'a Value {
        result
            .as_record()
            .expect("PROBE result is a Record")
            .get(&Value::from_string(key))
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
        interp.execute("[ 1 2 ADD ] PROBE").await.unwrap();
        let result = interp.stack.last().cloned().unwrap();
        assert_eq!(field(&result, "purity").as_text(), Some("pure"));
        assert_eq!(field(&result, "confidence").as_text(), Some("complete"));
        assert_eq!(strings(field(&result, "effects")), Vec::<&str>::new());
        assert_eq!(strings(field(&result, "gaps")), Vec::<&str>::new());
    }

    #[tokio::test]
    async fn an_effectful_block_reports_its_effect_without_ever_running() {
        let mut interp = Interpreter::new();
        interp.execute("[ 42 PRINT ] PROBE").await.unwrap();
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
            .execute("[ TOTALLY-UNDEFINED-WORD ] PROBE")
            .await
            .unwrap();
        let result = interp.stack.last().cloned().unwrap();
        assert_eq!(field(&result, "confidence").as_text(), Some("conservative"));
        assert_eq!(strings(field(&result, "gaps")), vec!["gap.unresolvedWord"]);
    }

    #[tokio::test]
    async fn an_empty_block_probes_to_a_trivial_contract() {
        let mut interp = Interpreter::new();
        interp.execute("[ ] PROBE").await.unwrap();
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
        interp.execute("[ 1 ] KEEP PROBE").await.unwrap();
        assert_eq!(interp.stack.len(), 2);
        assert!(interp.stack.first().unwrap().as_vector_view().is_some());
        assert!(interp.stack.last().unwrap().as_record().is_some());
    }

    #[test]
    fn probing_never_mutates_the_dictionary() {
        let mut interp = Interpreter::new();
        let dictionary_epoch = interp.dictionary_epoch;
        let user_word_count = interp.user_words.len();
        interp
            .stack
            .push(Value::from_vector_promoted(vec![Value::from_symbol("ADD")]));

        op_probe(&mut interp).unwrap();

        assert_eq!(interp.dictionary_epoch, dictionary_epoch);
        assert_eq!(interp.user_words.len(), user_word_count);
    }
}
