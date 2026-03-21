// rust/src/interpreter/simd-vector-operations.rs
//
// WASM SIMD acceleration for integer vector arithmetic.
// Provides fast paths for element-wise operations on vectors of integer fractions (denominator=1).

use crate::types::{Value, ValueData};
use num_traits::{One, ToPrimitive};

/// Minimum vector length to use SIMD path (below this, scalar is faster due to branch overhead)
const SIMD_THRESHOLD: usize = 8;

/// Check if a Value is a flat vector of integer fractions (denominator=1, fits in i64).
/// Returns the extracted i64 values if so.
pub fn extract_integer_vector(val: &Value) -> Option<Vec<i64>> {
    let children = match &val.data {
        ValueData::Vector(v) => v,
        _ => return None,
    };

    if children.len() < SIMD_THRESHOLD {
        return None;
    }

    let mut result = Vec::with_capacity(children.len());
    for child in children.iter() {
        match &child.data {
            ValueData::Scalar(f) if f.denominator.is_one() => match f.numerator.to_i64() {
                Some(n) => result.push(n),
                None => return None,
            },
            _ => return None,
        }
    }
    Some(result)
}

/// Build a Value vector from i64 values (all as integer fractions).
pub fn create_value_from_integer_vector(values: Vec<i64>) -> Value {
    let children: Vec<Value> = values.into_iter().map(Value::from_int).collect();
    Value::from_children(children)
}

fn extract_integer_scalar(value: &Value) -> Option<i64> {
    match &value.data {
        ValueData::Scalar(f) if f.denominator.is_one() => f.numerator.to_i64(),
        _ => None,
    }
}

fn apply_simd_binary(a: &Value, b: &Value, op: fn(&[i64], &[i64]) -> Vec<i64>) -> Option<Value> {
    let va = extract_integer_vector(a)?;
    let vb = extract_integer_vector(b)?;
    if va.len() != vb.len() {
        return None;
    }
    Some(create_value_from_integer_vector(op(&va, &vb)))
}

// --- WASM SIMD implementations ---

#[cfg(target_arch = "wasm32")]
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

// --- Scalar fallback for non-WASM targets (tests, native builds) ---

#[cfg(not(target_arch = "wasm32"))]
mod wasm_impl {
    #[inline]
    pub fn simd_add(a: &[i64], b: &[i64]) -> Vec<i64> {
        a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
    }

    #[inline]
    pub fn simd_sub(a: &[i64], b: &[i64]) -> Vec<i64> {
        a.iter().zip(b.iter()).map(|(x, y)| x - y).collect()
    }

    #[inline]
    pub fn simd_mul(a: &[i64], b: &[i64]) -> Vec<i64> {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).collect()
    }

    #[inline]
    pub fn simd_scalar_add(vec: &[i64], scalar: i64) -> Vec<i64> {
        vec.iter().map(|x| x + scalar).collect()
    }

    #[inline]
    pub fn simd_scalar_mul(vec: &[i64], scalar: i64) -> Vec<i64> {
        vec.iter().map(|x| x * scalar).collect()
    }
}

/// Try to perform SIMD-accelerated addition on two values.
/// Returns Some(result) if both are integer vectors of sufficient length, None otherwise.
pub fn apply_simd_add(a: &Value, b: &Value) -> Option<Value> {
    apply_simd_binary(a, b, wasm_impl::simd_add)
}

/// Try to perform SIMD-accelerated subtraction on two values.
pub fn apply_simd_sub(a: &Value, b: &Value) -> Option<Value> {
    apply_simd_binary(a, b, wasm_impl::simd_sub)
}

/// Try to perform SIMD-accelerated multiplication on two values.
pub fn apply_simd_mul(a: &Value, b: &Value) -> Option<Value> {
    apply_simd_binary(a, b, wasm_impl::simd_mul)
}

/// Try to perform SIMD-accelerated scalar addition (vector + scalar or scalar + vector).
pub fn apply_simd_scalar_add(vec_val: &Value, scalar_val: &Value) -> Option<Value> {
    let va = extract_integer_vector(vec_val)?;
    let scalar = extract_integer_scalar(scalar_val)?;
    Some(create_value_from_integer_vector(wasm_impl::simd_scalar_add(
        &va, scalar,
    )))
}

/// Try to perform SIMD-accelerated scalar multiplication (vector * scalar or scalar * vector).
pub fn apply_simd_scalar_mul(vec_val: &Value, scalar_val: &Value) -> Option<Value> {
    let va = extract_integer_vector(vec_val)?;
    let scalar = extract_integer_scalar(scalar_val)?;
    Some(create_value_from_integer_vector(wasm_impl::simd_scalar_mul(
        &va, scalar,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_int_vector(values: &[i64]) -> Value {
        let children: Vec<Value> = values.iter().map(|&v| Value::from_int(v)).collect();
        Value::from_children(children)
    }

    #[test]
    fn test_extract_integer_vector() {
        let v = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let result = extract_integer_vector(&v);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_extract_integer_vector_too_small() {
        let v = create_int_vector(&[1, 2, 3]);
        let result = extract_integer_vector(&v);
        assert!(result.is_none());
    }

    #[test]
    fn test_simd_add_vectors() {
        let a = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let b = create_int_vector(&[10, 20, 30, 40, 50, 60, 70, 80]);
        let result = apply_simd_add(&a, &b).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![11, 22, 33, 44, 55, 66, 77, 88]);
    }

    #[test]
    fn test_simd_sub_vectors() {
        let a = create_int_vector(&[10, 20, 30, 40, 50, 60, 70, 80]);
        let b = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let result = apply_simd_sub(&a, &b).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![9, 18, 27, 36, 45, 54, 63, 72]);
    }

    #[test]
    fn test_simd_mul_vectors() {
        let a = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let b = create_int_vector(&[2, 3, 4, 5, 6, 7, 8, 9]);
        let result = apply_simd_mul(&a, &b).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![2, 6, 12, 20, 30, 42, 56, 72]);
    }

    #[test]
    fn test_simd_scalar_add() {
        let v = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let s = Value::from_int(100);
        let result = apply_simd_scalar_add(&v, &s).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![101, 102, 103, 104, 105, 106, 107, 108]);
    }

    #[test]
    fn test_simd_scalar_mul() {
        let v = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let s = Value::from_int(3);
        let result = apply_simd_scalar_mul(&v, &s).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![3, 6, 9, 12, 15, 18, 21, 24]);
    }

    #[test]
    fn test_simd_add_odd_length() {
        let a = create_int_vector(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let b = create_int_vector(&[10, 20, 30, 40, 50, 60, 70, 80, 90]);
        let result = apply_simd_add(&a, &b).unwrap();
        let expected = extract_integer_vector(&result).unwrap();
        assert_eq!(expected, vec![11, 22, 33, 44, 55, 66, 77, 88, 99]);
    }
}
