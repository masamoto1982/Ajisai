//! Shape Words: `ZIP`, `SUM`, `PUT`, and the seeded generator `RANDOM`.
//!
//! The companion module to `ordering_ops`, on the same reasoning: each of these
//! is expressible in the existing vocabulary, and each one written that way
//! costs asymptotically more, or (for `RANDOM`) is not expressible at all
//! because the language has no bit mixing to build a generator out of.

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::extract_integer_from_value;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value};
use num_bigint::BigInt;

use super::ordering_ops::{elements_of, restore, take_operand};

/// `ZIP ( [ [ vec... ] ] -> [ [ tuple... ] ] )`: bundle equal-length vectors
/// position by position.
///
/// Three jobs in one Word. Walking features beside labels is
/// `XS YS 2 COLLECT ZIP { .. } MAP`; transposing a matrix is `M ZIP`; and
/// walking a vector with its own indices is
/// `0 N RANGE XS 2 COLLECT ZIP`. All three previously went through
/// `1 COLLECT 'IV' BIND XS IV GET`, the index-plumbing phrase that made up
/// roughly half the tokens of any program that had to look two things up at
/// the same position.
pub fn op_zip(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    if value.is_nil() {
        interp.stack.push(value);
        return Ok(());
    }

    let rows = match elements_of(&value, "vector of vectors") {
        Ok(rows) => rows,
        Err(e) => {
            restore(interp, value);
            return Err(e);
        }
    };

    if rows.is_empty() {
        interp.stack.push(Value::from_vector(Vec::new()));
        return Ok(());
    }

    let mut columns: Vec<Vec<Value>> = Vec::new();
    for row in &rows {
        match row.as_vector_view() {
            Some(view) => columns.push(view.into_owned()),
            None => {
                restore(interp, value);
                return Err(AjisaiError::create_structure_error(
                    "vector of vectors",
                    "vector holding a non-vector element",
                ));
            }
        }
    }

    let width = columns[0].len();
    if let Some(other) = columns.iter().find(|column| column.len() != width) {
        restore(interp, value);
        return Err(AjisaiError::VectorLengthMismatch {
            len1: width,
            len2: other.len(),
        });
    }

    // A transpose copies every cell exactly once, so the price is the whole
    // grid: rows x width, measured on the rows rather than on the outer vector
    // (whose "elements" are the rows themselves).
    let cell_units = columns
        .iter()
        .map(|column| {
            crate::interpreter::collection_meter::element_cost_of_slice(column).copies(column.len())
        })
        .fold(0u64, u64::saturating_add);
    if let Err(e) = crate::interpreter::collection_meter::charge(interp, cell_units) {
        restore(interp, value);
        return Err(e);
    }

    let out: Vec<Value> = (0..width)
        .map(|position| {
            Value::from_vector(
                columns
                    .iter()
                    .map(|column| column[position].clone())
                    .collect(),
            )
        })
        .collect();
    interp
        .stack
        .push_with_role(Value::from_vector(out), Interpretation::Unassigned);
    Ok(())
}

/// `SUM ( [ vec ] -> [ total ] )`: fold the outermost axis with `ADD`.
///
/// `0 { ADD } FOLD` says the same thing in four tokens, and those four tokens
/// appear in every inner product, mean, variance, norm, loss and Gini in any
/// program that computes anything. Naming it is worth one Core slot on
/// frequency alone; it also keeps a summation on the scalar fast path instead
/// of re-entering the interpreter once per element.
///
/// Folding the *outer* axis is what makes a centroid one call:
/// `[ [ 1 2 ] [ 3 4 ] ] SUM` is `[ 4/1 6/1 ]`.
pub fn op_sum(interp: &mut Interpreter) -> Result<()> {
    let value = take_operand(interp)?;
    if value.is_nil() {
        interp.stack.push(value);
        return Ok(());
    }

    let items = match elements_of(&value, "vector") {
        Ok(items) => items,
        Err(e) => {
            restore(interp, value);
            return Err(e);
        }
    };

    // The empty sum is zero, which is what `0 { ADD } FOLD` answers and what
    // makes `SUM` a fold rather than a partial operation.
    let mut total = Value::from_int(0);
    for item in &items {
        match crate::interpreter::arithmetic::add_values_metered(interp, &total, item) {
            Ok(next) => total = next,
            Err(e) => {
                restore(interp, value);
                return Err(e);
            }
        }
    }
    interp
        .stack
        .push_with_role(total, Interpretation::RawNumber);
    Ok(())
}

/// `PUT ( [ vec ] [ idx ] [ value ] -> [ vec' ] )`: a copy of `vec` with the
/// element at `idx` replaced. A negative index counts from the end, as
/// everywhere else.
///
/// The alternative was rebuilding the whole vector through a `MAP` and a
/// `COND` per element, or the `TAKE`/`CONCAT` surgery that spells the same
/// thing in three phrases. Updating one position is what a parameter step, a
/// centroid move, a confusion-matrix increment and a histogram bin all do.
pub fn op_put(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 3 {
        return Err(AjisaiError::StackUnderflow);
    }
    let keep = interp.consumption_mode == ConsumptionMode::Keep;
    let replacement = interp.stack.pop().expect("checked by len()");
    let index_value = interp.stack.pop().expect("checked by len()");
    let target = interp.stack.pop().expect("checked by len()");

    let restore_all = |interp: &mut Interpreter, target: Value, index: Value, value: Value| {
        if keep {
            interp.stack.push(target);
            interp.stack.push(index);
            interp.stack.push(value);
        }
    };

    // `PUT` answers with a copy of the vector, one element replaced, so it
    // copies the whole thing however small the edit is.
    if let Err(e) =
        crate::interpreter::collection_meter::charge_copy_of(interp, &target, target.len())
    {
        if !keep {
            interp.stack.push(target);
            interp.stack.push(index_value);
            interp.stack.push(replacement);
        }
        return Err(e);
    }

    let mut items = match target.as_vector_view() {
        Some(view) => view.into_owned(),
        None => {
            if !keep {
                interp.stack.push(target);
                interp.stack.push(index_value);
                interp.stack.push(replacement);
            }
            return Err(AjisaiError::create_structure_error(
                "vector",
                "non-vector value",
            ));
        }
    };

    let raw_index = match extract_integer_from_value(&index_value) {
        Ok(index) => index,
        Err(e) => {
            if !keep {
                interp.stack.push(target);
                interp.stack.push(index_value);
                interp.stack.push(replacement);
            }
            return Err(e);
        }
    };

    let length = items.len();
    let position = if raw_index < 0 {
        length as i64 + raw_index
    } else {
        raw_index
    };
    if position < 0 || position as usize >= length {
        if !keep {
            interp.stack.push(target);
            interp.stack.push(index_value);
            interp.stack.push(replacement);
        }
        return Err(AjisaiError::IndexOutOfBounds {
            index: raw_index,
            length,
        });
    }

    items[position as usize] = replacement.clone();
    restore_all(interp, target, index_value, replacement);
    interp
        .stack
        .push_with_role(Value::from_vector(items), Interpretation::Unassigned);
    Ok(())
}

/// The generator behind `RANDOM`: SplitMix64, run over the seed.
///
/// Chosen because it is a pure function of a 64-bit state with no warm-up and
/// no hidden state to carry between calls, which is the whole requirement
/// here — the Word must be a function of its operands like every other.
fn split_mix_64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The number of distinct values a drawn rational may take. A power of two, so
/// every draw is an exact rational with a small denominator rather than a
/// ratio nobody can read.
const RANDOM_DENOMINATOR: u64 = 1 << 53;

/// `RANDOM ( [ seed ] [ count ] -> [ rationals ] )`: `count` exact rationals
/// in [0, 1), determined entirely by `seed`.
///
/// The vocabulary's refusal of transcendental functions has a reason — no
/// exact answer to them exists — and that reason does not reach randomness. A
/// seeded generator is a pure, deterministic function of its operands and
/// returns exact rationals, so it violates neither exactness nor purity, and
/// without it shuffling, train/test splitting, k-fold cross-validation,
/// random initialization, stochastic descent, bootstrapping and k-means seeding
/// are all unwritable.
///
/// Composes with `ORDER`: `SEED N RANDOM ORDER` is a shuffle permutation.
pub fn op_random(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    let keep = interp.consumption_mode == ConsumptionMode::Keep;
    let count_value = interp.stack.pop().expect("checked by len()");
    let seed_value = interp.stack.pop().expect("checked by len()");

    let put_back = |interp: &mut Interpreter, seed: &Value, count: &Value| {
        if !keep {
            interp.stack.push(seed.clone());
            interp.stack.push(count.clone());
        }
    };

    let seed = match extract_integer_from_value(&seed_value) {
        Ok(seed) => seed,
        Err(e) => {
            put_back(interp, &seed_value, &count_value);
            return Err(e);
        }
    };
    let count = match extract_integer_from_value(&count_value) {
        Ok(count) => count,
        Err(e) => {
            put_back(interp, &seed_value, &count_value);
            return Err(e);
        }
    };
    if count < 0 {
        put_back(interp, &seed_value, &count_value);
        return Err(AjisaiError::create_structure_error(
            "a count of zero or more",
            "a negative count",
        ));
    }

    // The same space water level `RANGE` and `FILL` observe: a well-formed
    // request that cannot be materialized within budget projects onto NIL
    // with reason `spaceExhausted`, recoverable with `^`, rather than driving
    // the host into an allocation failure.
    if count as usize > interp.runtime_limits.max_materialized_elements {
        if keep {
            interp.stack.push(seed_value);
            interp.stack.push(count_value);
        }
        interp
            .stack
            .push(Value::nil_with_reason(NilReason::SpaceExhausted));
        return Ok(());
    }

    if let Err(e) =
        crate::interpreter::collection_meter::charge_materialization(interp, count as usize)
    {
        put_back(interp, &seed_value, &count_value);
        return Err(e);
    }

    let mut state = seed as u64;
    let denominator = BigInt::from(RANDOM_DENOMINATOR);
    let draws: Vec<Value> = (0..count)
        .map(|_| {
            let raw = split_mix_64(&mut state) % RANDOM_DENOMINATOR;
            // `Fraction::new` reduces, so a draw whose numerator shares a
            // factor with 2^53 comes back in lowest terms — `1/2`, not
            // `4503599627370496/9007199254740992`.
            Value::from_fraction(Fraction::new(BigInt::from(raw), denominator.clone()))
        })
        .collect();

    if keep {
        interp.stack.push(seed_value);
        interp.stack.push(count_value);
    }
    interp
        .stack
        .push_with_role(Value::from_vector(draws), Interpretation::Unassigned);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    /// The projection probe `nil_conformance_tests::projecting_word_set_matches_registry`
    /// requires of every Word that declares a projection condition. `RANDOM`
    /// projects past the space water level, as `RANGE` and `FILL` do: a count
    /// the host cannot materialize is a well-formed request rather than a
    /// malformed one, so it projects onto NIL and stays recoverable with `^`.
    #[tokio::test]
    async fn random_projects_past_the_space_water_level() {
        let mut interp = Interpreter::new();
        interp.execute("7 99999999 RANDOM").await.unwrap();
        let answer = interp.stack.last().cloned().expect("an answer was pushed");
        assert!(answer.is_nil(), "{answer:?}");
        assert_eq!(
            answer
                .absence_metadata()
                .and_then(|absence| absence.reason.as_ref())
                .map(|reason| reason.as_protocol_str()),
            Some("spaceExhausted")
        );

        let mut malformed = Interpreter::new();
        assert!(malformed.execute("7 'ten' RANDOM").await.is_err());
    }
}
