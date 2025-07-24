use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType};

pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            interp.stack.push(Value { val_type: ValueType::Number(n1.add(n2)) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            interp.stack.push(Value { val_type: ValueType::Number(n1.sub(n2)) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            interp.stack.push(Value { val_type: ValueType::Number(n1.mul(n2)) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    match &b.val_type {
        ValueType::Number(n) if n.numerator == 0 => return Err(AjisaiError::DivisionByZero),
        _ => {}
    }
    
    match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            interp.stack.push(Value { val_type: ValueType::Number(n1.div(n2)) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}

// 比較演算子
pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            interp.stack.push(Value { val_type: ValueType::Boolean(n1.gt(n2)) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}

pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            interp.stack.push(Value { val_type: ValueType::Boolean(n1.ge(n2)) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    interp.stack.push(Value { val_type: ValueType::Boolean(a == b) });
    Ok(())
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            interp.stack.push(Value { val_type: ValueType::Boolean(n1.lt(n2)) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            interp.stack.push(Value { val_type: ValueType::Boolean(n1.le(n2)) });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("number", "other type")),
    }
}

// 論理演算（三値論理）
pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::Boolean(b) => {
            interp.stack.push(Value { val_type: ValueType::Boolean(!b) });
            Ok(())
        },
        ValueType::Nil => {
            interp.stack.push(Value { val_type: ValueType::Nil });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("boolean or nil", "other type")),
    }
}

pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b_val = interp.stack.pop().unwrap();
    let a_val = interp.stack.pop().unwrap();
    
    let result = match (a_val.val_type, b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => {
            Value { val_type: ValueType::Boolean(a && b) }
        },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) => {
            Value { val_type: ValueType::Boolean(false) }
        },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) | (ValueType::Nil, ValueType::Nil) => {
            Value { val_type: ValueType::Nil }
        },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    
    interp.stack.push(result);
    Ok(())
}

pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b_val = interp.stack.pop().unwrap();
    let a_val = interp.stack.pop().unwrap();
    
    let result = match (a_val.val_type, b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => {
            Value { val_type: ValueType::Boolean(a || b) }
        },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) => {
            Value { val_type: ValueType::Boolean(true) }
        },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) | (ValueType::Nil, ValueType::Nil) => {
            Value { val_type: ValueType::Nil }
        },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    
    interp.stack.push(result);
    Ok(())
}

// Nil関連
pub fn op_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    interp.stack.push(Value { 
        val_type: ValueType::Boolean(matches!(val.val_type, ValueType::Nil)) 
    });
    Ok(())
}

pub fn op_not_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    interp.stack.push(Value { 
        val_type: ValueType::Boolean(!matches!(val.val_type, ValueType::Nil)) 
    });
    Ok(())
}

pub fn op_default(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let default_val = interp.stack.pop().unwrap();
    let val = interp.stack.pop().unwrap();
    
    if matches!(val.val_type, ValueType::Nil) {
        interp.stack.push(default_val);
    } else {
        interp.stack.push(val);
    }
    Ok(())
}
