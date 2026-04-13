

use crate::error::{AjisaiError, Result};
use crate::interpreter::tensor_ops::FlatTensor;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive, Zero};


const DEFAULT_HASH_BITS: u32 = 256;


const PRIME_BITS: u32 = 127;

lazy_static::lazy_static! {

    static ref PRIME1: BigInt = BigInt::parse_bytes(
        b"170141183460469231731687303715884105727", 10
    ).unwrap();


    static ref PRIME2: BigInt = BigInt::parse_bytes(
        b"170141183460469231731687303715884105655", 10
    ).unwrap();


    static ref PRIME3: BigInt = BigInt::parse_bytes(
        b"170141183460469231731687303715884104993", 10
    ).unwrap();


    static ref HASH_BASE: BigInt = BigInt::from(257u32);
}


fn serialize_value_for_hash(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    serialize_value_inner_for_hash(value, &mut bytes);
    bytes
}

fn serialize_value_inner_for_hash(val: &Value, bytes: &mut Vec<u8>) {

    if val.is_nil() {
        bytes.push(0x06);
        return;
    }


    if val.is_scalar() {
        if let Some(frac) = val.as_scalar() {

            let canonical = Fraction::new(frac.numerator(), frac.denominator());
            let (can_num, can_den) = canonical.to_bigint_pair();
            bytes.push(0x01);
            if can_num < BigInt::zero() {
                bytes.push(0x00);
            } else {
                bytes.push(0x01);
            }
            let num_bytes = if can_num < BigInt::zero() {
                (-&can_num).to_bytes_le().1
            } else {
                can_num.to_bytes_le().1
            };
            bytes.extend_from_slice(&(num_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&num_bytes);
            let den_bytes = can_den.to_bytes_le().1;
            bytes.extend_from_slice(&(den_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&den_bytes);
            return;
        }
    }


    if let ValueData::Vector(children) = &val.data {
        bytes.push(0x04);
        bytes.extend_from_slice(&(children.len() as u32).to_le_bytes());
        for elem in children.iter() {
            serialize_value_inner_for_hash(elem, bytes);
        }
    }
}


fn compute_polynomial_hash(bytes: &[u8], prime: &BigInt) -> BigInt {
    let mut hash = BigInt::zero();
    let mut power = BigInt::one();

    for &byte in bytes {

        hash = (&hash + &power * BigInt::from(byte)) % prime;

        power = (&power * &*HASH_BASE) % prime;
    }

    hash
}


fn compute_multi_prime_hash(bytes: &[u8], output_bits: u32) -> BigInt {
    let h1 = compute_polynomial_hash(bytes, &PRIME1);
    let h2 = compute_polynomial_hash(bytes, &PRIME2);
    let h3 = compute_polynomial_hash(bytes, &PRIME3);


    let combined = &h1 + (&h2 << PRIME_BITS as usize) + (&h3 << (2 * PRIME_BITS) as usize);


    let output_modulus = BigInt::one() << output_bits as usize;


    let mut result = combined.clone();


    let shift1 = output_bits / 3;
    let shift2 = output_bits * 2 / 3;
    result = &result ^ (&result >> shift1 as usize);
    result = &result ^ (&result >> shift2 as usize);


    result % output_modulus
}


fn extract_positive_integer_from_value(val: &Value) -> Option<u32> {
    let tensor = FlatTensor::from_value(val).ok()?;
    if tensor.data.len() != 1 {
        return None;
    }
    let scalar = &tensor.data[0];
    if !scalar.is_integer() || scalar.is_zero() || scalar.numerator() <= BigInt::from(0) {
        return None;
    }
    scalar.numerator().to_u32()
}

fn parse_hash_args_in_keep_mode(interp: &Interpreter) -> Result<(u32, Value)> {
    let target = interp
        .stack
        .last()
        .cloned()
        .ok_or_else(|| AjisaiError::from("HASH requires a value to hash"))?;

    if interp.stack.len() >= 2 {
        if let Some(bits) = extract_positive_integer_from_value(&interp.stack[interp.stack.len() - 2]) {
            return Ok((bits, target));
        }
    }

    Ok((DEFAULT_HASH_BITS, target))
}


pub fn op_hash(interp: &mut Interpreter) -> Result<()> {

    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "HASH".into(),
            mode: "Stack".into(),
        });
    }

    if interp.stack.is_empty() {
        return Err(AjisaiError::from("HASH requires a value to hash"));
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (output_bits, target_value) = if is_keep_mode {
        parse_hash_args_in_keep_mode(interp)?
    } else {
        parse_hash_args(interp)?
    };


    if output_bits < 32 || output_bits > 1024 {
        return Err(AjisaiError::from(
            "HASH: output bits must be between 32 and 1024",
        ));
    }


    let bytes = serialize_value_for_hash(&target_value);


    let hash_value = compute_multi_prime_hash(&bytes, output_bits);


    let denominator = BigInt::one() << output_bits as usize;
    let result_fraction = Fraction::new(hash_value, denominator);


    interp
        .stack
        .push(Value::from_vector(vec![Value::from_number(
            result_fraction,
        )]));

    Ok(())
}


fn parse_hash_args(interp: &mut Interpreter) -> Result<(u32, Value)> {
    let target = interp
        .stack
        .pop()
        .ok_or_else(|| AjisaiError::from("HASH requires a value to hash"))?;

    if let Some(bits_val) = interp.stack.last() {
        if let Some(bits) = extract_positive_integer_from_value(bits_val) {
            interp.stack.pop();
            return Ok((bits, target));
        }
    }

    Ok((DEFAULT_HASH_BITS, target))
}
