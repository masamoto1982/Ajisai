use crate::types::{Value, ValueData};

const SIMD_THRESHOLD: usize = 8;

pub fn extract_integer_vector(val: &Value) -> Option<Vec<i64>> {
    let children: &Vec<Value> = match &val.data {
        ValueData::Vector(v) => v,
        ValueData::Tensor { data, shape } => {
            if shape.len() != 1 || data.len() < SIMD_THRESHOLD {
                return None;
            }
            let mut result: Vec<i64> = Vec::with_capacity(data.len());
            for f in data.iter() {
                if !f.is_integer() {
                    return None;
                }
                match f.to_i64() {
                    Some(n) => result.push(n),
                    None => return None,
                }
            }
            return Some(result);
        }
        ValueData::Boolean(_)
        | ValueData::Scalar(_)
        | ValueData::ExactScalar(_)
        | ValueData::Record { .. }
        | ValueData::Nil
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => return None,
    };

    if children.len() < SIMD_THRESHOLD {
        return None;
    }

    let mut result: Vec<i64> = Vec::with_capacity(children.len());
    for child in children.iter() {
        match &child.data {
            ValueData::Scalar(f) if f.is_integer() => match f.to_i64() {
                Some(n) => result.push(n),
                None => return None,
            },
            ValueData::Boolean(_)
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Vector(_)
            | ValueData::Tensor { .. }
            | ValueData::Record { .. }
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => return None,
        }
    }
    Some(result)
}

pub fn create_value_from_integer_vector(values: Vec<i64>) -> Value {
    let children: Vec<Value> = values.into_iter().map(Value::from_int).collect();
    Value::from_children(children)
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
    op: fn(i64, i64) -> i64,
    lane: fn(&[i64], &[i64]) -> Vec<i64>,
) -> Option<(Value, bool)> {
    let va: Vec<i64> = extract_integer_vector(a)?;
    let vb: Vec<i64> = extract_integer_vector(b)?;
    if va.len() != vb.len() {
        return None;
    }
    let (result, parallel) =
        crate::interpreter::parallel::elementwise_binary(word, &va, &vb, op, lane);
    Some((create_value_from_integer_vector(result), parallel))
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
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
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
#[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
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

// Public SIMD lane kernels: one chunk's worth of work. The native parallel
// dispatcher (`interpreter::parallel`) uses these as the sequential fallback
// (below threshold and on wasm), while the per-lane scalar `op` closures it
// fans across threads are required to agree with them element-wise so the
// parallel and sequential outputs stay bit-identical (Same Result).
pub fn lane_add(a: &[i64], b: &[i64]) -> Vec<i64> {
    wasm_impl::simd_add(a, b)
}

pub fn lane_sub(a: &[i64], b: &[i64]) -> Vec<i64> {
    wasm_impl::simd_sub(a, b)
}

pub fn lane_mul(a: &[i64], b: &[i64]) -> Vec<i64> {
    wasm_impl::simd_mul(a, b)
}

pub fn lane_scalar_add(a: &[i64], scalar: i64) -> Vec<i64> {
    wasm_impl::simd_scalar_add(a, scalar)
}

pub fn lane_scalar_mul(a: &[i64], scalar: i64) -> Vec<i64> {
    wasm_impl::simd_scalar_mul(a, scalar)
}

/// Returns `(result, parallel_used)`; `parallel_used` is `true` only when the
/// multi-core kernel actually fired (observational metric only).
pub fn apply_simd_add(a: &Value, b: &Value) -> Option<(Value, bool)> {
    apply_simd_binary("+", a, b, |x, y| x + y, lane_add)
}

pub fn apply_simd_sub(a: &Value, b: &Value) -> Option<(Value, bool)> {
    apply_simd_binary("-", a, b, |x, y| x - y, lane_sub)
}

pub fn apply_simd_mul(a: &Value, b: &Value) -> Option<(Value, bool)> {
    apply_simd_binary("*", a, b, |x, y| x * y, lane_mul)
}

pub fn apply_simd_scalar_add(vec_val: &Value, scalar_val: &Value) -> Option<(Value, bool)> {
    let va: Vec<i64> = extract_integer_vector(vec_val)?;
    let scalar: i64 = extract_integer_scalar(scalar_val)?;
    let (result, parallel) = crate::interpreter::parallel::elementwise_scalar(
        "+",
        &va,
        scalar,
        |x, s| x + s,
        lane_scalar_add,
    );
    Some((create_value_from_integer_vector(result), parallel))
}

pub fn apply_simd_scalar_mul(vec_val: &Value, scalar_val: &Value) -> Option<(Value, bool)> {
    let va: Vec<i64> = extract_integer_vector(vec_val)?;
    let scalar: i64 = extract_integer_scalar(scalar_val)?;
    let (result, parallel) = crate::interpreter::parallel::elementwise_scalar(
        "*",
        &va,
        scalar,
        |x, s| x * s,
        lane_scalar_mul,
    );
    Some((create_value_from_integer_vector(result), parallel))
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
    fn test_simd_add_odd_length() {
        let a = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let b = create_int_vector(&[10, 20, 30, 40, 50, 60, 70, 80, 90]);
        let (result, _) = apply_simd_add(&a, &b).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![11, 22, 33, 44, 55, 66, 77, 88, 99]);
    }
}
