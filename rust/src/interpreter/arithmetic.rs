use crate::interpreter::{Interpreter, error::{LPLError, Result}};
use crate::types::{Value, ValueType, Fraction};

fn value_type_name(val_type: &ValueType) -> &'static str {
    match val_type {
        ValueType::Number(_) => "number",
        ValueType::String(_) => "string",
        ValueType::Boolean(_) => "boolean",
        ValueType::Symbol(_) => "symbol",
        ValueType::Vector(_) => "vector",
        ValueType::Nil => "nil",
    }
}

fn apply_unary_to_vector<F>(vec: &[Value], f: F) -> Vec<Value>
where
    F: Fn(&Value) -> Value,
{
    vec.iter().map(f).collect()
}

fn apply_binary_to_vectors<F>(v1: &[Value], v2: &[Value], f: F) -> Result<Vec<Value>>
where
    F: Fn(&Value, &Value) -> Result<Value>,
{
    if v1.len() != v2.len() {
        return Err(LPLError::VectorLengthMismatch {
            len1: v1.len(),
            len2: v2.len(),
        });
    }
    
    v1.iter().zip(v2.iter())
        .map(|(a, b)| f(a, b))
        .collect::<Result<Vec<Value>>>()
}

fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Fraction + Copy,
{
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let b = interp.bookshelf.pop().unwrap();
    let a = interp.bookshelf.pop().unwrap();
    
    let result = match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Number(op(n1, n2)) }
        },
        
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Number(op(elem_n, n)) }
                } else {
                    elem.clone()
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        (ValueType::Number(n), ValueType::Vector(v)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Number(op(n, elem_n)) }
                } else {
                    elem.clone()
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        (ValueType::Vector(v1), ValueType::Vector(v2)) => {
            let result = apply_binary_to_vectors(v1, v2, |a, b| {
                match (&a.val_type, &b.val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value { val_type: ValueType::Number(op(n1, n2)) })
                    },
                    _ => Ok(a.clone())
                }
            })?;
            Value { val_type: ValueType::Vector(result) }
        },
        
        _ => return Err(LPLError::type_error(
            "number or vector",
            &format!("{} and {}", value_type_name(&a.val_type), value_type_name(&b.val_type))
        )),
    };
    
    interp.bookshelf.push(result);
    Ok(())
}

fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool + Copy,
{
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let b = interp.bookshelf.pop().unwrap();
    let a = interp.bookshelf.pop().unwrap();
    
    let result = match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Boolean(op(n1, n2)) }
        },
        
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Boolean(op(elem_n, n)) }
                } else {
                    Value { val_type: ValueType::Boolean(false) }
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        (ValueType::Number(n), ValueType::Vector(v)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Boolean(op(n, elem_n)) }
                } else {
                    Value { val_type: ValueType::Boolean(false) }
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        (ValueType::Vector(v1), ValueType::Vector(v2)) => {
            let result = apply_binary_to_vectors(v1, v2, |a, b| {
                match (&a.val_type, &b.val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value { val_type: ValueType::Boolean(op(n1, n2)) })
                    },
                    _ => Ok(Value { val_type: ValueType::Boolean(false) })
                }
            })?;
            Value { val_type: ValueType::Vector(result) }
        },
        
        _ => return Err(LPLError::type_error(
            "number or vector",
            &format!("{} and {}", value_type_name(&a.val_type), value_type_name(&b.val_type))
        )),
    };
    
    interp.bookshelf.push(result);
    Ok(())
}

pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| a.add(b))
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| a.sub(b))
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| a.mul(b))
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let b = interp.bookshelf.pop().unwrap();
    let a = interp.bookshelf.pop().unwrap();
    
    match &b.val_type {
        ValueType::Number(n) if n.numerator == 0 => return Err(LPLError::DivisionByZero),
        ValueType::Vector(v) => {
            for elem in v {
                if let ValueType::Number(n) = &elem.val_type {
                    if n.numerator == 0 {
                        return Err(LPLError::DivisionByZero);
                    }
                }
            }
        },
        _ => {}
    }
    
    let result = match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Number(n1.div(n2)) }
        },
        
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Number(elem_n.div(n)) }
                } else {
                    elem.clone()
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        (ValueType::Number(n), ValueType::Vector(v)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Number(n.div(elem_n)) }
                } else {
                    elem.clone()
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        (ValueType::Vector(v1), ValueType::Vector(v2)) => {
            let result = apply_binary_to_vectors(v1, v2, |a, b| {
                match (&a.val_type, &b.val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value { val_type: ValueType::Number(n1.div(n2)) })
                    },
                    _ => Ok(a.clone())
                }
            })?;
            Value { val_type: ValueType::Vector(result) }
        },
        
        _ => return Err(LPLError::type_error(
            "number or vector",
            &format!("{} and {}", value_type_name(&a.val_type), value_type_name(&b.val_type))
        )),
    };
    
    interp.bookshelf.push(result);
    Ok(())
}

pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.gt(b))
}

pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.ge(b))
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.lt(b))
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.le(b))
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let b = interp.bookshelf.pop().unwrap();
    let a = interp.bookshelf.pop().unwrap();
    
    interp.bookshelf.push(Value { val_type: ValueType::Boolean(a == b) });
    Ok(())
}

pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    let result = match val.val_type {
        ValueType::Boolean(b) => Value { val_type: ValueType::Boolean(!b) },
        ValueType::Nil => Value { val_type: ValueType::Nil },
        ValueType::Vector(v) => {
            let result = apply_unary_to_vector(&v, |elem| {
                match &elem.val_type {
                    ValueType::Boolean(b) => Value { val_type: ValueType::Boolean(!b) },
                    ValueType::Nil => Value { val_type: ValueType::Nil },
                    _ => elem.clone(),
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        _ => return Err(LPLError::type_error(
            "boolean, nil, or vector",
            value_type_name(&val.val_type)
        )),
    };
    
    interp.bookshelf.push(result);
    Ok(())
}

pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let b_val = interp.bookshelf.pop().unwrap();
    let a_val = interp.bookshelf.pop().unwrap();
    
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
        _ => return Err(LPLError::type_error(
            "boolean or nil",
            "other types"
        )),
    };
    
    interp.bookshelf.push(result);
    Ok(())
}

pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let b_val = interp.bookshelf.pop().unwrap();
    let a_val = interp.bookshelf.pop().unwrap();
    
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
        _ => return Err(LPLError::type_error(
            "boolean or nil",
            "other types"
        )),
    };
    
    interp.bookshelf.push(result);
    Ok(())
}

pub fn op_some_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    interp.bookshelf.push(Value { 
        val_type: ValueType::Boolean(!matches!(val.val_type, ValueType::Nil)) 
    });
    Ok(())
}

pub fn op_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    interp.bookshelf.push(Value { 
        val_type: ValueType::Boolean(matches!(val.val_type, ValueType::Nil)) 
    });
    Ok(())
}

pub fn op_not_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    interp.bookshelf.push(Value { 
        val_type: ValueType::Boolean(!matches!(val.val_type, ValueType::Nil)) 
    });
    Ok(())
}

pub fn op_default(interp: &mut Interpreter) -> Result<()> {
    if interp.bookshelf.len() < 2 {
        return Err(LPLError::BookshelfUnderflow);
    }
    
    let default_val = interp.bookshelf.pop().unwrap();
    let val = interp.bookshelf.pop().unwrap();
    
    if matches!(val.val_type, ValueType::Nil) {
        interp.bookshelf.push(default_val);
    } else {
        interp.bookshelf.push(val);
    }
    Ok(())
}
