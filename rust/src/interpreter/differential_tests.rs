//! Cross-cutting test suite: quantized vs. non-quantized execution differential.

use crate::interpreter::Interpreter;
use crate::types::Stack;
use proptest::prelude::*;

fn run_with_quantization_mode(code: &str, force_no_quant: bool) -> Stack {
    let mut interp = Interpreter::new();
    interp.set_force_no_quant(force_no_quant);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        interp.execute(code).await.expect("code should execute");
    });

    interp.get_stack().clone()
}

pub fn run_with_both_paths(code: &str) -> (Stack, Stack) {
    (
        run_with_quantization_mode(code, false),
        run_with_quantization_mode(code, true),
    )
}

#[test]
fn differential_harness_smoke() {
    let code = "[ 1 2 3 4 ] { [ 1 ] + } MAP";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_arity_logic_and_compare() {
    // Element-wise logic over numeric truth lanes (1/0). Scalar TRUE/FALSE are
    // distinct truth values (finding B2) routed through K3; the dense
    // element-wise path operates on numeric 1/0 lanes, so this parity check
    // uses 1/0 vectors.
    let code = "[ 1 0 ] [ 1 1 ] AND";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_arity_lte_pair() {
    let code = "[ 1 ] [ 2 ] LTE";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_math_module_sqrt() {
    let code = "'math' IMPORT 4 MATH@SQRT";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_math_module_sqrt_eps_hyphen() {
    let code = "'math' IMPORT 2 1/100 MATH@SQRT-EPS";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_hof_with_pure_callback() {
    // Phase 1-B: a HOF with a pure callback should be classifiable as Pure
    // and produce identical results on both quantized and force-no-quant
    // paths.
    let code = "[ 1 2 3 4 ] { [ 2 ] * } MAP";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_hof_with_impure_user_callback() {
    // Phase 1-B PushCodeBlock recursion: a HOF whose callback calls an
    // impure user word (PRINT-then-pass-through) must produce identical
    // observable stack output on both paths even though the quantized
    // path falls back to the generic-compiled lane.
    let code = "{ ,, PRINT } 'TRACE' DEF [ 1 2 ] { TRACE } MAP";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_cond_pure_branches() {
    // COND is now a pure dispatcher; both guard and body blocks here are pure.
    let code = "[ 5 ] { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

fn vector_literal(values: &[i64]) -> String {
    let body = values
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    format!("[ {} ]", body)
}

proptest! {
    #[test]
    fn differential_elementwise_integer_vectors(
        lhs in proptest::collection::vec(-32i64..=32, 1..8),
        rhs in proptest::collection::vec(-32i64..=32, 1..8),
        op in prop_oneof![Just("+"), Just("-"), Just("*")],
    ) {
        let len = lhs.len().min(rhs.len());
        let code = format!("{} {} {}", vector_literal(&lhs[..len]), vector_literal(&rhs[..len]), op);
        let (quantized, plain) = run_with_both_paths(&code);
        prop_assert_eq!(quantized, plain, "program: {}", code);
    }
}

proptest! {
    #[test]
    fn differential_map_pure_integer_callback(
        values in proptest::collection::vec(-32i64..=32, 1..8),
        constant in -16i64..=16,
        op in prop_oneof![Just("+"), Just("-"), Just("*")],
    ) {
        let code = format!("{} {{ [ {} ] {} }} MAP", vector_literal(&values), constant, op);
        let (quantized, plain) = run_with_both_paths(&code);
        prop_assert_eq!(quantized, plain, "program: {}", code);
    }
}
