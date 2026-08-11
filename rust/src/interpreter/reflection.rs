use crate::error::{AjisaiError, Result};
use crate::interpreter::cast::cast_value_helpers::apply_unary_cast;
use crate::interpreter::Interpreter;
use crate::types::code_data::{code_data_to_tokens, tokens_to_code_data};
use crate::types::{Value, ValueData};

pub(crate) fn op_reflect(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, reflect)
}

fn reflect(value: &Value) -> Result<Value> {
    match &value.data {
        ValueData::CodeBlock(tokens) => Ok(tokens_to_code_data(tokens)),
        ValueData::Vector(_) => Ok(Value::from_code_block(code_data_to_tokens(value)?)),
        _ => Err(AjisaiError::from(
            "REFLECT requires a CodeBlock or canonical code-data Vector",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::RuntimeLimits;
    use crate::types::{Interpretation, Token};
    use std::sync::Arc;

    #[tokio::test]
    async fn reflect_is_explicit_involutive_and_dictionary_independent() {
        let mut first = Interpreter::new();
        first
            .execute("{ FOO '🙂' 1/1 } REFLECT REFLECT")
            .await
            .unwrap();
        let reflected = first.stack.last().cloned().unwrap();
        assert!(reflected.is_code_block());
        assert!(first.user_words.is_empty());
        assert!(first.collect_output().is_empty());

        let mut second = Interpreter::new();
        second
            .execute("{ 1 } 'FOO' DEF { FOO '🙂' 1/1 } REFLECT REFLECT")
            .await
            .unwrap();
        assert_eq!(second.stack.last(), Some(&reflected));
        assert_eq!(second.user_words.len(), 1);
    }

    #[tokio::test]
    async fn eat_keep_and_failure_restore_follow_unary_cast_discipline() {
        let mut eat = Interpreter::new();
        eat.execute("{ 1 } REFLECT").await.unwrap();
        assert_eq!(eat.stack.len(), 1);
        assert!(eat.stack.last().unwrap().as_vector().is_some());

        let mut keep = Interpreter::new();
        keep.execute("{ 1 } KEEP REFLECT").await.unwrap();
        assert_eq!(keep.stack.len(), 2);
        assert!(keep.stack.first().unwrap().is_code_block());

        let mut failed = Interpreter::new();
        assert!(failed.execute("1 REFLECT").await.is_err());
        assert_eq!(failed.stack.last().and_then(Value::as_i64), Some(1));

        let mut keep_failed = Interpreter::new();
        assert!(keep_failed.execute("1 KEEP REFLECT").await.is_err());
        assert_eq!(keep_failed.stack.len(), 1);
        assert_eq!(keep_failed.stack.last().and_then(Value::as_i64), Some(1));
    }

    #[tokio::test]
    async fn rejects_every_non_reflectable_domain_and_noncanonical_vector() {
        for source in [
            "1 REFLECT",
            "TRUE REFLECT",
            "'text' REFLECT",
            "NIL REFLECT",
            "[ 1 2 3 ] REFLECT",
            "[ [ 1 2 ] [ 3 4 ] ] REFLECT",
        ] {
            let mut interp = Interpreter::new();
            assert!(interp.execute(source).await.is_err(), "accepted {source}");
        }
    }

    #[tokio::test]
    async fn malformed_data_and_nil_preserve_the_operand_on_failure() {
        let malformed = "[ 'AJISAI-CODE-1' [ 'number' 'not-a-number' ] ]";
        for source in [
            format!("{malformed} REFLECT"),
            format!("{malformed} KEEP REFLECT"),
            "NIL REFLECT".to_string(),
            "NIL KEEP REFLECT".to_string(),
        ] {
            let mut interp = Interpreter::new();
            assert!(interp.execute(&source).await.is_err(), "accepted {source}");
            assert_eq!(interp.stack.len(), 1, "operand was not restored: {source}");
        }
    }

    #[test]
    fn reflection_output_role_is_always_unassigned() {
        let mut interp = Interpreter::new();
        interp.stack.push_with_role(
            Value::from_code_block(vec![Token::Number("1".into())]),
            Interpretation::RawNumber,
        );
        interp
            .execute_section_core(&[Token::Symbol("REFLECT".into())], 0)
            .unwrap();
        assert_eq!(interp.stack.last_role(), Interpretation::Unassigned);
    }

    #[tokio::test]
    async fn reflected_results_match_through_a_compiled_user_word() {
        let mut direct = Interpreter::new();
        direct.execute("{ 1 ADD } REFLECT").await.unwrap();
        let expected = direct.stack.last().cloned().unwrap();

        let mut through_user_word = Interpreter::new();
        through_user_word
            .execute("{ REFLECT } 'REFLECT-THROUGH-USER' DEF { 1 ADD } REFLECT-THROUGH-USER")
            .await
            .unwrap();
        assert_eq!(through_user_word.stack.last(), Some(&expected));
        assert_eq!(
            through_user_word.stack.last_role(),
            Interpretation::Unassigned
        );
    }

    #[test]
    fn primitive_does_not_resolve_symbols_or_mutate_interpreter_state() {
        let mut interp = Interpreter::new();
        let dictionary_epoch = interp.dictionary_epoch;
        let body_store_len = interp.body_store.len();
        let dependency_count = interp.dependents.len();
        let resolve_cache_len = interp.resolve_cache.len();
        interp.stack.push(Value::from_code_block(vec![Token::Symbol(
            "UNRESOLVED-BY-REFLECT".into(),
        )]));

        op_reflect(&mut interp).unwrap();

        assert_eq!(interp.dictionary_epoch, dictionary_epoch);
        assert_eq!(interp.body_store.len(), body_store_len);
        assert_eq!(interp.dependents.len(), dependency_count);
        assert_eq!(interp.resolve_cache.len(), resolve_cache_len);
        assert!(interp.user_words.is_empty());
        assert!(interp.collect_output().is_empty());
    }

    #[tokio::test]
    async fn reflected_code_requires_exec_and_dynamic_def_shares_identity() {
        let data = "[ 'AJISAI-CODE-1' [ 'number' '1' ] [ 'symbol' 'ADD' ] ]";
        let mut interp = Interpreter::new();
        interp.execute(&format!("{data} REFLECT")).await.unwrap();
        assert!(interp.stack.last().unwrap().is_code_block());
        assert_eq!(interp.stack.len(), 1);
        interp.stack.clear();
        interp.execute("{ 1 ADD } 'LITERAL' DEF").await.unwrap();
        interp
            .execute(&format!("{data} REFLECT 'DYNAMIC' DEF"))
            .await
            .unwrap();
        let literal = &interp.user_words["LITERAL"].lines;
        let dynamic = &interp.user_words["DYNAMIC"].lines;
        assert!(Arc::ptr_eq(literal, dynamic));
        interp.execute("5 DYNAMIC").await.unwrap();
        assert_eq!(interp.stack.last().and_then(Value::as_i64), Some(6));
    }

    #[tokio::test]
    async fn reflected_numbers_cannot_bypass_exec_or_def_digit_limits() {
        let limits = RuntimeLimits {
            max_numeric_literal_digits: 4,
            ..RuntimeLimits::default()
        };
        let data = "[ 'AJISAI-CODE-1' [ 'number' '12345' ] ] REFLECT";

        let mut exec = Interpreter::new();
        exec.set_runtime_limits(limits);
        let error = exec.execute(&format!("{data} EXEC")).await.unwrap_err();
        assert!(error.to_string().contains("numericLiteralDigits"));

        let mut def = Interpreter::new();
        def.set_runtime_limits(limits);
        let error = def
            .execute(&format!("{data} 'TOO-BIG' DEF"))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("numericLiteralDigits"));
        assert!(!def.user_words.contains_key("TOO-BIG"));

        let mut higher_order = Interpreter::new();
        higher_order.set_runtime_limits(limits);
        let error = higher_order
            .execute(&format!("[ 1 ] {data} MAP"))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("numericLiteralDigits"));
    }
}
