use crate::elastic::ElasticMode;
use crate::interpreter::higher_order::{
    execute_hedged_fold_kernel, execute_hedged_map_kernel, execute_hedged_predicate_kernel,
};
use crate::interpreter::quantized_block::quantize_code_block;
use crate::interpreter::Interpreter;
use crate::types::fraction::Fraction;
use crate::types::{Token, Value};

fn t_num(n: &str) -> Token {
    Token::Number(n.into())
}

fn t_sym(s: &str) -> Token {
    Token::Symbol(s.into())
}

fn map_mul2_tokens() -> Vec<Token> {
    vec![Token::VectorStart, t_num("2"), Token::VectorEnd, t_sym("*")]
}

fn predicate_lt2_tokens() -> Vec<Token> {
    vec![Token::VectorStart, t_num("2"), Token::VectorEnd, t_sym("<")]
}

fn fold_add_tokens() -> Vec<Token> {
    vec![t_sym("+")]
}

fn bump_dictionary_epoch(interp: &mut Interpreter) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        interp
            .execute("{ [ 1 ] + } 'INC_GUARD' DEF")
            .await
            .expect("define helper word");
    });
}

fn value_to_i64(v: &Value) -> i64 {
    if let Some(f) = v.as_scalar() {
        return f.to_i64().expect("scalar i64");
    }
    if v.len() == 1 {
        if let Some(child) = v.get_child(0) {
            if let Some(f) = child.as_scalar() {
                return f.to_i64().expect("inner scalar i64");
            }
        }
    }
    panic!("expected scalar or single-element vector");
}

#[test]
fn fast_guarded_guard_hit_uses_quantized_without_race() {
    let mut interp = Interpreter::new();
    interp.set_elastic_mode(ElasticMode::FastGuarded);

    let tokens = map_mul2_tokens();
    let qb = quantize_code_block(&tokens, &mut interp).expect("quantize block");

    let out = execute_hedged_map_kernel(
        &mut interp,
        "MAP",
        &qb,
        Some(tokens.as_slice()),
        Value::from_number(Fraction::from(3_i64)),
    )
    .expect("fast-guarded map should succeed");

    assert_eq!(value_to_i64(&out), 6);

    let m = interp.runtime_metrics();
    assert_eq!(m.hedged_race_started_count, 0);
    assert_eq!(m.hedged_race_fallback_count, 0);
    assert!(
        m.quantized_block_use_count >= 1,
        "guard hit should execute quantized path"
    );
}

#[test]
fn fast_guarded_guard_miss_falls_back_to_plain_without_race() {
    let mut interp = Interpreter::new();
    interp.set_elastic_mode(ElasticMode::FastGuarded);

    let tokens = map_mul2_tokens();
    let qb = quantize_code_block(&tokens, &mut interp).expect("quantize block");

    // Mutate dictionary epoch after quantization so the guard signature mismatches.
    bump_dictionary_epoch(&mut interp);

    let out = execute_hedged_map_kernel(
        &mut interp,
        "MAP",
        &qb,
        Some(tokens.as_slice()),
        Value::from_number(Fraction::from(3_i64)),
    )
    .expect("guard-miss fallback should succeed");

    assert_eq!(value_to_i64(&out), 6);

    let m = interp.runtime_metrics();
    assert_eq!(m.hedged_race_started_count, 0);
    assert!(
        m.hedged_race_fallback_count >= 1,
        "guard miss should record fallback"
    );
    assert!(
        interp
            .drain_hedged_trace()
            .iter()
            .any(|msg| msg.contains("fast-guarded:fallback")),
        "guard miss should emit fast-guarded fallback trace"
    );
}

#[test]
fn fast_guarded_predicate_guard_miss_falls_back_to_plain_without_race() {
    let mut interp = Interpreter::new();
    interp.set_elastic_mode(ElasticMode::FastGuarded);

    let tokens = predicate_lt2_tokens();
    let qb = quantize_code_block(&tokens, &mut interp).expect("quantize block");
    bump_dictionary_epoch(&mut interp);

    let out = execute_hedged_predicate_kernel(
        &mut interp,
        "FILTER",
        &qb,
        Some(tokens.as_slice()),
        Value::from_number(Fraction::from(1_i64)),
    )
    .expect("guard-miss predicate fallback should succeed");

    assert!(out, "1 < 2 should be true");
    let m = interp.runtime_metrics();
    assert_eq!(m.hedged_race_started_count, 0);
    assert!(m.hedged_race_fallback_count >= 1);
}

#[test]
fn fast_guarded_fold_guard_miss_falls_back_to_plain_without_race() {
    let mut interp = Interpreter::new();
    interp.set_elastic_mode(ElasticMode::FastGuarded);

    let tokens = fold_add_tokens();
    let qb = quantize_code_block(&tokens, &mut interp).expect("quantize block");
    bump_dictionary_epoch(&mut interp);

    let out = execute_hedged_fold_kernel(
        &mut interp,
        "FOLD",
        &qb,
        Some(tokens.as_slice()),
        Value::from_number(Fraction::from(10_i64)),
        Value::from_number(Fraction::from(5_i64)),
    )
    .expect("guard-miss fold fallback should succeed");

    assert_eq!(value_to_i64(&out), 15);
    let m = interp.runtime_metrics();
    assert_eq!(m.hedged_race_started_count, 0);
    assert!(m.hedged_race_fallback_count >= 1);
}
