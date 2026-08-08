use super::{require_exact, Tensor, TensorData, TensorMemoryBudget, TensorOperatorError};
use crate::types::fraction::{Fraction, RoundingMode};
use num_bigint::BigInt;

/// The tie rule is inherited from Core's `QUANTIZE`, which rounds halves away
/// from zero to match `ROUND`. A language should not hold two tie rules, so the
/// tensor operator does not introduce a second one.
const TIE_RULE: RoundingMode = RoundingMode::HalfAway;

/// Reference implementation of `tensor.regrid.v1`: round every element to the
/// nearest multiple of `1/denominator`.
///
/// This is the profile's denominator growth strategy, and it is the tensor
/// lifting of Core's `QUANTIZE` word. Exact arithmetic pays for exactness in
/// representation size: a rational's denominator records the whole history of
/// the computation that produced it, so a chain of operations grows one without
/// bound. Rounding back onto a declared grid keeps the representation the size
/// of the answer rather than the size of the computation.
///
/// Two properties make it worth doing at tensor rank rather than element by
/// element:
///
/// - Once every element of both operands shares the grid `1/d`, a contraction
///   of length `K` sums terms that already share the denominator `d²`. The
///   result's denominator is `d²` **whatever `K` is** — the reduction length
///   drops out. Without a shared grid the denominators are generically coprime
///   and the contraction's denominator is their product, which grows with `K`.
/// - Applying it once per layer holds the denominator at `d` across depth, so
///   representation size stops depending on how deep the graph is.
///
/// Rounding is exact and fully specified, so every backend must produce the
/// identical rational. That is the property the approximate dtypes cannot
/// offer, and the reason an exact graph is reproducible rather than merely
/// accurate.
pub fn tensor_regrid(
    input: &Tensor,
    denominator: &BigInt,
    budget: TensorMemoryBudget,
) -> Result<Tensor, TensorOperatorError> {
    require_exact("REGRID", input.dtype())?;
    if denominator <= &BigInt::from(0) {
        return Err(TensorOperatorError::NonPositiveDenominator);
    }
    let TensorData::Q(values) = input.data() else {
        unreachable!("exact dtype was checked")
    };
    let step = Fraction::new(BigInt::from(1), denominator.clone());
    let quantized = values
        .iter()
        .map(|value| value.quantize(&step, TIE_RULE).0)
        .collect();
    Tensor::new(
        input.shape().dimensions().to_vec(),
        TensorData::Q(quantized),
        budget,
    )
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor_profile::DType;

    const OPEN: TensorMemoryBudget = TensorMemoryBudget::new(usize::MAX, usize::MAX);

    fn q(numerator: i64, denominator: i64) -> Fraction {
        Fraction::new(BigInt::from(numerator), BigInt::from(denominator))
    }

    fn q_tensor(shape: Vec<usize>, values: Vec<Fraction>) -> Tensor {
        Tensor::new(shape, TensorData::Q(values), OPEN).unwrap()
    }

    #[test]
    fn rounds_every_element_onto_the_declared_grid() {
        let input = q_tensor(vec![3], vec![q(119, 125), q(32, 125), q(-119, 125)]);
        let result = tensor_regrid(&input, &BigInt::from(10), OPEN).unwrap();
        let TensorData::Q(values) = result.data() else {
            panic!("dtype changed")
        };
        // 0.952 -> 10/10, 0.256 -> 3/10, -0.952 -> -10/10.
        assert_eq!(values, &vec![q(1, 1), q(3, 10), q(-1, 1)]);
    }

    #[test]
    fn every_result_denominator_divides_the_grid() {
        let input = q_tensor(vec![4], vec![q(1, 7), q(2, 11), q(5, 13), q(9, 17)]);
        let result = tensor_regrid(&input, &BigInt::from(64), OPEN).unwrap();
        let TensorData::Q(values) = result.data() else {
            panic!("dtype changed")
        };
        for value in values {
            assert_eq!(BigInt::from(64) % value.denominator(), BigInt::from(0));
        }
    }

    #[test]
    fn ties_round_away_from_zero_exactly_as_core_quantize_does() {
        let input = q_tensor(vec![2], vec![q(1, 2), q(-1, 2)]);
        let result = tensor_regrid(&input, &BigInt::from(1), OPEN).unwrap();
        assert_eq!(result.data(), &TensorData::Q(vec![q(1, 1), q(-1, 1)]));
    }

    #[test]
    fn error_is_bounded_by_half_a_grid_step() {
        let denominator = BigInt::from(1000);
        let input = q_tensor(vec![3], vec![q(1, 7), q(22, 7), q(-355, 113)]);
        let result = tensor_regrid(&input, &denominator, OPEN).unwrap();
        let (TensorData::Q(before), TensorData::Q(after)) = (input.data(), result.data()) else {
            panic!("dtype changed")
        };
        let half_step = q(1, 2000);
        for (original, rounded) in before.iter().zip(after) {
            assert!(original.sub(rounded).abs() <= half_step);
        }
    }

    #[test]
    fn quantizing_is_idempotent_on_its_own_grid() {
        let input = q_tensor(vec![3], vec![q(1, 3), q(2, 7), q(9, 11)]);
        let once = tensor_regrid(&input, &BigInt::from(128), OPEN).unwrap();
        let twice = tensor_regrid(&once, &BigInt::from(128), OPEN).unwrap();
        assert_eq!(once.data(), twice.data());
    }

    #[test]
    fn rejects_a_non_positive_denominator() {
        let input = q_tensor(vec![1], vec![q(1, 3)]);
        for denominator in [BigInt::from(0), BigInt::from(-8)] {
            assert_eq!(
                tensor_regrid(&input, &denominator, OPEN),
                Err(TensorOperatorError::NonPositiveDenominator)
            );
        }
    }

    #[test]
    fn rejects_an_approximate_input_that_has_no_denominator_to_bound() {
        let input = Tensor::new(vec![2], TensorData::F32(vec![0.5, 0.25]), OPEN).unwrap();
        assert_eq!(
            tensor_regrid(&input, &BigInt::from(4), OPEN),
            Err(TensorOperatorError::ApproximateDTypeUnsupported {
                operator: "REGRID",
                dtype: DType::F32,
            })
        );
    }
}
