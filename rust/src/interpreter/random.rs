






















use crate::error::{AjisaiError, Result};
use crate::interpreter::tensor_ops::FlatTensor;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::Value;
use num_bigint::{BigInt, Sign};
use num_traits::{One, ToPrimitive};


const DEFAULT_DENOMINATOR_BITS: u32 = 32;






fn compute_uniform_random(denominator: &BigInt) -> Result<BigInt> {
    if *denominator <= BigInt::one() {
        return Ok(BigInt::from(0));
    }

    let denom_bits = denominator.bits() as usize;
    let total_bits = denom_bits + 64;
    let bytes = (total_bits + 7) / 8;

    let mut buf = vec![0u8; bytes];
    getrandom::getrandom(&mut buf).map_err(|e| {
        AjisaiError::from(format!("CSPRNG: failed to generate random bytes: {}", e))
    })?;

    let random_value = BigInt::from_bytes_le(Sign::Plus, &buf);
    Ok(&random_value % denominator)
}


fn extract_positive_integer_from_value(val: &Value) -> Option<BigInt> {
    let tensor = FlatTensor::from_value(val).ok()?;
    if tensor.data.len() != 1 {
        return None;
    }
    let scalar = &tensor.data[0];
    if !scalar.is_integer() || scalar.is_zero() || scalar.numerator() <= BigInt::from(0) {
        return None;
    }
    Some(scalar.numerator())
}

fn parse_csprng_args_in_keep_mode(interp: &Interpreter) -> Result<(BigInt, usize)> {
    let default_denom = BigInt::from(1u64 << DEFAULT_DENOMINATOR_BITS);

    if interp.stack.is_empty() {
        return Ok((default_denom, 1));
    }

    let top = interp
        .stack
        .last()
        .ok_or_else(|| AjisaiError::from("CSPRNG requires stack value"))?;
    let Some(first_int) = extract_positive_integer_from_value(top) else {
        return Ok((default_denom, 1));
    };

    if interp.stack.len() >= 2 {
        if let Some(second_int) = extract_positive_integer_from_value(&interp.stack[interp.stack.len() - 2]) {
            let count = first_int
                .to_usize()
                .ok_or_else(|| AjisaiError::from("CSPRNG: count too large"))?;
            return Ok((second_int, count));
        }
    }

    let count = first_int
        .to_usize()
        .ok_or_else(|| AjisaiError::from("CSPRNG: count too large"))?;
    Ok((default_denom, count))
}


pub fn op_csprng(interp: &mut Interpreter) -> Result<()> {

    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "CSPRNG".into(),
            mode: "Stack".into(),
        });
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (denominator, count) = if is_keep_mode {
        parse_csprng_args_in_keep_mode(interp)?
    } else {
        parse_csprng_args(interp)?
    };

    if denominator <= BigInt::from(0) {
        return Err(AjisaiError::from("CSPRNG: denominator must be positive"));
    }

    let mut result_vec = Vec::with_capacity(count);
    for _ in 0..count {
        let numerator = compute_uniform_random(&denominator)?;
        let frac = Fraction::new(numerator, denominator.clone());
        result_vec.push(Value::from_number(frac));
    }

    interp.stack.push(Value::from_vector(result_vec));

    Ok(())
}


fn parse_csprng_args(interp: &mut Interpreter) -> Result<(BigInt, usize)> {
    let default_denom = BigInt::from(1u64 << DEFAULT_DENOMINATOR_BITS);


    if interp.stack.is_empty() {
        return Ok((default_denom, 1));
    }


    let top = interp
        .stack
        .last()
        .ok_or_else(|| AjisaiError::from("CSPRNG requires stack value"))?;


    let Some(first_int) = extract_positive_integer_from_value(top) else {
        return Ok((default_denom, 1));
    };


    interp.stack.pop();


    if let Some(second) = interp.stack.last() {
        if let Some(second_int) = extract_positive_integer_from_value(second) {

            interp.stack.pop();
            let count = first_int
                .to_usize()
                .ok_or_else(|| AjisaiError::from("CSPRNG: count too large"))?;
            return Ok((second_int, count));
        }
    }


    let count = first_int
        .to_usize()
        .ok_or_else(|| AjisaiError::from("CSPRNG: count too large"))?;

    Ok((default_denom, count))
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_csprng_rejects_stack_mode() {
        let mut interp = Interpreter::new();
        let result = interp.execute(".. CSPRNG").await;
        assert!(result.is_err(), "CSPRNG should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("CSPRNG") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for CSPRNG, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_csprng_generates_single_value() {
        let mut interp = Interpreter::new();
        let result = interp.execute("CSPRNG").await;
        assert!(result.is_ok(), "CSPRNG should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        let val = &interp.stack[0];
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 1);
        } else {
            panic!("Expected Vector");
        }
    }

    #[tokio::test]
    async fn test_csprng_generates_multiple_values() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 5 ] CSPRNG").await;
        assert!(
            result.is_ok(),
            "CSPRNG with count should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);

        let val = &interp.stack[0];
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 5);
        } else {
            panic!("Expected Vector");
        }
    }

    #[tokio::test]
    async fn test_csprng_with_denominator() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ 6 ] [ 3 ] CSPRNG").await;
        assert!(
            result.is_ok(),
            "CSPRNG with denominator should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);

        let val = &interp.stack[0];
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 3);
        } else {
            panic!("Expected Vector");
        }
    }

    #[tokio::test]
    async fn test_csprng_preserves_non_integer_on_stack() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ 1/2 ] CSPRNG").await;
        assert!(result.is_ok());

        assert_eq!(interp.stack.len(), 2);
    }

    #[tokio::test]
    async fn test_csprng_keep_mode_preserves_operand() {
        let mut interp = Interpreter::new();
        interp.execute("[ 5 ] ,, CSPRNG").await.unwrap();
        assert_eq!(interp.stack.len(), 2);
    }

    #[tokio::test]
    async fn test_csprng_scalar_args_supported() {
        let mut interp = Interpreter::new();
        let result = interp.execute("6 3 CSPRNG").await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_csprng_small_denominator_efficiency() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ 2 ] [ 50 ] CSPRNG").await;
        assert!(result.is_ok());

        let val = &interp.stack[0];
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 50);
        } else {
            panic!("Expected Vector");
        }
    }
}
