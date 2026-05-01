use super::common::ExecutableCode;
use super::runners::{
    execute_plain_fold_kernel, execute_plain_map_kernel, execute_plain_predicate_kernel,
    execute_quantized_fold_kernel, execute_quantized_map_kernel,
    execute_quantized_predicate_kernel,
};
use crate::elastic::{
    can_hedge_hof_kernel, validate_hedged_winner, ElasticMode, HedgedCandidateResult, HedgedPath,
};
use crate::error::Result;
use crate::interpreter::quantized_block::QuantizedBlock;
use crate::interpreter::Interpreter;
use crate::types::{Token, Value};

fn hedged_mode(mode: ElasticMode) -> bool {
    matches!(mode, ElasticMode::HedgedSafe | ElasticMode::HedgedTrace)
}

fn fast_guarded_mode(mode: ElasticMode) -> bool {
    matches!(mode, ElasticMode::FastGuarded)
}

fn is_quantized_block_guard_valid(interp: &Interpreter, qb: &QuantizedBlock) -> bool {
    let epoch_ok = qb.guard_signature.dictionary_epoch == interp.dictionary_epoch
        && qb.guard_signature.module_epoch == interp.module_epoch
        && qb.guard_signature.kernel_kind == qb.kernel_kind
        && qb.guard_signature.purity == qb.purity;
    if !epoch_ok {
        return false;
    }

    for dep in &qb.dependency_words {
        if interp.resolve_word_entry_readonly(dep).is_none() {
            return false;
        }
    }

    true
}

fn trace_hedged(interp: &Interpreter, msg: &str) {
    if interp.elastic_mode() == ElasticMode::HedgedTrace {
        eprintln!("[hedged] {}", msg);
    }
}

fn normalize_map_kernel_result(value: Value) -> Value {
    if value.len() == 1 {
        if let Some(child) = value.get_child(0) {
            if child.as_scalar().is_some() {
                return child.clone();
            }
        }
    }
    value
}

pub(crate) fn execute_hedged_map_kernel(
    interp: &mut Interpreter,
    op_name: &str,
    qb: &QuantizedBlock,
    plain_tokens: Option<&[Token]>,
    elem: Value,
) -> Result<Value> {
    let Some(tokens) = plain_tokens else {
        return execute_quantized_map_kernel(interp, qb, elem);
    };
    if fast_guarded_mode(interp.elastic_mode()) {
        if is_quantized_block_guard_valid(interp, qb) {
            return execute_quantized_map_kernel(interp, qb, elem);
        }
        interp.runtime_metrics.hedged_race_fallback_count += 1;
        interp.push_hedged_trace(format!(
            "fast-guarded:fallback op={} reason=guard-miss",
            op_name
        ));
        let plain_exec = ExecutableCode::CodeBlock(tokens.to_vec());
        return execute_plain_map_kernel(interp, &plain_exec, elem);
    }
    if !hedged_mode(interp.elastic_mode()) || !can_hedge_hof_kernel(op_name) {
        return execute_quantized_map_kernel(interp, qb, elem);
    }
    interp.runtime_metrics.hedged_race_started_count += 1;
    interp.push_hedged_trace(format!("hof-race:start op={}", op_name));
    let epoch_at_spawn = interp.current_epoch_snapshot();
    let quantized = execute_quantized_map_kernel(interp, qb, elem.clone());
    let plain_exec = ExecutableCode::CodeBlock(tokens.to_vec());
    let plain = execute_plain_map_kernel(interp, &plain_exec, elem);

    match (quantized, plain) {
        (Ok(q), Ok(p)) => {
            let q = normalize_map_kernel_result(q);
            let p = normalize_map_kernel_result(p);
            if q != p {
                interp.runtime_metrics.hedged_race_validation_reject_count += 1;
                interp.runtime_metrics.hedged_race_fallback_count += 1;
                interp
                    .push_hedged_trace(format!("hof-race:fallback op={} reason=mismatch", op_name));
                trace_hedged(
                    interp,
                    "map winner rejected: quantized/plain mismatch -> plain fallback",
                );
                return Ok(p);
            }
            let candidate = HedgedCandidateResult {
                path: HedgedPath::Quantized,
                stack: vec![q.clone()],
                epoch_at_spawn,
            };
            match validate_hedged_winner(&candidate, &interp.current_epoch_snapshot(), 1) {
                Ok(_) => {
                    interp.runtime_metrics.hedged_race_winner_quantized_count += 1;
                    interp.push_hedged_trace(format!(
                        "hof-race:winner op={} path=quantized",
                        op_name
                    ));
                    Ok(q)
                }
                Err(_) => {
                    interp.runtime_metrics.hedged_race_validation_reject_count += 1;
                    interp.runtime_metrics.hedged_race_fallback_count += 1;
                    interp.push_hedged_trace(format!(
                        "hof-race:fallback op={} reason=validation",
                        op_name
                    ));
                    Ok(p)
                }
            }
        }
        (Err(_), Ok(p)) => {
            let p = normalize_map_kernel_result(p);
            interp.runtime_metrics.hedged_race_winner_plain_count += 1;
            interp.runtime_metrics.hedged_race_fallback_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=plain", op_name));
            Ok(p)
        }
        (Ok(q), Err(_)) => {
            let q = normalize_map_kernel_result(q);
            interp.runtime_metrics.hedged_race_winner_quantized_count += 1;
            interp.push_hedged_trace(format!(
                "hof-race:winner op={} path=quantized reason=plain-error",
                op_name
            ));
            Ok(q)
        }
        (Err(eq), Err(_)) => Err(eq),
    }
}

pub(crate) fn execute_hedged_predicate_kernel(
    interp: &mut Interpreter,
    op_name: &str,
    qb: &QuantizedBlock,
    plain_tokens: Option<&[Token]>,
    elem: Value,
) -> Result<bool> {
    let Some(tokens) = plain_tokens else {
        return execute_quantized_predicate_kernel(interp, qb, elem);
    };
    if fast_guarded_mode(interp.elastic_mode()) {
        if is_quantized_block_guard_valid(interp, qb) {
            return execute_quantized_predicate_kernel(interp, qb, elem);
        }
        interp.runtime_metrics.hedged_race_fallback_count += 1;
        interp.push_hedged_trace(format!(
            "fast-guarded:fallback op={} reason=guard-miss",
            op_name
        ));
        let plain_exec = ExecutableCode::CodeBlock(tokens.to_vec());
        return execute_plain_predicate_kernel(interp, &plain_exec, elem);
    }
    if !hedged_mode(interp.elastic_mode()) || !can_hedge_hof_kernel(op_name) {
        return execute_quantized_predicate_kernel(interp, qb, elem);
    }
    interp.runtime_metrics.hedged_race_started_count += 1;
    interp.push_hedged_trace(format!("hof-race:start op={}", op_name));
    let quantized = execute_quantized_predicate_kernel(interp, qb, elem.clone());
    let plain_exec = ExecutableCode::CodeBlock(tokens.to_vec());
    let plain = execute_plain_predicate_kernel(interp, &plain_exec, elem);
    match (quantized, plain) {
        (Ok(q), Ok(p)) => {
            if q != p {
                interp.runtime_metrics.hedged_race_validation_reject_count += 1;
                interp.runtime_metrics.hedged_race_fallback_count += 1;
                interp
                    .push_hedged_trace(format!("hof-race:fallback op={} reason=mismatch", op_name));
                return Ok(p);
            }
            interp.runtime_metrics.hedged_race_winner_quantized_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=quantized", op_name));
            Ok(q)
        }
        (Err(_), Ok(p)) => {
            interp.runtime_metrics.hedged_race_winner_plain_count += 1;
            interp.runtime_metrics.hedged_race_fallback_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=plain", op_name));
            Ok(p)
        }
        (Ok(q), Err(_)) => {
            interp.runtime_metrics.hedged_race_winner_quantized_count += 1;
            interp.push_hedged_trace(format!(
                "hof-race:winner op={} path=quantized reason=plain-error",
                op_name
            ));
            Ok(q)
        }
        (Err(eq), Err(_)) => Err(eq),
    }
}

pub(crate) fn execute_hedged_fold_kernel(
    interp: &mut Interpreter,
    op_name: &str,
    qb: &QuantizedBlock,
    plain_tokens: Option<&[Token]>,
    acc: Value,
    elem: Value,
) -> Result<Value> {
    let Some(tokens) = plain_tokens else {
        return execute_quantized_fold_kernel(interp, qb, acc, elem);
    };
    if fast_guarded_mode(interp.elastic_mode()) {
        if is_quantized_block_guard_valid(interp, qb) {
            return execute_quantized_fold_kernel(interp, qb, acc, elem);
        }
        interp.runtime_metrics.hedged_race_fallback_count += 1;
        interp.push_hedged_trace(format!(
            "fast-guarded:fallback op={} reason=guard-miss",
            op_name
        ));
        let plain_exec = ExecutableCode::CodeBlock(tokens.to_vec());
        return execute_plain_fold_kernel(interp, &plain_exec, acc, elem);
    }
    if !hedged_mode(interp.elastic_mode()) || !can_hedge_hof_kernel(op_name) {
        return execute_quantized_fold_kernel(interp, qb, acc, elem);
    }
    interp.runtime_metrics.hedged_race_started_count += 1;
    interp.push_hedged_trace(format!("hof-race:start op={}", op_name));
    let quantized = execute_quantized_fold_kernel(interp, qb, acc.clone(), elem.clone());
    let plain_exec = ExecutableCode::CodeBlock(tokens.to_vec());
    let plain = execute_plain_fold_kernel(interp, &plain_exec, acc, elem);
    match (quantized, plain) {
        (Ok(q), Ok(p)) => {
            if q != p {
                interp.runtime_metrics.hedged_race_validation_reject_count += 1;
                interp.runtime_metrics.hedged_race_fallback_count += 1;
                interp
                    .push_hedged_trace(format!("hof-race:fallback op={} reason=mismatch", op_name));
                return Ok(p);
            }
            interp.runtime_metrics.hedged_race_winner_quantized_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=quantized", op_name));
            Ok(q)
        }
        (Err(_), Ok(p)) => {
            interp.runtime_metrics.hedged_race_winner_plain_count += 1;
            interp.runtime_metrics.hedged_race_fallback_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=plain", op_name));
            Ok(p)
        }
        (Ok(q), Err(_)) => {
            interp.runtime_metrics.hedged_race_winner_quantized_count += 1;
            interp.push_hedged_trace(format!(
                "hof-race:winner op={} path=quantized reason=plain-error",
                op_name
            ));
            Ok(q)
        }
        (Err(eq), Err(_)) => Err(eq),
    }
}
