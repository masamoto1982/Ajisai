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

#[test]
fn differential_vector_add_end_to_end() {
    // Phase 3, hierarchy A: the element-wise integer add wiring (which now
    // routes through the gated parallel dispatcher) must produce a result
    // bit-identical to the sequentially-computed reference (Same Result) and
    // identical across the quantized / force-no-quant paths. The kept size is
    // modest (well under the materialization limit) so parsing stays cheap; the
    // parallel==sequential contract itself is pinned by the kernel proptests in
    // `interpreter::parallel`.
    use crate::interpreter::simd_ops::extract_integer_vector;

    let n = 2048;
    let lhs: Vec<i64> = (0..n as i64).collect();
    let rhs: Vec<i64> = (0..n as i64).map(|x| x * 3 - 5).collect();
    let expected: Vec<i64> = lhs.iter().zip(rhs.iter()).map(|(a, b)| a + b).collect();

    let code = format!("{} {} +", vector_literal(&lhs), vector_literal(&rhs));
    let (quantized, plain) = run_with_both_paths(&code);
    assert_eq!(quantized, plain, "quantized vs plain diverged on vector add");

    let top = plain.last().expect("result on stack");
    let got = extract_integer_vector(top).expect("integer vector result");
    assert_eq!(got, expected, "vector add != sequential reference");
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

/// A vector literal of big-integer values written out in full decimal, used as
/// the exact (BigInt) reference for an overflowing i64 lane op.
fn big_vector_literal(values: &[i128]) -> String {
    let body = values
        .iter()
        .map(i128::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    format!("[ {} ]", body)
}

#[test]
fn differential_integer_add_overflow_stays_exact() {
    // 奇策本命: the speculative i64 lane must never silently wrap. When a lane
    // sum exceeds i64::MAX the engine falls back to the exact BigInt path, so
    // the result is the true mathematical value `i64::MAX + 1`, identical on
    // the quantized and force-no-quant paths. The vector is long enough
    // (>= SIMD_THRESHOLD) to engage the integer SIMD lane.
    let max = i64::MAX;
    let lhs = vec![max; 8];
    let rhs = vec![1i64; 8];
    let code = format!("{} {} +", vector_literal(&lhs), vector_literal(&rhs));
    let (quantized, plain) = run_with_both_paths(&code);
    assert_eq!(quantized, plain, "overflow add diverged across paths");

    // The exact result `i64::MAX + 1` cannot be an i64, so the fast-path
    // extractor must decline it — proving we did NOT wrap to i64::MIN.
    use crate::interpreter::simd_ops::extract_integer_vector;
    let top = plain.last().expect("result on stack");
    assert!(
        extract_integer_vector(top).is_none(),
        "result must be a big-integer lane, not a wrapped i64 vector"
    );

    // Strong value check: the result equals the exact big-integer reference
    // vector parsed from full-decimal literals (BigInt path), not a wrap.
    let expected_vals = vec![(max as i128) + 1; 8];
    let reference = run_with_quantization_mode(&big_vector_literal(&expected_vals), true);
    assert_eq!(
        plain.last(),
        reference.last(),
        "overflow add result must equal the exact BigInt reference"
    );
}

#[test]
fn differential_integer_mul_overflow_stays_exact() {
    // Same speculative-lowering contract for multiplication.
    let big = 1i64 << 62;
    let lhs = vec![big; 8];
    let rhs = vec![4i64; 8]; // big * 4 overflows i64
    let code = format!("{} {} *", vector_literal(&lhs), vector_literal(&rhs));
    let (quantized, plain) = run_with_both_paths(&code);
    assert_eq!(quantized, plain, "overflow mul diverged across paths");

    let expected_vals = vec![(big as i128) * 4; 8];
    let reference = run_with_quantization_mode(&big_vector_literal(&expected_vals), true);
    assert_eq!(
        plain.last(),
        reference.last(),
        "overflow mul result must equal the exact BigInt reference"
    );
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
