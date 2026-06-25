use crate::types::{Value, ValueData};
use std::borrow::Cow;

const SIMD_THRESHOLD: usize = 8;

/// Borrow (or, when borrowing is impossible, materialize) the integer lane of a
/// `Value` as `Cow<[i64]>`.
///
/// The hot case is a 1-D pure-integer dense `Tensor` with no `nil` holes: its
/// `numerators` column is *already* the `&[i64]` the kernel wants, so we hand
/// out a `Cow::Borrowed` with zero allocation and zero per-element `Fraction`
/// construction (handoff 手1 — stop the representation round-trip). A dense
/// tensor's numerators are guaranteed to fit `i64` (otherwise it would never
/// have been densified), so the borrow is bit-identical to the old
/// element-by-element `to_i64()` extraction.
///
/// `Vector` inputs (AoS) and any tensor that is rational or has `nil` lanes
/// fall back to the owned path, which preserves the previous behavior of
/// declining (returning `None`) on the first non-integer / non-`i64` lane.
pub(crate) fn extract_integer_lane(val: &Value) -> Option<Cow<'_, [i64]>> {
    match &val.data {
        ValueData::Tensor { data, shape } => {
            if shape.len() != 1 || data.len() < SIMD_THRESHOLD {
                return None;
            }
            if data.is_pure_integer && data.all_lanes_valid() {
                return Some(Cow::Borrowed(data.numerators.as_slice()));
            }
            let mut result: Vec<i64> = Vec::with_capacity(data.len());
            for f in data.iter() {
                if !f.is_integer() {
                    return None;
                }
                result.push(f.to_i64()?);
            }
            Some(Cow::Owned(result))
        }
        ValueData::Vector(children) => {
            if children.len() < SIMD_THRESHOLD {
                return None;
            }
            let mut result: Vec<i64> = Vec::with_capacity(children.len());
            for child in children.iter() {
                match &child.data {
                    ValueData::Scalar(f) if f.is_integer() => result.push(f.to_i64()?),
                    _ => return None,
                }
            }
            Some(Cow::Owned(result))
        }
        ValueData::Boolean(_)
        | ValueData::Scalar(_)
        | ValueData::ExactScalar(_)
        | ValueData::Record { .. }
        | ValueData::Nil
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => None,
    }
}

/// Owned-result wrapper retained for tests. Prefer [`extract_integer_lane`] in
/// hot paths to keep the borrow.
#[cfg(test)]
pub fn extract_integer_vector(val: &Value) -> Option<Vec<i64>> {
    extract_integer_lane(val).map(|lane| lane.into_owned())
}

/// Build the SoA result of an integer-lane op: a 1-D pure-integer dense
/// `Tensor`, keeping the dense column representation rather than degrading to an
/// AoS `Vector` of boxed `Value`s (handoff 手1).
pub fn create_value_from_integer_vector(values: Vec<i64>) -> Value {
    Value::from_int_tensor(values)
}

// ── Overflow-checked sequential lanes ───────────────────────────────────────
//
// Speculative `i64` lowering (handoff 奇策本命): the result is bit-identical to
// the exact rational answer *iff* no lane overflows. These lanes return `None`
// the moment a lane overflows so the caller can fall back to the exact
// `Fraction`/BigInt path. They are the below-threshold / wasm fallback and the
// element-wise contract partner of the checked parallel kernels.

fn checked_lane_add(a: &[i64], b: &[i64]) -> Option<Vec<i64>> {
    let mut out = Vec::with_capacity(a.len());
    for (&x, &y) in a.iter().zip(b.iter()) {
        out.push(x.checked_add(y)?);
    }
    Some(out)
}

fn checked_lane_sub(a: &[i64], b: &[i64]) -> Option<Vec<i64>> {
    let mut out = Vec::with_capacity(a.len());
    for (&x, &y) in a.iter().zip(b.iter()) {
        out.push(x.checked_sub(y)?);
    }
    Some(out)
}

fn checked_lane_mul(a: &[i64], b: &[i64]) -> Option<Vec<i64>> {
    let mut out = Vec::with_capacity(a.len());
    for (&x, &y) in a.iter().zip(b.iter()) {
        out.push(x.checked_mul(y)?);
    }
    Some(out)
}

fn checked_scalar_add(a: &[i64], scalar: i64) -> Option<Vec<i64>> {
    let mut out = Vec::with_capacity(a.len());
    for &x in a.iter() {
        out.push(x.checked_add(scalar)?);
    }
    Some(out)
}

fn checked_scalar_sub(a: &[i64], scalar: i64) -> Option<Vec<i64>> {
    let mut out = Vec::with_capacity(a.len());
    for &x in a.iter() {
        out.push(x.checked_sub(scalar)?);
    }
    Some(out)
}

fn checked_scalar_rsub(a: &[i64], scalar: i64) -> Option<Vec<i64>> {
    let mut out = Vec::with_capacity(a.len());
    for &x in a.iter() {
        out.push(scalar.checked_sub(x)?);
    }
    Some(out)
}

fn checked_scalar_mul(a: &[i64], scalar: i64) -> Option<Vec<i64>> {
    let mut out = Vec::with_capacity(a.len());
    for &x in a.iter() {
        out.push(x.checked_mul(scalar)?);
    }
    Some(out)
}

fn checked_scalar_div(a: &[i64], scalar: i64) -> Option<Vec<i64>> {
    if scalar == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(a.len());
    for &x in a.iter() {
        if x % scalar != 0 {
            return None;
        }
        out.push(x.checked_div(scalar)?);
    }
    Some(out)
}

fn checked_scalar_rdiv(a: &[i64], scalar: i64) -> Option<Vec<i64>> {
    let mut out = Vec::with_capacity(a.len());
    for &x in a.iter() {
        if x == 0 || scalar % x != 0 {
            return None;
        }
        out.push(scalar.checked_div(x)?);
    }
    Some(out)
}

fn extract_integer_scalar(value: &Value) -> Option<i64> {
    match &value.data {
        ValueData::Scalar(f) if f.is_integer() => f.to_i64(),
        ValueData::Boolean(_)
        | ValueData::Scalar(_)
        | ValueData::ExactScalar(_)
        | ValueData::Vector(_)
        | ValueData::Tensor { .. }
        | ValueData::Record { .. }
        | ValueData::Nil
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => None,
    }
}

fn apply_simd_binary(
    word: &str,
    a: &Value,
    b: &Value,
    op: fn(i64, i64) -> Option<i64>,
    lane: fn(&[i64], &[i64]) -> Option<Vec<i64>>,
) -> Option<(Value, bool)> {
    let va: Cow<'_, [i64]> = extract_integer_lane(a)?;
    let vb: Cow<'_, [i64]> = extract_integer_lane(b)?;
    if va.len() != vb.len() {
        return None;
    }
    let (result, parallel) = crate::interpreter::parallel::elementwise_binary_checked(
        word,
        va.as_ref(),
        vb.as_ref(),
        op,
        lane,
    );
    // `None` => a lane overflowed `i64`; decline so the caller recomputes on
    // the exact path (Same Result). Otherwise emit the SoA tensor result.
    Some((create_value_from_integer_vector(result?), parallel))
}

// SIMD intrinsics path: only when wasm32 is built with the `simd128` target
// feature enabled. This is no longer the baseline default (see
// `.cargo/config.toml`); a future `build:wasm:simd` path can opt in with
// `-C target-feature=+simd128` to take this kernel. Without simd128 the scalar
// fallback below is used so the baseline wasm build always compiles.
//
// TODO(portability): expose this as an explicit optimized build target
// (e.g. npm `build:wasm:simd`) and/or runtime feature detection, rather than a
// global compile-time flag.
#[cfg(all(test, target_arch = "wasm32", target_feature = "simd128"))]
mod wasm_impl {
    use std::arch::wasm32::*;

    #[inline]
    pub fn simd_add(a: &[i64], b: &[i64]) -> Vec<i64> {
        debug_assert_eq!(a.len(), b.len());
        let len = a.len();
        let mut result = Vec::with_capacity(len);

        let chunks = len / 2;
        let remainder = len % 2;

        for i in 0..chunks {
            let idx = i * 2;
            let va = i64x2(a[idx], a[idx + 1]);
            let vb = i64x2(b[idx], b[idx + 1]);
            let vr = i64x2_add(va, vb);
            result.push(i64x2_extract_lane::<0>(vr));
            result.push(i64x2_extract_lane::<1>(vr));
        }

        if remainder > 0 {
            let idx = chunks * 2;
            result.push(a[idx] + b[idx]);
        }

        result
    }

    #[inline]
    pub fn simd_sub(a: &[i64], b: &[i64]) -> Vec<i64> {
        debug_assert_eq!(a.len(), b.len());
        let len = a.len();
        let mut result = Vec::with_capacity(len);

        let chunks = len / 2;
        let remainder = len % 2;

        for i in 0..chunks {
            let idx = i * 2;
            let va = i64x2(a[idx], a[idx + 1]);
            let vb = i64x2(b[idx], b[idx + 1]);
            let vr = i64x2_sub(va, vb);
            result.push(i64x2_extract_lane::<0>(vr));
            result.push(i64x2_extract_lane::<1>(vr));
        }

        if remainder > 0 {
            let idx = chunks * 2;
            result.push(a[idx] - b[idx]);
        }

        result
    }

    #[inline]
    pub fn simd_mul(a: &[i64], b: &[i64]) -> Vec<i64> {
        debug_assert_eq!(a.len(), b.len());
        let len = a.len();
        let mut result = Vec::with_capacity(len);

        let chunks = len / 2;
        let remainder = len % 2;

        for i in 0..chunks {
            let idx = i * 2;
            let va = i64x2(a[idx], a[idx + 1]);
            let vb = i64x2(b[idx], b[idx + 1]);
            let vr = i64x2_mul(va, vb);
            result.push(i64x2_extract_lane::<0>(vr));
            result.push(i64x2_extract_lane::<1>(vr));
        }

        if remainder > 0 {
            let idx = chunks * 2;
            result.push(a[idx] * b[idx]);
        }

        result
    }

    #[inline]
    pub fn simd_scalar_add(vec: &[i64], scalar: i64) -> Vec<i64> {
        let len = vec.len();
        let mut result = Vec::with_capacity(len);

        let vs = i64x2(scalar, scalar);
        let chunks = len / 2;
        let remainder = len % 2;

        for i in 0..chunks {
            let idx = i * 2;
            let va = i64x2(vec[idx], vec[idx + 1]);
            let vr = i64x2_add(va, vs);
            result.push(i64x2_extract_lane::<0>(vr));
            result.push(i64x2_extract_lane::<1>(vr));
        }

        if remainder > 0 {
            let idx = chunks * 2;
            result.push(vec[idx] + scalar);
        }

        result
    }

    #[inline]
    pub fn simd_scalar_mul(vec: &[i64], scalar: i64) -> Vec<i64> {
        let len = vec.len();
        let mut result = Vec::with_capacity(len);

        let vs = i64x2(scalar, scalar);
        let chunks = len / 2;
        let remainder = len % 2;

        for i in 0..chunks {
            let idx = i * 2;
            let va = i64x2(vec[idx], vec[idx + 1]);
            let vr = i64x2_mul(va, vs);
            result.push(i64x2_extract_lane::<0>(vr));
            result.push(i64x2_extract_lane::<1>(vr));
        }

        if remainder > 0 {
            let idx = chunks * 2;
            result.push(vec[idx] * scalar);
        }

        result
    }
}

// Scalar fallback: native builds, and any wasm build without `simd128`
// (now the baseline). Same observable result as the intrinsics path.
#[cfg(all(test, not(all(target_arch = "wasm32", target_feature = "simd128"))))]
mod wasm_impl {
    #[inline]
    pub fn simd_add(a: &[i64], b: &[i64]) -> Vec<i64> {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| x + y)
            .collect::<Vec<i64>>()
    }

    #[inline]
    pub fn simd_sub(a: &[i64], b: &[i64]) -> Vec<i64> {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| x - y)
            .collect::<Vec<i64>>()
    }

    #[inline]
    pub fn simd_mul(a: &[i64], b: &[i64]) -> Vec<i64> {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| x * y)
            .collect::<Vec<i64>>()
    }

    #[inline]
    pub fn simd_scalar_add(vec: &[i64], scalar: i64) -> Vec<i64> {
        vec.iter().map(|x| x + scalar).collect::<Vec<i64>>()
    }

    #[inline]
    pub fn simd_scalar_mul(vec: &[i64], scalar: i64) -> Vec<i64> {
        vec.iter().map(|x| x * scalar).collect::<Vec<i64>>()
    }
}

// Wrapping (non-overflow-checked) SIMD lane kernels. The production integer
// path is now overflow-checked (see `checked_lane_*` and the speculative
// lowering in `apply_simd_*`), so these wrapping kernels survive only as the
// element-wise reference the `interpreter::parallel` bit-identity proptests
// compare the multi-core kernel against. They are therefore test-only.
#[cfg(test)]
pub fn lane_add(a: &[i64], b: &[i64]) -> Vec<i64> {
    wasm_impl::simd_add(a, b)
}

#[cfg(test)]
pub fn lane_sub(a: &[i64], b: &[i64]) -> Vec<i64> {
    wasm_impl::simd_sub(a, b)
}

#[cfg(test)]
pub fn lane_mul(a: &[i64], b: &[i64]) -> Vec<i64> {
    wasm_impl::simd_mul(a, b)
}

#[cfg(test)]
pub fn lane_scalar_add(a: &[i64], scalar: i64) -> Vec<i64> {
    wasm_impl::simd_scalar_add(a, scalar)
}

#[cfg(test)]
pub fn lane_scalar_mul(a: &[i64], scalar: i64) -> Vec<i64> {
    wasm_impl::simd_scalar_mul(a, scalar)
}

/// Returns `(result, parallel_used)`; `parallel_used` is `true` only when the
/// multi-core kernel actually fired (observational metric only).
pub fn apply_simd_add(a: &Value, b: &Value) -> Option<(Value, bool)> {
    apply_simd_binary("+", a, b, |x, y| x.checked_add(y), checked_lane_add)
}

pub fn apply_simd_sub(a: &Value, b: &Value) -> Option<(Value, bool)> {
    apply_simd_binary("-", a, b, |x, y| x.checked_sub(y), checked_lane_sub)
}

pub fn apply_simd_mul(a: &Value, b: &Value) -> Option<(Value, bool)> {
    apply_simd_binary("*", a, b, |x, y| x.checked_mul(y), checked_lane_mul)
}

pub fn apply_simd_scalar_add(vec_val: &Value, scalar_val: &Value) -> Option<(Value, bool)> {
    let va: Cow<'_, [i64]> = extract_integer_lane(vec_val)?;
    let scalar: i64 = extract_integer_scalar(scalar_val)?;
    let (result, parallel) = crate::interpreter::parallel::elementwise_scalar_checked(
        "+",
        va.as_ref(),
        scalar,
        |x, s| x.checked_add(s),
        checked_scalar_add,
    );
    Some((create_value_from_integer_vector(result?), parallel))
}

pub fn apply_simd_scalar_sub(vec_val: &Value, scalar_val: &Value) -> Option<(Value, bool)> {
    let va: Cow<'_, [i64]> = extract_integer_lane(vec_val)?;
    let scalar: i64 = extract_integer_scalar(scalar_val)?;
    let (result, parallel) = crate::interpreter::parallel::elementwise_scalar_checked(
        "-",
        va.as_ref(),
        scalar,
        |x, s| x.checked_sub(s),
        checked_scalar_sub,
    );
    Some((create_value_from_integer_vector(result?), parallel))
}

pub fn apply_simd_scalar_rsub(scalar_val: &Value, vec_val: &Value) -> Option<(Value, bool)> {
    let va: Cow<'_, [i64]> = extract_integer_lane(vec_val)?;
    let scalar: i64 = extract_integer_scalar(scalar_val)?;
    let (result, parallel) = crate::interpreter::parallel::elementwise_scalar_checked(
        "-",
        va.as_ref(),
        scalar,
        |x, s| s.checked_sub(x),
        checked_scalar_rsub,
    );
    Some((create_value_from_integer_vector(result?), parallel))
}

pub fn apply_simd_scalar_mul(vec_val: &Value, scalar_val: &Value) -> Option<(Value, bool)> {
    let va: Cow<'_, [i64]> = extract_integer_lane(vec_val)?;
    let scalar: i64 = extract_integer_scalar(scalar_val)?;
    let (result, parallel) = crate::interpreter::parallel::elementwise_scalar_checked(
        "*",
        va.as_ref(),
        scalar,
        |x, s| x.checked_mul(s),
        checked_scalar_mul,
    );
    Some((create_value_from_integer_vector(result?), parallel))
}

pub fn apply_simd_scalar_div(vec_val: &Value, scalar_val: &Value) -> Option<(Value, bool)> {
    let va: Cow<'_, [i64]> = extract_integer_lane(vec_val)?;
    let scalar: i64 = extract_integer_scalar(scalar_val)?;
    if scalar == 0 {
        return None;
    }
    let (result, parallel) = crate::interpreter::parallel::elementwise_scalar_checked(
        "/",
        va.as_ref(),
        scalar,
        |x, s| (x % s == 0).then(|| x.checked_div(s)).flatten(),
        checked_scalar_div,
    );
    Some((create_value_from_integer_vector(result?), parallel))
}

pub fn apply_simd_scalar_rdiv(scalar_val: &Value, vec_val: &Value) -> Option<(Value, bool)> {
    let va: Cow<'_, [i64]> = extract_integer_lane(vec_val)?;
    let scalar: i64 = extract_integer_scalar(scalar_val)?;
    let (result, parallel) = crate::interpreter::parallel::elementwise_scalar_checked(
        "/",
        va.as_ref(),
        scalar,
        |x, s| (x != 0 && s % x == 0).then(|| s.checked_div(x)).flatten(),
        checked_scalar_rdiv,
    );
    Some((create_value_from_integer_vector(result?), parallel))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_int_vector(values: &[i64]) -> Value {
        let children: Vec<Value> = values
            .iter()
            .map(|&v| Value::from_int(v))
            .collect::<Vec<Value>>();
        Value::from_children(children)
    }

    #[test]
    fn test_extract_integer_vector() {
        let v: Value = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let result: Option<Vec<i64>> = extract_integer_vector(&v);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_extract_integer_vector_too_small() {
        let v: Value = create_int_vector(&[1, 2, 3]);
        let result: Option<Vec<i64>> = extract_integer_vector(&v);
        assert!(result.is_none());
    }

    #[test]
    fn test_simd_add_vectors() {
        let a: Value = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let b: Value = create_int_vector(&[10, 20, 30, 40, 50, 60, 70, 80]);
        let (result, _) = apply_simd_add(&a, &b).unwrap();
        let expected: Vec<i64> = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![11, 22, 33, 44, 55, 66, 77, 88]);
    }

    #[test]
    fn test_simd_sub_vectors() {
        let a: Value = create_int_vector(&[10, 20, 30, 40, 50, 60, 70, 80]);
        let b: Value = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let (result, _) = apply_simd_sub(&a, &b).unwrap();
        let expected: Vec<i64> = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![9, 18, 27, 36, 45, 54, 63, 72]);
    }

    #[test]
    fn test_simd_mul_vectors() {
        let a = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let b = create_int_vector(&[2, 3, 4, 5, 6, 7, 8, 9]);
        let (result, _) = apply_simd_mul(&a, &b).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![2, 6, 12, 20, 30, 42, 56, 72]);
    }

    #[test]
    fn test_simd_scalar_add() {
        let v = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let s = Value::from_int(100);
        let (result, _) = apply_simd_scalar_add(&v, &s).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![101, 102, 103, 104, 105, 106, 107, 108]);
    }

    #[test]
    fn test_simd_scalar_mul() {
        let v = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let s = Value::from_int(3);
        let (result, _) = apply_simd_scalar_mul(&v, &s).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![3, 6, 9, 12, 15, 18, 21, 24]);
    }

    #[test]
    fn test_simd_scalar_sub_both_orders() {
        let v = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let s = Value::from_int(10);

        let (result, _) = apply_simd_scalar_sub(&v, &s).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![-9, -8, -7, -6, -5, -4, -3, -2]);

        let (result, _) = apply_simd_scalar_rsub(&s, &v).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![9, 8, 7, 6, 5, 4, 3, 2]);
    }

    #[test]
    fn test_simd_scalar_div_both_orders_when_integral() {
        let v = create_int_vector(&[2, 4, 10, 20, -2, -4, -10, -20]);
        let s = Value::from_int(20);

        let (result, _) = apply_simd_scalar_div(&v, &Value::from_int(2)).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![1, 2, 5, 10, -1, -2, -5, -10]);

        let (result, _) = apply_simd_scalar_rdiv(&s, &v).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![10, 5, 2, 1, -10, -5, -2, -1]);
    }

    #[test]
    fn test_simd_scalar_div_declines_fractional_or_zero_results() {
        let v = create_int_vector(&[2, 3, 4, 5, 6, 7, 8, 9]);
        assert!(apply_simd_scalar_div(&v, &Value::from_int(2)).is_none());
        assert!(apply_simd_scalar_div(&v, &Value::from_int(0)).is_none());

        let with_zero = create_int_vector(&[1, 2, 0, 4, 5, 6, 7, 8]);
        assert!(apply_simd_scalar_rdiv(&Value::from_int(8), &with_zero).is_none());
    }

    #[test]
    fn test_simd_add_odd_length() {
        let a = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let b = create_int_vector(&[10, 20, 30, 40, 50, 60, 70, 80, 90]);
        let (result, _) = apply_simd_add(&a, &b).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![11, 22, 33, 44, 55, 66, 77, 88, 99]);
    }

    // ── 手1: zero-copy + SoA output ─────────────────────────────────────────

    #[test]
    fn extract_integer_lane_borrows_pure_integer_tensor() {
        // A 1-D pure-integer dense tensor must be borrowed (no allocation),
        // not re-materialized element by element.
        let tensor = Value::from_int_tensor(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        match extract_integer_lane(&tensor) {
            Some(Cow::Borrowed(slice)) => assert_eq!(slice, &[1, 2, 3, 4, 5, 6, 7, 8]),
            other => panic!("expected a borrowed lane, got {:?}", other),
        }
    }

    #[test]
    fn simd_result_is_soa_tensor_not_aos_vector() {
        // The output keeps the dense column (SoA) representation rather than
        // degrading to a boxed-Value vector.
        let a = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let b = create_int_vector(&[1, 1, 1, 1, 1, 1, 1, 1]);
        let (result, _) = apply_simd_add(&a, &b).unwrap();
        assert!(
            matches!(result.data, ValueData::Tensor { .. }),
            "integer SIMD result must be a dense Tensor, got {:?}",
            result.data
        );
    }

    #[test]
    fn simd_round_trips_tensor_inputs() {
        // Tensor → op → Tensor stays in the dense representation across a
        // chain, so a second op can borrow the first op's output directly.
        let a = Value::from_int_tensor(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let b = Value::from_int_tensor(vec![2, 2, 2, 2, 2, 2, 2, 2]);
        let (sum, _) = apply_simd_add(&a, &b).unwrap();
        assert!(matches!(extract_integer_lane(&sum), Some(Cow::Borrowed(_))));
        let (product, _) = apply_simd_mul(&sum, &b).unwrap();
        let got = extract_integer_vector(&product).unwrap();
        assert_eq!(got, vec![6, 8, 10, 12, 14, 16, 18, 20]);
    }

    // ── 奇策本命: speculative lowering declines on overflow ──────────────────

    #[test]
    fn simd_add_declines_on_overflow() {
        // i64::MAX + 1 overflows; the speculative lane must decline (return
        // None) so the caller recomputes on the exact path — never a silent
        // wrap. All lanes overflow here.
        let a = create_int_vector(&[i64::MAX; 8]);
        let b = create_int_vector(&[1; 8]);
        assert!(
            apply_simd_add(&a, &b).is_none(),
            "overflowing add must decline, not wrap"
        );
    }

    #[test]
    fn simd_mul_declines_on_overflow() {
        let a = create_int_vector(&[i64::MAX, 1, 1, 1, 1, 1, 1, 1]);
        let b = create_int_vector(&[2, 1, 1, 1, 1, 1, 1, 1]);
        assert!(
            apply_simd_mul(&a, &b).is_none(),
            "overflowing mul must decline, not wrap"
        );
    }

    #[test]
    fn simd_scalar_mul_declines_on_overflow() {
        let v = create_int_vector(&[i64::MAX, 0, 0, 0, 0, 0, 0, 0]);
        let s = Value::from_int(2);
        assert!(
            apply_simd_scalar_mul(&v, &s).is_none(),
            "overflowing scalar mul must decline, not wrap"
        );
    }

    #[test]
    fn simd_add_no_overflow_just_below_boundary() {
        // (i64::MAX - 1) + 1 == i64::MAX is representable; the fast path must
        // be taken and the value exact.
        let a = create_int_vector(&[i64::MAX - 1; 8]);
        let b = create_int_vector(&[1; 8]);
        let (result, _) = apply_simd_add(&a, &b).unwrap();
        let got = extract_integer_vector(&result).unwrap();
        assert_eq!(got, vec![i64::MAX; 8]);
    }
}
