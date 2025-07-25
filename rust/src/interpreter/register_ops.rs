use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType};

pub fn op_r_add(interp: &mut Interpreter) -> Result<()> {
    let a = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    let r = interp.register.as_ref()
        .ok_or(AjisaiError::RegisterEmpty)?;
    
    let result = match (&a.val_type, &r.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Number(n1.add(n2)) }
        },
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result: Vec<Value> = v.iter()
                .map(|elem| {
                    if let ValueType::Number(elem_n) = &elem.val_type {
                        Value { val_type: ValueType::Number(elem_n.add(n)) }
                    } else {
                        elem.clone()
                    }
                })
                .collect();
            Value { val_type: ValueType::Vector(result) }
        },
        _ => return Err(AjisaiError::type_error("number or vector", "other type")),
    };
    
    interp.stack.push(result);
    Ok(())
}

pub fn op_r_sub(interp: &mut Interpreter) -> Result<()> {
    let a = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    let r = interp.register.as_ref()
        .ok_or(AjisaiError::RegisterEmpty)?;
    
    let result = match (&a.val_type, &r.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Number(n1.sub(n2)) }
        },
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result: Vec<Value> = v.iter()
                .map(|elem| {
                    if let ValueType::Number(elem_n) = &elem.val_type {
                        Value { val_type: ValueType::Number(elem_n.sub(n)) }
                    } else {
                        elem.clone()
                    }
                })
                .collect();
            Value { val_type: ValueType::Vector(result) }
        },
        _ => return Err(AjisaiError::type_error("number or vector", "other type")),
    };
    
    interp.stack.push(result);
    Ok(())
}

pub fn op_r_mul(interp: &mut Interpreter) -> Result<()> {
    let a = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    let r = interp.register.as_ref()
        .ok_or(AjisaiError::RegisterEmpty)?;
    
    let result = match (&a.val_type, &r.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Number(n1.mul(n2)) }
        },
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result: Vec<Value> = v.iter()
                .map(|elem| {
                    if let ValueType::Number(elem_n) = &elem.val_type {
                        Value { val_type: ValueType::Number(elem_n.mul(n)) }
                    } else {
                        elem.clone()
                    }
                })
                .collect();
            Value { val_type: ValueType::Vector(result) }
        },
        _ => return Err(AjisaiError::type_error("number or vector", "other type")),
    };
    
    interp.stack.push(result);
    Ok(())
}

pub fn op_r_div(interp: &mut Interpreter) -> Result<()> {
    let a = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    let r = interp.register.as_ref()
        .ok_or(AjisaiError::RegisterEmpty)?;
    
    // ゼロ除算チェック
    if let ValueType::Number(n) = &r.val_type {
        if n.numerator == 0 {
            return Err(AjisaiError::DivisionByZero);
        }
    }
    
    let result = match (&a.val_type, &r.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Number(n1.div(n2)) }
        },
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result: Vec<Value> = v.iter()
                .map(|elem| {
                    if let ValueType::Number(elem_n) = &elem.val_type {
                        Value { val_type: ValueType::Number(elem_n.div(n)) }
                    } else {
                        elem.clone()
                    }
                })
                .collect();
            Value { val_type: ValueType::Vector(result) }
        },
        _ => return Err(AjisaiError::type_error("number or vector", "other type")),
    };
    
    interp.stack.push(result);
    Ok(())
}
