//! Measurements of the property the denominator growth strategy exists to
//! control. Exact arithmetic is only usable if representation size stays a
//! function of the answer rather than of the computation, and these tests
//! measure that directly instead of asserting it in prose.
//!
//! See `docs/dev/rational-tensor-growth-2026-08.md` for the analysis.

use super::*;
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use std::collections::BTreeMap;

const OPEN: TensorMemoryBudget = TensorMemoryBudget::new(usize::MAX, usize::MAX);

fn q(numerator: i64, denominator: i64) -> Fraction {
    Fraction::new(BigInt::from(numerator), BigInt::from(denominator))
}

fn q_tensor(shape: Vec<usize>, values: Vec<Fraction>) -> Tensor {
    Tensor::new(shape, TensorData::Q(values), OPEN).unwrap()
}

/// The widest denominator in the tensor, in bits — the representation cost the
/// strategy is trying to bound.
fn denominator_bits(tensor: &Tensor) -> u64 {
    let TensorData::Q(values) = tensor.data() else {
        panic!("not an exact tensor")
    };
    values
        .iter()
        .map(|value| value.denominator().bits())
        .max()
        .unwrap_or(0)
}

/// Distinct primes, so no two denominators share a factor and nothing cancels.
/// This is the generic case for rationals that were never put on a grid.
const PRIMES: [i64; 64] = [
    3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193,
    197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293, 307,
    311, 313,
];

fn ungridded_row(length: usize, offset: usize) -> Vec<Fraction> {
    (0..length).map(|i| q(1, PRIMES[offset + i])).collect()
}

/// Numerators over a common denominator `d`: every element is on the grid 1/d.
fn gridded_row(length: usize, d: i64) -> Vec<Fraction> {
    (0..length).map(|i| q(2 * (i as i64 % 7) + 1, d)).collect()
}

fn contraction(left: Vec<Fraction>, right: Vec<Fraction>) -> Tensor {
    let k = left.len();
    matmul(
        &q_tensor(vec![1, k], left),
        &q_tensor(vec![k, 1], right),
        OPEN,
    )
    .unwrap()
}

#[test]
fn exact_arithmetic_has_no_rounding_error_to_be_free_of() {
    // The float identity that famously fails, and its exact counterpart.
    assert_ne!(0.1f64 + 0.2f64, 0.3f64);
    let sum = tensor_add(
        &q_tensor(vec![1], vec![q(1, 10)]),
        &q_tensor(vec![1], vec![q(2, 10)]),
        OPEN,
    )
    .unwrap();
    assert_eq!(sum.data(), &TensorData::Q(vec![q(3, 10)]));
}

/// Without a shared grid, a contraction's denominator is the product of its
/// terms' denominators, so representation cost grows with the reduction length.
#[test]
fn ungridded_contraction_denominator_grows_with_reduction_length() {
    let measure =
        |k: usize| denominator_bits(&contraction(ungridded_row(k, 0), ungridded_row(k, 24)));
    let (two, eight, twenty_four) = (measure(2), measure(8), measure(24));
    assert!(
        two < eight && eight < twenty_four,
        "expected growth with K, measured {two} < {eight} < {twenty_four}"
    );
    // Denominator bits accumulate per term, so the cost is about linear in K.
    assert!(
        twenty_four > 8 * two,
        "K=24 reached only {twenty_four} bits against {two} at K=2"
    );
}

/// The core of the strategy. Once both operands are on the grid 1/d, every
/// product already shares the denominator d², so the sum has denominator d²
/// whatever the reduction length is — the reduction length drops out.
#[test]
fn a_shared_grid_makes_contraction_denominator_independent_of_length() {
    const D: i64 = 256;
    let squared_grid_bits = BigInt::from(D * D).bits();
    for k in [2usize, 8, 32, 64] {
        let bits = denominator_bits(&contraction(gridded_row(k, D), gridded_row(k, D)));
        assert!(
            bits <= squared_grid_bits,
            "K={k} produced {bits} denominator bits, above the d² bound {squared_grid_bits}"
        );
    }
    // The same contraction length without a shared grid is an order of
    // magnitude more expensive to represent, and keeps getting worse with K.
    let gridded = denominator_bits(&contraction(gridded_row(24, D), gridded_row(24, D)));
    let ungridded = denominator_bits(&contraction(ungridded_row(24, 0), ungridded_row(24, 24)));
    assert!(
        ungridded > 10 * gridded,
        "gridded {gridded} bits vs ungridded {ungridded} bits at K=24"
    );
}

/// A grid bounds one contraction, but chaining squares the denominator each
/// time, so depth alone still runs away. Quantizing per layer is what makes the
/// cost constant.
///
/// The measured growth is milder than the d^(2^L) worst case because a
/// power-of-two grid lets sums cancel trailing zeros. Milder is not bounded:
/// the unquantized chain keeps climbing, and only the quantized one holds.
#[test]
fn quantizing_each_layer_holds_the_denominator_constant_across_depth() {
    const D: i64 = 16;
    const LAYERS: usize = 8;
    let grid_bits = BigInt::from(D).bits();
    let weights = q_tensor(vec![2, 2], gridded_row(4, D));

    let mut unquantized = q_tensor(vec![2, 2], gridded_row(4, D));
    let mut quantized = unquantized.clone();
    let mut unquantized_bits = vec![denominator_bits(&unquantized)];
    for _ in 0..LAYERS {
        unquantized = matmul(&unquantized, &weights, OPEN).unwrap();
        unquantized_bits.push(denominator_bits(&unquantized));

        quantized = matmul(&quantized, &weights, OPEN).unwrap();
        quantized = tensor_regrid(&quantized, &BigInt::from(D), OPEN).unwrap();
        assert!(
            denominator_bits(&quantized) <= grid_bits,
            "quantized layer exceeded the declared grid"
        );
    }
    assert!(
        unquantized_bits.windows(2).all(|pair| pair[1] >= pair[0]),
        "denominator cost should never fall without quantizing: {unquantized_bits:?}"
    );
    assert!(
        *unquantized_bits.last().unwrap() >= 3 * unquantized_bits[0],
        "unquantized depth should compound: {unquantized_bits:?}"
    );
}

/// Rounding onto a grid of step 1/d moves each element by at most 1/(2d). The
/// bound is exact and declared, not a statistical expectation.
#[test]
fn quantization_error_is_bounded_by_half_a_grid_step() {
    const D: i64 = 512;
    let input = q_tensor(vec![8], ungridded_row(8, 0));
    let rounded = tensor_regrid(&input, &BigInt::from(D), OPEN).unwrap();
    let (TensorData::Q(before), TensorData::Q(after)) = (input.data(), rounded.data()) else {
        panic!("dtype changed")
    };
    let half_step = q(1, 2 * D);
    for (original, quantized) in before.iter().zip(after) {
        assert!(original.sub(quantized).abs() <= half_step);
    }
}

/// The enforcement half of the strategy. Exact arithmetic's classic failure
/// mode is not a wrong answer but an unbounded one: the computation simply
/// stops finishing. A declared denominator ceiling converts that into a report
/// naming the operation that overflowed it.
#[test]
fn the_denominator_budget_reports_unbounded_growth_instead_of_absorbing_it() {
    let budget = TensorMemoryBudget::new(usize::MAX, usize::MAX).with_denominator_bits(24);
    let row = q_tensor(vec![1, 16], ungridded_row(16, 0));
    let column = q_tensor(vec![16, 1], ungridded_row(16, 16));
    assert!(matches!(
        matmul(&row, &column, budget),
        Err(TensorOperatorError::Tensor(
            TensorError::DenominatorBudgetExceeded { limit: 24, .. }
        ))
    ));
}

/// The same computation, under the same ceiling, with the strategy applied.
#[test]
fn quantizing_keeps_the_same_computation_inside_the_same_ceiling() {
    const D: i64 = 256;
    let budget = TensorMemoryBudget::new(usize::MAX, usize::MAX).with_denominator_bits(24);
    let row = q_tensor(vec![1, 16], gridded_row(16, D));
    let column = q_tensor(vec![16, 1], gridded_row(16, D));
    let product = matmul(&row, &column, budget).unwrap();
    let bounded = tensor_regrid(&product, &BigInt::from(D), budget).unwrap();
    assert!(denominator_bits(&bounded) <= BigInt::from(D).bits());
}

/// An exact tensor never carries Core's absence marker: a NIL fraction is not
/// a number and cannot enter a profile value.
#[test]
fn a_nil_fraction_is_refused_entry_to_an_exact_tensor() {
    assert_eq!(
        Tensor::new(vec![2], TensorData::Q(vec![q(1, 2), Fraction::nil()]), OPEN),
        Err(TensorError::NilExactElement(1))
    );
}

/// Division by zero is where the exact and approximate domains genuinely
/// differ: a float gets an infinity that stays in the tensor, a rational has
/// no value at all and the operation is reported.
#[test]
fn exact_division_by_zero_is_reported_rather_than_given_a_value() {
    assert_eq!(
        tensor_div(
            &q_tensor(vec![2], vec![q(1, 1), q(2, 1)]),
            &q_tensor(vec![2], vec![q(1, 2), q(0, 1)]),
            OPEN,
        ),
        Err(TensorOperatorError::ExactDivisionByZero)
    );
}

/// REDUCE_MAX leans on negative infinity as its identity. The rationals have
/// no such element, so an empty exact slice is reported instead of answered.
#[test]
fn an_empty_exact_maximum_has_no_identity_to_return() {
    let empty = q_tensor(vec![2, 0], vec![]);
    assert_eq!(
        reduce_max(&empty, &[1], false, OPEN),
        Err(TensorOperatorError::EmptyExactReduction {
            operator: "REDUCE_MAX"
        })
    );
    // A sum does have one, so it answers zero.
    assert_eq!(
        reduce_sum(&empty, &[1], false, OPEN).unwrap().data(),
        &TensorData::Q(vec![q(0, 1), q(0, 1)])
    );
}

/// Operators whose value is irrational for rational inputs are undefined over
/// the exact dtype rather than silently returning an approximation.
#[test]
fn transcendental_operators_are_undefined_over_the_exact_dtype() {
    let input = q_tensor(vec![2], vec![q(1, 2), q(2, 1)]);
    for (operator, result) in [
        ("EXP", tensor_exp(&input, OPEN)),
        ("LOG", tensor_log(&input, OPEN)),
        ("RSQRT", tensor_rsqrt(&input, OPEN)),
    ] {
        assert_eq!(
            result,
            Err(TensorOperatorError::ExactDTypeUnsupported {
                operator,
                dtype: DType::Q,
            }),
            "{operator} should be undefined over Q"
        );
    }
}

/// End to end: the same graph shape with and without a QUANTIZE node, under a
/// declared denominator ceiling. One completes; the other reports which node
/// overflowed. This is the strategy as a graph property, not a local trick.
#[test]
fn a_graph_stays_inside_its_ceiling_only_when_it_quantizes() {
    const D: u64 = 64;
    let budget = TensorMemoryBudget::new(usize::MAX, usize::MAX).with_denominator_bits(13);
    let exact = |shape: &[usize]| GraphType::Tensor {
        dtype: DType::Q,
        shape: shape
            .iter()
            .copied()
            .map(SymbolicDimension::Known)
            .collect(),
    };
    let value = |id: &str, shape: &[usize]| GraphValue {
        id: id.to_owned(),
        value_type: exact(shape),
    };
    let context = GraphValidationContext {
        profile_id: "org.ajisai.tensor/0.1".to_owned(),
        operator_semantics: BTreeMap::from([
            ("tensor.matmul.v1".to_owned(), OperatorSemantics::Matmul),
            ("tensor.regrid.v1".to_owned(), OperatorSemantics::Regrid),
        ]),
    };
    let matmul_node = |id: &str, left: &str, right: &str, out: &str| GraphNode {
        id: id.to_owned(),
        operator_semantic_id: "tensor.matmul.v1".to_owned(),
        inputs: vec![left.to_owned(), right.to_owned()],
        outputs: vec![value(out, &[2, 2])],
        attributes: BTreeMap::new(),
    };
    let quantize_node = |id: &str, input: &str, out: &str| GraphNode {
        id: id.to_owned(),
        operator_semantic_id: "tensor.regrid.v1".to_owned(),
        inputs: vec![input.to_owned()],
        outputs: vec![value(out, &[2, 2])],
        attributes: BTreeMap::from([("denominator".to_owned(), serde_json::json!(D))]),
    };
    let inputs = BTreeMap::from([
        (
            "%activations".to_owned(),
            q_tensor(vec![2, 2], gridded_row(4, D as i64)),
        ),
        (
            "%weights".to_owned(),
            q_tensor(vec![2, 2], gridded_row(4, D as i64)),
        ),
    ]);
    let declarations = vec![value("%activations", &[2, 2]), value("%weights", &[2, 2])];

    let unquantized = Graph {
        schema_version: 1,
        profiles: vec!["org.ajisai.tensor/0.1".to_owned()],
        inputs: declarations.clone(),
        nodes: vec![
            matmul_node("@layer1", "%activations", "%weights", "%hidden"),
            matmul_node("@layer2", "%hidden", "%weights", "%output"),
        ],
        outputs: vec!["%output".to_owned()],
        artifacts: vec![],
    };
    assert!(matches!(
        execute_graph(&unquantized, &context, &inputs, budget),
        Err(GraphExecutionError::Operator(TensorOperatorError::Tensor(
            TensorError::DenominatorBudgetExceeded { limit: 13, .. }
        )))
    ));

    let quantized = Graph {
        nodes: vec![
            matmul_node("@layer1", "%activations", "%weights", "%raw1"),
            quantize_node("@bound1", "%raw1", "%hidden"),
            matmul_node("@layer2", "%hidden", "%weights", "%raw2"),
            quantize_node("@bound2", "%raw2", "%output"),
        ],
        ..unquantized
    };
    let outputs = execute_graph(&quantized, &context, &inputs, budget).unwrap();
    assert!(denominator_bits(&outputs["%output"]) <= BigInt::from(D).bits());
}
