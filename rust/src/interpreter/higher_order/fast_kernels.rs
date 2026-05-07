use crate::error::{AjisaiError, Result};
use crate::interpreter::quantized_block::QuantizedBlock;
use crate::interpreter::Interpreter;
use crate::types::fraction::Fraction;
use crate::types::{Token, Value};

#[derive(Clone)]
enum FastUnaryMapKernel {
    AddConst(Fraction),
    SubConst(Fraction),
    MulConst(Fraction),
    DivConst(Fraction),
    ModConst(Fraction),
    EqConst(Fraction),
    LtConst(Fraction),
    Abs,
    Neg,
    Not,
}

#[derive(Clone)]
enum FastUnaryPredicateKernel {
    EqConst(Fraction),
    LtConst(Fraction),
    Not,
}

#[derive(Clone, Copy)]
enum FastBinaryFoldKernel {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

fn parse_const_number_token(token: &Token) -> Option<Fraction> {
    if let Token::Number(n) = token {
        return Fraction::from_str(n).ok();
    }
    None
}

fn detect_fast_unary_map_kernel(tokens: &[Token]) -> Option<FastUnaryMapKernel> {
    if tokens.len() == 1 {
        if let Token::Symbol(sym) = &tokens[0] {
            return match Interpreter::normalize_symbol(sym).as_ref() {
                "ABS" => Some(FastUnaryMapKernel::Abs),
                "NEG" => Some(FastUnaryMapKernel::Neg),
                "NOT" => Some(FastUnaryMapKernel::Not),
                _ => None,
            };
        }
    }

    if tokens.len() != 4 {
        return None;
    }

    match (&tokens[0], &tokens[1], &tokens[2], &tokens[3]) {
        (Token::VectorStart, constant, Token::VectorEnd, Token::Symbol(op)) => {
            let c = parse_const_number_token(constant)?;
            match Interpreter::normalize_symbol(op).as_ref() {
                "+" => Some(FastUnaryMapKernel::AddConst(c)),
                "-" => Some(FastUnaryMapKernel::SubConst(c)),
                "*" => Some(FastUnaryMapKernel::MulConst(c)),
                "/" => Some(FastUnaryMapKernel::DivConst(c)),
                "MOD" => Some(FastUnaryMapKernel::ModConst(c)),
                "=" => Some(FastUnaryMapKernel::EqConst(c)),
                "<" => Some(FastUnaryMapKernel::LtConst(c)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn execute_fast_unary_map_kernel(
    kernel: &FastUnaryMapKernel,
    elem: Value,
) -> Option<Result<Value>> {
    let x = elem.as_scalar()?.clone();
    Some(match kernel {
        FastUnaryMapKernel::AddConst(c) => Ok(Value::from_number(x.add(c))),
        FastUnaryMapKernel::SubConst(c) => Ok(Value::from_number(x.sub(c))),
        FastUnaryMapKernel::MulConst(c) => Ok(Value::from_number(x.mul(c))),
        FastUnaryMapKernel::DivConst(c) => {
            if c.is_zero() {
                return Some(Err(AjisaiError::from("MAP fast kernel: division by zero")));
            }
            Ok(Value::from_number(x.div(c)))
        }
        FastUnaryMapKernel::ModConst(c) => {
            if c.is_zero() {
                return Some(Err(AjisaiError::from("MAP fast kernel: modulo by zero")));
            }
            Ok(Value::from_number(x.modulo(c)))
        }
        FastUnaryMapKernel::EqConst(c) => Ok(Value::from_bool(x == *c)),
        FastUnaryMapKernel::LtConst(c) => Ok(Value::from_bool(x.lt(c))),
        FastUnaryMapKernel::Abs => Ok(Value::from_number(x.abs())),
        FastUnaryMapKernel::Neg => Ok(Value::from_number(x.mul(&Fraction::from(-1_i64)))),
        FastUnaryMapKernel::Not => Ok(Value::from_bool(x.is_zero())),
    })
}

pub(super) fn try_execute_fast_quantized_map_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    elem: Value,
) -> Option<Result<Value>> {
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_unary_map_kernel(&line.source_tokens)?;
    interp.runtime_metrics.quantized_block_use_count += 1;
    execute_fast_unary_map_kernel(&kernel, elem)
}

/// Apply a unary fast kernel to every Fraction in `data`, producing a fresh
/// `Vec<Fraction>` of the same length. Returns `Err` for division/modulo by
/// zero. Caller is responsible for wrapping the output as a `Tensor` with
/// the appropriate shape.
fn apply_fast_unary_map_to_data(
    kernel: &FastUnaryMapKernel,
    data: &[Fraction],
) -> Result<Vec<Fraction>> {
    let mut out = Vec::with_capacity(data.len());
    match kernel {
        FastUnaryMapKernel::AddConst(c) => {
            for x in data {
                out.push(x.add(c));
            }
        }
        FastUnaryMapKernel::SubConst(c) => {
            for x in data {
                out.push(x.sub(c));
            }
        }
        FastUnaryMapKernel::MulConst(c) => {
            for x in data {
                out.push(x.mul(c));
            }
        }
        FastUnaryMapKernel::DivConst(c) => {
            if c.is_zero() {
                return Err(AjisaiError::from("MAP fast kernel: division by zero"));
            }
            for x in data {
                out.push(x.div(c));
            }
        }
        FastUnaryMapKernel::ModConst(c) => {
            if c.is_zero() {
                return Err(AjisaiError::from("MAP fast kernel: modulo by zero"));
            }
            for x in data {
                out.push(x.modulo(c));
            }
        }
        FastUnaryMapKernel::EqConst(c) => {
            for x in data {
                out.push(if x == c { Fraction::from(1_i64) } else { Fraction::from(0_i64) });
            }
        }
        FastUnaryMapKernel::LtConst(c) => {
            for x in data {
                out.push(if x.lt(c) { Fraction::from(1_i64) } else { Fraction::from(0_i64) });
            }
        }
        FastUnaryMapKernel::Abs => {
            for x in data {
                out.push(x.abs());
            }
        }
        FastUnaryMapKernel::Neg => {
            let neg_one = Fraction::from(-1_i64);
            for x in data {
                out.push(x.mul(&neg_one));
            }
        }
        FastUnaryMapKernel::Not => {
            for x in data {
                out.push(if x.is_zero() { Fraction::from(1_i64) } else { Fraction::from(0_i64) });
            }
        }
    }
    Ok(out)
}

fn apply_fast_unary_predicate_to_data(
    kernel: &FastUnaryPredicateKernel,
    data: &[Fraction],
) -> Vec<bool> {
    let mut out = Vec::with_capacity(data.len());
    match kernel {
        FastUnaryPredicateKernel::EqConst(c) => {
            for x in data {
                out.push(x == c);
            }
        }
        FastUnaryPredicateKernel::LtConst(c) => {
            for x in data {
                out.push(x.lt(c));
            }
        }
        FastUnaryPredicateKernel::Not => {
            for x in data {
                out.push(x.is_zero());
            }
        }
    }
    out
}

fn fold_fast_binary_over_data(
    kernel: FastBinaryFoldKernel,
    init: Fraction,
    data: &[Fraction],
) -> Result<Fraction> {
    let mut acc = init;
    match kernel {
        FastBinaryFoldKernel::Add => {
            for x in data {
                acc = acc.add(x);
            }
        }
        FastBinaryFoldKernel::Sub => {
            for x in data {
                acc = acc.sub(x);
            }
        }
        FastBinaryFoldKernel::Mul => {
            for x in data {
                acc = acc.mul(x);
            }
        }
        FastBinaryFoldKernel::Div => {
            for x in data {
                if x.is_zero() {
                    return Err(AjisaiError::from("FOLD fast kernel: division by zero"));
                }
                acc = acc.div(x);
            }
        }
        FastBinaryFoldKernel::Mod => {
            for x in data {
                if x.is_zero() {
                    return Err(AjisaiError::from("FOLD fast kernel: modulo by zero"));
                }
                acc = acc.modulo(x);
            }
        }
    }
    Ok(acc)
}

/// Result of an attempted bulk Tensor MAP. `None` means the input was not a
/// 1-D dense Tensor or the kernel is not fast-bulk eligible — caller falls
/// back to the per-element path.
pub(crate) struct BulkMapResult {
    pub data: Vec<Fraction>,
}

pub(super) fn try_bulk_quantized_map(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    target: &Value,
) -> Option<Result<BulkMapResult>> {
    let (data, shape) = target.as_dense_tensor()?;
    if shape.len() != 1 {
        return None;
    }
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_unary_map_kernel(&line.source_tokens)?;
    interp.runtime_metrics.quantized_block_use_count += data.len() as u64;
    interp.runtime_metrics.vtu_bulk_kernel_use_count =
        interp.runtime_metrics.vtu_bulk_kernel_use_count.saturating_add(1);
    Some(apply_fast_unary_map_to_data(&kernel, data).map(|d| BulkMapResult { data: d }))
}

pub(crate) struct BulkPredicateResult {
    pub flags: Vec<bool>,
}

pub(super) fn try_bulk_quantized_predicate(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    target: &Value,
) -> Option<BulkPredicateResult> {
    let (data, shape) = target.as_dense_tensor()?;
    if shape.len() != 1 {
        return None;
    }
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_unary_predicate_kernel(&line.source_tokens)?;
    interp.runtime_metrics.quantized_block_use_count += data.len() as u64;
    interp.runtime_metrics.vtu_bulk_kernel_use_count =
        interp.runtime_metrics.vtu_bulk_kernel_use_count.saturating_add(1);
    Some(BulkPredicateResult {
        flags: apply_fast_unary_predicate_to_data(&kernel, data),
    })
}

/// Extract the single Fraction inside a length-1 vector/tensor or a Scalar.
fn singleton_scalar(v: &Value) -> Option<Fraction> {
    if let Some(f) = v.as_scalar() {
        return Some(f.clone());
    }
    if v.len() == 1 {
        let child = v.child(0)?;
        return child.as_scalar().cloned();
    }
    None
}

pub(super) fn try_bulk_quantized_fold(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    init: &Value,
    target: &Value,
) -> Option<Result<Value>> {
    let init_scalar = singleton_scalar(init)?;
    let (data, shape) = target.as_dense_tensor()?;
    if shape.len() != 1 {
        return None;
    }
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_binary_fold_kernel(&line.source_tokens)?;
    interp.runtime_metrics.quantized_block_use_count += data.len() as u64;
    interp.runtime_metrics.vtu_bulk_kernel_use_count =
        interp.runtime_metrics.vtu_bulk_kernel_use_count.saturating_add(1);
    // Mirror the init shape: callers using `[ x ]` expect a Tensor[1] back,
    // while a bare Scalar accumulator should stay Scalar.
    let wrap_singleton = init.as_scalar().is_none();
    Some(fold_fast_binary_over_data(kernel, init_scalar, data).map(|f| {
        if wrap_singleton {
            Value::from_tensor(vec![f], vec![1])
        } else {
            Value::from_number(f)
        }
    }))
}

fn detect_fast_unary_predicate_kernel(tokens: &[Token]) -> Option<FastUnaryPredicateKernel> {
    if tokens.len() == 1 {
        if let Token::Symbol(sym) = &tokens[0] {
            if Interpreter::normalize_symbol(sym).as_ref() == "NOT" {
                return Some(FastUnaryPredicateKernel::Not);
            }
        }
        return None;
    }

    if tokens.len() != 4 {
        return None;
    }

    match (&tokens[0], &tokens[1], &tokens[2], &tokens[3]) {
        (Token::VectorStart, constant, Token::VectorEnd, Token::Symbol(op)) => {
            let c = parse_const_number_token(constant)?;
            match Interpreter::normalize_symbol(op).as_ref() {
                "=" => Some(FastUnaryPredicateKernel::EqConst(c)),
                "<" => Some(FastUnaryPredicateKernel::LtConst(c)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn execute_fast_unary_predicate_kernel(
    kernel: &FastUnaryPredicateKernel,
    elem: Value,
) -> Option<Result<bool>> {
    let x = elem.as_scalar()?.clone();
    Some(match kernel {
        FastUnaryPredicateKernel::EqConst(c) => Ok(x == *c),
        FastUnaryPredicateKernel::LtConst(c) => Ok(x.lt(c)),
        FastUnaryPredicateKernel::Not => Ok(x.is_zero()),
    })
}

pub(super) fn try_execute_fast_quantized_predicate_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    elem: Value,
) -> Option<Result<bool>> {
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_unary_predicate_kernel(&line.source_tokens)?;
    interp.runtime_metrics.quantized_block_use_count += 1;
    execute_fast_unary_predicate_kernel(&kernel, elem)
}

fn detect_fast_binary_fold_kernel(tokens: &[Token]) -> Option<FastBinaryFoldKernel> {
    if tokens.len() != 1 {
        return None;
    }
    let Token::Symbol(sym) = &tokens[0] else {
        return None;
    };
    match Interpreter::normalize_symbol(sym).as_ref() {
        "+" => Some(FastBinaryFoldKernel::Add),
        "-" => Some(FastBinaryFoldKernel::Sub),
        "*" => Some(FastBinaryFoldKernel::Mul),
        "/" => Some(FastBinaryFoldKernel::Div),
        "MOD" => Some(FastBinaryFoldKernel::Mod),
        _ => None,
    }
}

fn execute_fast_binary_fold_kernel(
    kernel: FastBinaryFoldKernel,
    acc: Value,
    elem: Value,
) -> Option<Result<Value>> {
    let a = acc.as_scalar()?.clone();
    let b = elem.as_scalar()?.clone();
    Some(match kernel {
        FastBinaryFoldKernel::Add => Ok(Value::from_number(a.add(&b))),
        FastBinaryFoldKernel::Sub => Ok(Value::from_number(a.sub(&b))),
        FastBinaryFoldKernel::Mul => Ok(Value::from_number(a.mul(&b))),
        FastBinaryFoldKernel::Div => {
            if b.is_zero() {
                return Some(Err(AjisaiError::from("FOLD fast kernel: division by zero")));
            }
            Ok(Value::from_number(a.div(&b)))
        }
        FastBinaryFoldKernel::Mod => {
            if b.is_zero() {
                return Some(Err(AjisaiError::from("FOLD fast kernel: modulo by zero")));
            }
            Ok(Value::from_number(a.modulo(&b)))
        }
    })
}

pub(super) fn try_execute_fast_quantized_fold_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    acc: Value,
    elem: Value,
) -> Option<Result<Value>> {
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_binary_fold_kernel(&line.source_tokens)?;
    interp.runtime_metrics.quantized_block_use_count += 1;
    execute_fast_binary_fold_kernel(kernel, acc, elem)
}
