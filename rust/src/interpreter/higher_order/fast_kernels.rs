//! Specialized kernels for eligible quantized HOF blocks. Routing only: a
//! kernel must produce exactly what the generic quantized-block route would
//! produce, and it **declines** (returns `None`, falling back to the generic
//! route) any input whose outcome the generic route defines through the error
//! model — e.g. division or modulo by zero, which the Bubble Rule turns into
//! a NIL bubble, never a kernel-specific error. Disable the whole family via
//! `AJISAI_NO_FAST_KERNEL` / `set_fast_kernel_enabled` for A/B comparison.

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

// Kernel detection matches tokens textually, without dictionary resolution,
// so it may only recognize words whose meaning can never change: protected
// core builtins (`+ - * / MOD = < NOT`). Module words such as `ABS`/`NEG`
// must not appear here — their availability depends on IMPORT state, and a
// textual match would give them meaning on the kernel route that the generic
// route does not have.
fn detect_fast_unary_map_kernel(tokens: &[Token]) -> Option<FastUnaryMapKernel> {
    if tokens.len() == 1 {
        if let Token::Symbol(sym) = &tokens[0] {
            if Interpreter::normalize_symbol(sym).as_ref() == "NOT" {
                return Some(FastUnaryMapKernel::Not);
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
                "+" => Some(FastUnaryMapKernel::AddConst(c)),
                "-" => Some(FastUnaryMapKernel::SubConst(c)),
                "*" => Some(FastUnaryMapKernel::MulConst(c)),
                // A zero divisor/modulus is the generic route's case: the
                // Bubble Rule turns it into a NIL bubble per element, so the
                // kernel declines instead of inventing its own outcome.
                "/" if !c.is_zero() => Some(FastUnaryMapKernel::DivConst(c)),
                "MOD" if !c.is_zero() => Some(FastUnaryMapKernel::ModConst(c)),
                "=" => Some(FastUnaryMapKernel::EqConst(c)),
                "<" => Some(FastUnaryMapKernel::LtConst(c)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn execute_fast_unary_map_kernel(kernel: &FastUnaryMapKernel, elem: Value) -> Option<Value> {
    let x = elem.as_scalar()?.clone();
    Some(match kernel {
        FastUnaryMapKernel::AddConst(c) => Value::from_number(x.add(c)),
        FastUnaryMapKernel::SubConst(c) => Value::from_number(x.sub(c)),
        FastUnaryMapKernel::MulConst(c) => Value::from_number(x.mul(c)),
        FastUnaryMapKernel::DivConst(c) => Value::from_number(x.div(c)),
        FastUnaryMapKernel::ModConst(c) => Value::from_number(x.modulo(c)),
        FastUnaryMapKernel::EqConst(c) => Value::from_bool(x == *c),
        FastUnaryMapKernel::LtConst(c) => Value::from_bool(x.lt(c)),
        FastUnaryMapKernel::Not => Value::from_bool(x.is_zero()),
    })
}

pub(super) fn try_execute_fast_quantized_map_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    elem: Value,
) -> Option<Value> {
    if !interp.fast_kernel_enabled {
        return None;
    }
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_unary_map_kernel(&line.source_tokens)?;
    let result = execute_fast_unary_map_kernel(&kernel, elem)?;
    interp.runtime_metrics.quantized_block_use_count += 1;
    Some(result)
}

/// Apply a unary fast kernel to every Fraction in `data`, producing a fresh
/// `Vec<Fraction>` of the same length. Zero divisors/moduli never reach this
/// point — `detect_fast_unary_map_kernel` declines them. Caller is
/// responsible for wrapping the output as a `Tensor` with the appropriate
/// shape.
fn apply_fast_unary_map_to_data(kernel: &FastUnaryMapKernel, data: &[Fraction]) -> Vec<Fraction> {
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
            for x in data {
                out.push(x.div(c));
            }
        }
        FastUnaryMapKernel::ModConst(c) => {
            for x in data {
                out.push(x.modulo(c));
            }
        }
        FastUnaryMapKernel::EqConst(c) => {
            for x in data {
                out.push(if x == c {
                    Fraction::from(1_i64)
                } else {
                    Fraction::from(0_i64)
                });
            }
        }
        FastUnaryMapKernel::LtConst(c) => {
            for x in data {
                out.push(if x.lt(c) {
                    Fraction::from(1_i64)
                } else {
                    Fraction::from(0_i64)
                });
            }
        }
        FastUnaryMapKernel::Not => {
            for x in data {
                out.push(if x.is_zero() {
                    Fraction::from(1_i64)
                } else {
                    Fraction::from(0_i64)
                });
            }
        }
    }
    out
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

/// Fold a fast binary kernel over `data`. Zero divisors/moduli never reach
/// this point — `try_bulk_quantized_fold` declines when the data contains
/// one, so the generic route's Bubble Rule defines that outcome.
fn fold_fast_binary_over_data(
    kernel: FastBinaryFoldKernel,
    init: Fraction,
    data: &[Fraction],
) -> Fraction {
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
                acc = acc.div(x);
            }
        }
        FastBinaryFoldKernel::Mod => {
            for x in data {
                acc = acc.modulo(x);
            }
        }
    }
    acc
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
) -> Option<BulkMapResult> {
    if !interp.fast_kernel_enabled {
        return None;
    }
    let (data, shape) = target.as_dense_tensor()?;
    if shape.len() != 1 {
        return None;
    }
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_unary_map_kernel(&line.source_tokens)?;
    interp.runtime_metrics.quantized_block_use_count += data.len() as u64;
    interp.runtime_metrics.vtu_bulk_kernel_use_count = interp
        .runtime_metrics
        .vtu_bulk_kernel_use_count
        .saturating_add(1);
    Some(BulkMapResult {
        data: apply_fast_unary_map_to_data(&kernel, &data.to_fractions()),
    })
}

pub(crate) struct BulkPredicateResult {
    pub flags: Vec<bool>,
}

pub(super) fn try_bulk_quantized_predicate(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    target: &Value,
) -> Option<BulkPredicateResult> {
    if !interp.fast_kernel_enabled {
        return None;
    }
    let (data, shape) = target.as_dense_tensor()?;
    if shape.len() != 1 {
        return None;
    }
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_unary_predicate_kernel(&line.source_tokens)?;
    interp.runtime_metrics.quantized_block_use_count += data.len() as u64;
    interp.runtime_metrics.vtu_bulk_kernel_use_count = interp
        .runtime_metrics
        .vtu_bulk_kernel_use_count
        .saturating_add(1);
    Some(BulkPredicateResult {
        flags: apply_fast_unary_predicate_to_data(&kernel, &data.to_fractions()),
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
) -> Option<Value> {
    if !interp.fast_kernel_enabled {
        return None;
    }
    let init_scalar = singleton_scalar(init)?;
    let (data, shape) = target.as_dense_tensor()?;
    if shape.len() != 1 {
        return None;
    }
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_binary_fold_kernel(&line.source_tokens)?;
    let fractions = data.to_fractions();
    // A zero divisor/modulus anywhere in the data is the generic route's
    // case (Bubble Rule → NIL bubble), so the kernel declines up front.
    if matches!(
        kernel,
        FastBinaryFoldKernel::Div | FastBinaryFoldKernel::Mod
    ) && fractions.iter().any(|x| x.is_zero())
    {
        return None;
    }
    interp.runtime_metrics.quantized_block_use_count += data.len() as u64;
    interp.runtime_metrics.vtu_bulk_kernel_use_count = interp
        .runtime_metrics
        .vtu_bulk_kernel_use_count
        .saturating_add(1);
    // Mirror the init shape: callers using `[ x ]` expect a Tensor[1] back,
    // while a bare Scalar accumulator should stay Scalar.
    let f = fold_fast_binary_over_data(kernel, init_scalar, &fractions);
    Some(if init.as_scalar().is_none() {
        Value::from_tensor(vec![f], vec![1])
    } else {
        Value::from_number(f)
    })
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
) -> Option<bool> {
    let x = elem.as_scalar()?.clone();
    Some(match kernel {
        FastUnaryPredicateKernel::EqConst(c) => x == *c,
        FastUnaryPredicateKernel::LtConst(c) => x.lt(c),
        FastUnaryPredicateKernel::Not => x.is_zero(),
    })
}

pub(super) fn try_execute_fast_quantized_predicate_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    elem: Value,
) -> Option<bool> {
    if !interp.fast_kernel_enabled {
        return None;
    }
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_unary_predicate_kernel(&line.source_tokens)?;
    let result = execute_fast_unary_predicate_kernel(&kernel, elem)?;
    interp.runtime_metrics.quantized_block_use_count += 1;
    Some(result)
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
) -> Option<Value> {
    let a = acc.as_scalar()?.clone();
    let b = elem.as_scalar()?.clone();
    Some(match kernel {
        FastBinaryFoldKernel::Add => Value::from_number(a.add(&b)),
        FastBinaryFoldKernel::Sub => Value::from_number(a.sub(&b)),
        FastBinaryFoldKernel::Mul => Value::from_number(a.mul(&b)),
        // Division/modulo by zero is the generic route's case (Bubble Rule →
        // NIL bubble), so the kernel declines and the block runs generically.
        FastBinaryFoldKernel::Div => {
            if b.is_zero() {
                return None;
            }
            Value::from_number(a.div(&b))
        }
        FastBinaryFoldKernel::Mod => {
            if b.is_zero() {
                return None;
            }
            Value::from_number(a.modulo(&b))
        }
    })
}

pub(super) fn try_execute_fast_quantized_fold_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    acc: Value,
    elem: Value,
) -> Option<Value> {
    if !interp.fast_kernel_enabled {
        return None;
    }
    let line = qb.compiled_plan.lines.first()?;
    let kernel = detect_fast_binary_fold_kernel(&line.source_tokens)?;
    let result = execute_fast_binary_fold_kernel(kernel, acc, elem)?;
    interp.runtime_metrics.quantized_block_use_count += 1;
    Some(result)
}
