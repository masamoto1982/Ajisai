//! Test suite for `crate::interpreter::quantized_block`.

use crate::interpreter::quantized_block::{
    is_quantizable_block, quantize_code_block, KernelKind, QuantizedArity, QuantizedPurity,
    VtuBackendCandidate, VtuSuitability,
};
use crate::interpreter::tensor_ops::{
    apply_binary_broadcast_with_metrics, apply_unary_flat_with_metrics,
};
use crate::interpreter::{Interpreter, RuntimeMetrics};
use crate::types::fraction::Fraction;
use crate::types::{Token, Value};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_interp() -> Interpreter {
    Interpreter::new()
}

fn num(s: &str) -> Token {
    Token::Number(s.into())
}

fn sym(s: &str) -> Token {
    Token::Symbol(s.into())
}

fn run_code(code: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        interp.execute(code).await.expect("code should execute");
    });
    interp
}

fn run_code_result(code: &str) -> std::result::Result<Interpreter, crate::error::AjisaiError> {
    let mut interp = Interpreter::new();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result = rt.block_on(async { interp.execute(code).await });
    match result {
        Ok(()) => Ok(interp),
        Err(e) => Err(e),
    }
}

fn stack_top(interp: &Interpreter) -> &Value {
    interp
        .get_stack()
        .last()
        .expect("stack should not be empty")
}

/// Extract i64 from top of stack, handling both raw scalars and
/// single-element vectors (which Ajisai often produces for numeric results).
fn stack_top_i64(interp: &Interpreter) -> i64 {
    let top = stack_top(interp);
    if let Some(f) = top.as_scalar() {
        return f.to_i64().expect("scalar should be representable as i64");
    }
    if top.len() == 1 {
        if let Some(child) = top.child(0) {
            if let Some(f) = child.as_scalar() {
                return f
                    .to_i64()
                    .expect("inner scalar should be representable as i64");
            }
        }
    }
    panic!(
        "stack top should be integer scalar or single-element vector, got len={}",
        top.len()
    );
}

/// Extract bool from top of stack, handling both scalars and single-element vectors.
fn stack_top_bool(interp: &Interpreter) -> bool {
    let top = stack_top(interp);
    if let Some(b) = top.as_truth() {
        return b;
    }
    if let Some(f) = top.as_scalar() {
        return !f.is_zero();
    }
    if top.len() == 1 {
        if let Some(child) = top.child(0) {
            if let Some(b) = child.as_truth() {
                return b;
            }
            if let Some(f) = child.as_scalar() {
                return !f.is_zero();
            }
        }
    }
    panic!("stack top should be boolean scalar or single-element vector");
}

// ---------------------------------------------------------------------------
// is_quantizable_block
// ---------------------------------------------------------------------------

#[test]
fn quantizes_simple_block() {
    let mut interp = make_interp();
    let tokens = vec![num("1"), sym("+")];
    assert!(is_quantizable_block(&tokens));
    assert!(quantize_code_block(&tokens, &mut interp).is_some());
}

#[test]
fn empty_block_is_not_quantizable() {
    assert!(!is_quantizable_block(&[]));
}

#[test]
fn linebreak_makes_block_non_quantizable() {
    let tokens = vec![num("1"), Token::LineBreak, sym("+")];
    assert!(!is_quantizable_block(&tokens));
}

// ---------------------------------------------------------------------------
// Arity inference — pure-builtin blocks only
// ---------------------------------------------------------------------------

/// `{ + }` consumes 2 from external stack, produces 1.
#[test]
fn arity_binary_add() {
    let mut interp = make_interp();
    let tokens = vec![sym("+")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.input_arity, QuantizedArity::Fixed(2));
    assert_eq!(qb.output_arity, QuantizedArity::Fixed(1));
}

/// `{ - }` — binary: 2 in, 1 out.
#[test]
fn arity_binary_sub() {
    let mut interp = make_interp();
    let tokens = vec![sym("-")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.input_arity, QuantizedArity::Fixed(2));
    assert_eq!(qb.output_arity, QuantizedArity::Fixed(1));
}

/// `{ < }` — binary comparison: 2 in, 1 out.
#[test]
fn arity_binary_compare() {
    let mut interp = make_interp();
    let tokens = vec![sym("<")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.input_arity, QuantizedArity::Fixed(2));
    assert_eq!(qb.output_arity, QuantizedArity::Fixed(1));
}

/// `{ NOT }` — unary: 1 in, 1 out.
#[test]
fn arity_unary_not() {
    let mut interp = make_interp();
    let tokens = vec![sym("NOT")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.input_arity, QuantizedArity::Fixed(1));
    assert_eq!(qb.output_arity, QuantizedArity::Fixed(1));
}

/// Words not in BUILTIN_SPECS compile to FallbackToken → arity is Variable.
/// DROP/DUP/SWAP are not Ajisai builtins, so arity inference gives Variable.
#[test]
fn arity_non_builtin_word_is_variable() {
    let mut interp = make_interp();
    // DROP is not an Ajisai builtin and not present in BUILTIN_SPECS;
    // it is therefore arity-Variable on the analyzer side.
    let tokens = vec![sym("DROP")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    // DROP is not in BUILTIN_SPECS → FallbackToken → Variable arity
    assert_eq!(qb.input_arity, QuantizedArity::Variable);
    assert_eq!(qb.output_arity, QuantizedArity::Variable);
}

/// `{ NOT < }` — two builtins: NOT is 1→1, then < is 2→1.
/// Combined arity is determined by simulation: start depth=0,
/// NOT needs 1 → min=-1, depth=0; then < needs 2 → depth=-2 (min=-2), depth=-1.
/// input_arity = 2, output_arity = 1.
#[test]
fn arity_chain_not_then_compare() {
    let mut interp = make_interp();
    let tokens = vec![sym("NOT"), sym("<")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    // NOT: 1→1, then <: 2→1.  Net: 2 from external, 1 result.
    assert_eq!(qb.input_arity, QuantizedArity::Fixed(2));
    assert_eq!(qb.output_arity, QuantizedArity::Fixed(1));
}

/// A block calling an unknown builtin (e.g. MAP) should fall back to Variable.
#[test]
fn arity_unknown_builtin_falls_back_to_variable() {
    let mut interp = make_interp();
    let tokens = vec![sym("MAP")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.input_arity, QuantizedArity::Variable);
    assert_eq!(qb.output_arity, QuantizedArity::Variable);
}

/// A block calling a user-defined word should have Variable arity.
#[test]
fn arity_user_word_is_variable() {
    let mut interp = make_interp();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        interp.execute("{ + } 'SUMX' DEF").await.unwrap();
    });
    let tokens = vec![sym("SUMX")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.input_arity, QuantizedArity::Variable);
    assert_eq!(qb.output_arity, QuantizedArity::Variable);
}

/// A block with literal push (Number token) contributes +1 to depth.
/// `{ 1 + }` — push literal 1 (depth +1), then + (needs 2, gets 1 from literal and
/// 1 from external, depth -1 after consuming + producing 1 → net 0).
/// input_arity = 1, output_arity = 1.
#[test]
fn arity_literal_then_add() {
    let mut interp = make_interp();
    // Simulate { 1 + } by constructing tokens directly
    let tokens = vec![num("1"), sym("+")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.input_arity, QuantizedArity::Fixed(1));
    assert_eq!(qb.output_arity, QuantizedArity::Fixed(1));
}

// ---------------------------------------------------------------------------
// Purity inference
// ---------------------------------------------------------------------------

/// Pure arithmetic block should be marked Pure.
#[test]
fn purity_arithmetic_is_pure() {
    let mut interp = make_interp();
    let tokens = vec![sym("+")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.purity, QuantizedPurity::Pure);
}

/// `can_fuse` and `can_short_circuit` should reflect purity.
#[test]
fn can_fuse_reflects_purity() {
    let mut interp = make_interp();
    let tokens = vec![sym("+")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert!(qb.can_fuse);
    assert!(qb.can_short_circuit);
}

#[test]
fn impure_builtin_is_rejected_at_gate() {
    // Phase 1-C: explicit impure builtins are rejected by `is_quantizable_block`
    // before quantization, so `quantize_code_block` returns None.
    let mut interp = make_interp();
    let tokens = vec![sym("PRINT")];
    assert!(!is_quantizable_block(&tokens));
    assert!(quantize_code_block(&tokens, &mut interp).is_none());
}

#[test]
fn kernel_kind_detects_map_unary_const_plus() {
    let mut interp = make_interp();
    let tokens = vec![Token::VectorStart, num("2"), Token::VectorEnd, sym("+")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.kernel_kind, KernelKind::MapUnaryPure);
    assert!(qb.fast_path_id.is_some());
    assert!(qb.eligible_for_cache);
    assert!(qb.eligible_for_fusion);
}

#[test]
fn kernel_kind_detects_predicate_not() {
    let mut interp = make_interp();
    let tokens = vec![sym("NOT")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.kernel_kind, KernelKind::PredicateUnaryPure);
    assert!(qb.fast_path_id.is_some());
}

#[test]
fn kernel_kind_defaults_to_generic_for_unknown_pattern() {
    let mut interp = make_interp();
    let tokens = vec![sym("MAP")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.kernel_kind, KernelKind::GenericCompiled);
    assert!(qb.fast_path_id.is_none());
    assert!(!qb.lowered_kernel_ir.is_empty());
}

// ---------------------------------------------------------------------------
// Dependency word collection
// ---------------------------------------------------------------------------

/// A block that calls a user-defined word records it in `dependency_words`.
/// The resolved name may be namespace-qualified (e.g. "NS@MY_ADD"), so we check
/// that at least one entry contains the word name.
#[test]
fn dependency_words_collected() {
    let mut interp = make_interp();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        interp.execute("{ + } 'MY_ADD' DEF").await.unwrap();
    });
    let tokens = vec![sym("MY_ADD")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert!(
        qb.dependency_words.iter().any(|w| w.contains("MY_ADD")),
        "expected 'MY_ADD' (possibly namespace-qualified) in dependency_words: {:?}",
        qb.dependency_words
    );
}

// ---------------------------------------------------------------------------
// End-to-end: verify quantized path is used for each HOF
// ---------------------------------------------------------------------------

#[test]
fn quantized_path_used_for_map() {
    // { [ 1 ] + } increments each element
    let interp = run_code("[ 1 2 3 ] { [ 1 ] + } MAP");
    assert!(interp.runtime_metrics().quantized_block_use_count >= 3);
}

#[test]
fn quantized_path_used_for_filter() {
    // { [ 0 ] <= NOT } = elem > 0
    let interp = run_code("[ -2 -1 0 1 2 3 ] { [ 0 ] <= NOT } FILTER");
    assert!(interp.runtime_metrics().quantized_block_use_count >= 1);
}

#[test]
fn quantized_path_used_for_any() {
    let interp = run_code("[ 1 2 3 ] { [ 2 ] = } ANY");
    assert!(interp.runtime_metrics().quantized_block_use_count >= 1);
}

#[test]
fn quantized_path_used_for_all() {
    // { [ 0 ] <= NOT } = elem > 0
    let interp = run_code("[ 1 2 3 ] { [ 0 ] <= NOT } ALL");
    assert!(interp.runtime_metrics().quantized_block_use_count >= 1);
}

#[test]
fn quantized_path_used_for_count() {
    // { [ 3 ] <= NOT } = elem > 3
    let interp = run_code("[ 1 2 3 4 5 ] { [ 3 ] <= NOT } COUNT");
    assert!(interp.runtime_metrics().quantized_block_use_count >= 1);
}

#[test]
fn quantized_path_used_for_fold() {
    let interp = run_code("[ 1 2 3 4 5 ] [ 0 ] { + } FOLD");
    assert!(interp.runtime_metrics().quantized_block_use_count >= 1);
}

#[test]
fn quantized_path_used_for_scan() {
    let interp = run_code("[ 1 2 3 ] [ 0 ] { + } SCAN");
    assert!(interp.runtime_metrics().quantized_block_use_count >= 1);
}

// ---------------------------------------------------------------------------
// Result correctness: quantized path must produce the same semantics
// ---------------------------------------------------------------------------

#[test]
fn filter_keeps_elements_above_zero() {
    // { [ 0 ] <= NOT } = elem > 0
    let interp = run_code("[ -2 -1 0 1 2 3 ] { [ 0 ] <= NOT } FILTER");
    let top = stack_top(&interp);
    assert_eq!(top.len(), 3, "expected [1, 2, 3], got len={}", top.len());
}

#[test]
fn filter_with_simple_fast_predicate_pattern() {
    let interp = run_code("[ -2 -1 0 1 2 3 ] { [ 2 ] < } FILTER");
    let top = stack_top(&interp);
    assert_eq!(
        top.len(),
        4,
        "expected [-2, -1, 0, 1], got len={}",
        top.len()
    );
}

#[test]
fn any_true_when_element_matches() {
    let interp = run_code("[ 1 2 3 ] { [ 2 ] = } ANY");
    assert!(stack_top_bool(&interp));
}

#[test]
fn any_with_simple_fast_predicate_pattern() {
    let interp = run_code("[ 1 2 3 ] { [ 2 ] = } ANY");
    assert!(stack_top_bool(&interp));
}

#[test]
fn any_false_when_no_match() {
    let interp = run_code("[ 1 2 3 ] { [ 5 ] = } ANY");
    assert!(!stack_top_bool(&interp));
}

#[test]
fn all_true_when_all_above_zero() {
    // { [ 0 ] <= NOT } = elem > 0
    let interp = run_code("[ 1 2 3 ] { [ 0 ] <= NOT } ALL");
    assert!(stack_top_bool(&interp));
}

#[test]
fn all_false_when_one_fails() {
    // -1 > 0 is false, so ALL should be false
    let interp = run_code("[ -1 2 3 ] { [ 0 ] <= NOT } ALL");
    assert!(!stack_top_bool(&interp));
}

#[test]
fn count_counts_matching_elements() {
    // { [ 3 ] <= NOT } = elem > 3, so 4 and 5 match
    let interp = run_code("[ 1 2 3 4 5 ] { [ 3 ] <= NOT } COUNT");
    assert_eq!(stack_top_i64(&interp), 2);
}

#[test]
fn fold_sum_is_correct() {
    let interp = run_code("[ 1 2 3 4 5 ] [ 0 ] { + } FOLD");
    assert_eq!(stack_top_i64(&interp), 15);
}

#[test]
fn fold_product_is_correct() {
    let interp = run_code("[ 1 2 3 4 ] [ 1 ] { * } FOLD");
    assert_eq!(stack_top_i64(&interp), 24);
}

#[test]
fn scan_produces_prefix_sums() {
    let interp = run_code("[ 1 2 3 ] [ 0 ] { + } SCAN");
    let top = stack_top(&interp);
    assert_eq!(top.len(), 3, "SCAN result should have 3 elements");
    // Each child may be a scalar or a single-element vector — extract i64 from either.
    let extract = |v: Value| -> i64 {
        if let Some(f) = v.as_scalar() {
            return f.to_i64().expect("scalar should be i64");
        }
        if v.len() == 1 {
            if let Some(child) = v.child(0) {
                if let Some(f) = child.as_scalar() {
                    return f.to_i64().expect("inner scalar should be i64");
                }
            }
        }
        panic!("SCAN element is not a numeric scalar or single-element vector");
    };
    let vals: Vec<i64> = (0..3)
        .map(|i| {
            extract(
                top.child(i)
                    .expect("len==3 implies child(i) exists for i<3"),
            )
        })
        .collect();
    assert_eq!(vals, vec![1, 3, 6]);
}

// ---------------------------------------------------------------------------
// Parity tests: quantized code block vs plain word-name path
// ---------------------------------------------------------------------------

#[test]
fn filter_quantized_vs_plain_parity() {
    let q = run_code("[ -2 -1 0 1 2 3 ] { [ 0 ] <= NOT } FILTER");
    let p = run_code("{ [ 0 ] <= NOT } 'GT0' DEF [ -2 -1 0 1 2 3 ] 'GT0' FILTER");
    assert_eq!(q.get_stack(), p.get_stack());
}

#[test]
fn any_quantized_vs_plain_parity() {
    let q = run_code("[ 1 2 3 ] { [ 2 ] = } ANY");
    let p = run_code("{ [ 2 ] = } 'EQ2' DEF [ 1 2 3 ] 'EQ2' ANY");
    assert_eq!(q.get_stack(), p.get_stack());
}

#[test]
fn all_quantized_vs_plain_parity() {
    let q = run_code("[ 1 2 3 ] { [ 0 ] <= NOT } ALL");
    let p = run_code("{ [ 0 ] <= NOT } 'GT0' DEF [ 1 2 3 ] 'GT0' ALL");
    assert_eq!(q.get_stack(), p.get_stack());
}

#[test]
fn count_quantized_vs_plain_parity() {
    let q = run_code("[ 1 2 3 4 5 ] { [ 3 ] <= NOT } COUNT");
    let p = run_code("{ [ 3 ] <= NOT } 'GT3' DEF [ 1 2 3 4 5 ] 'GT3' COUNT");
    assert_eq!(q.get_stack(), p.get_stack());
}

#[test]
fn fold_quantized_vs_plain_parity() {
    let q = run_code("[ 1 2 3 4 5 ] [ 0 ] { + } FOLD");
    let p = run_code("{ + } 'SUMX' DEF [ 1 2 3 4 5 ] [ 0 ] 'SUMX' FOLD");
    assert_eq!(q.get_stack(), p.get_stack());
}

#[test]
fn scan_quantized_vs_plain_parity() {
    let q = run_code("[ 1 2 3 ] [ 0 ] { + } SCAN");
    let p = run_code("{ + } 'SUMX' DEF [ 1 2 3 ] [ 0 ] 'SUMX' SCAN");
    assert_eq!(q.get_stack(), p.get_stack());
}

#[test]
fn predicate_error_stack_shape_matches_between_quantized_and_plain() {
    let q = run_code_result("[ 1 2 ] { [ 1 2 ] } FILTER")
        .err()
        .expect("quantized should error");
    let p = run_code_result("{ [ 1 2 ] } 'PAIR' DEF [ 1 2 ] 'PAIR' FILTER")
        .err()
        .expect("plain should error");
    assert!(q.to_string().contains("boolean result"));
    assert!(p.to_string().contains("boolean result"));
}

#[test]
fn fold_error_stack_shape_matches_between_quantized_and_plain() {
    let q = run_code_result("[ 1 2 3 ] [ 0 ] { + + } FOLD")
        .err()
        .expect("quantized should error");
    let p = run_code_result("{ + + } 'BAD' DEF [ 1 2 3 ] [ 0 ] 'BAD' FOLD")
        .err()
        .expect("plain should error");
    assert!(q.to_string().contains("Stack underflow"));
    assert!(p.to_string().contains("Stack underflow"));
}

// ---------------------------------------------------------------------------
// Purity: purity_table-based classification (Problem 1)
// ---------------------------------------------------------------------------

#[test]
fn impure_now_is_rejected_at_gate() {
    // NOW is impure (time / non-determinism); the Phase 1-C gate rejects it.
    let mut interp = make_interp();
    let tokens = vec![sym("NOW")];
    assert!(!is_quantizable_block(&tokens));
    assert!(quantize_code_block(&tokens, &mut interp).is_none());
}

#[test]
fn impure_csprng_is_rejected_at_gate() {
    let mut interp = make_interp();
    let tokens = vec![sym("CSPRNG")];
    assert!(!is_quantizable_block(&tokens));
    assert!(quantize_code_block(&tokens, &mut interp).is_none());
}

#[test]
fn impure_spawn_is_rejected_at_gate() {
    let mut interp = make_interp();
    let tokens = vec![sym("SPAWN")];
    assert!(!is_quantizable_block(&tokens));
    assert!(quantize_code_block(&tokens, &mut interp).is_none());
}

#[test]
fn purity_higher_order_dispatcher_alone_is_pure() {
    // After Phase 1-B, MAP is a pure dispatcher; a block consisting of
    // just `MAP` (with no callback in its tokens) is therefore Pure.
    let mut interp = make_interp();
    let tokens = vec![sym("MAP")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.purity, QuantizedPurity::Pure);
}

#[test]
fn purity_hof_with_impure_callback_is_side_effecting() {
    // PushCodeBlock recursion: a HOF whose callback contains an impure
    // user word should propagate impurity to the enclosing block.
    // (An explicit impure builtin in the callback is caught earlier by
    // the gate, so we use an impure user word to exercise the recursion.)
    let mut interp = make_interp();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        interp.execute("{ PRINT } 'SHOUT' DEF").await.unwrap();
    });
    let tokens = vec![
        Token::VectorStart,
        num("1"),
        Token::VectorEnd,
        Token::BlockStart,
        sym("SHOUT"),
        Token::BlockEnd,
        sym("MAP"),
    ];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.purity, QuantizedPurity::SideEffecting);
    assert!(!qb.can_fuse);
}

#[test]
fn purity_hof_with_pure_callback_is_pure() {
    // Companion to the impure-callback test: a pure callback leaves the
    // outer HOF block Pure.
    let mut interp = make_interp();
    let tokens = vec![
        Token::VectorStart,
        num("1"),
        num("2"),
        Token::VectorEnd,
        Token::BlockStart,
        Token::VectorStart,
        num("2"),
        Token::VectorEnd,
        sym("*"),
        Token::BlockEnd,
        sym("MAP"),
    ];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.purity, QuantizedPurity::Pure);
}

// ---------------------------------------------------------------------------
// Purity: CallUserWord propagation (Problem 2)
// ---------------------------------------------------------------------------

#[test]
fn purity_pure_user_word_is_pure() {
    let mut interp = make_interp();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        interp.execute("{ + } 'MY_ADD' DEF").await.unwrap();
    });
    let tokens = vec![sym("MY_ADD")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(
        qb.purity,
        QuantizedPurity::Pure,
        "pure user word should propagate Pure"
    );
    assert!(qb.can_fuse);
}

#[test]
fn purity_impure_user_word_is_side_effecting() {
    let mut interp = make_interp();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        interp.execute("{ PRINT } 'SHOUT' DEF").await.unwrap();
    });
    let tokens = vec![sym("SHOUT")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.purity, QuantizedPurity::SideEffecting);
}

#[test]
fn purity_nested_pure_user_word_is_pure() {
    let mut interp = make_interp();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        interp.execute("{ + } 'SUMX' DEF").await.unwrap();
        interp
            .execute("{ SUMX SUMX } 'SUMX_TWICE' DEF")
            .await
            .unwrap();
    });
    let tokens = vec![sym("ADD_TWICE")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.purity, QuantizedPurity::Pure);
}

#[test]
fn purity_recursive_user_word_is_conservative() {
    // A self-recursive word should not infinite-loop; fall back to SideEffecting.
    let mut interp = make_interp();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        interp.execute("{ REC } 'REC' DEF").await.unwrap();
    });
    let tokens = vec![sym("REC")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.purity, QuantizedPurity::SideEffecting);
}

// ---------------------------------------------------------------------------
// Arity: partial info preservation before first unknown op (Problem 3)
// ---------------------------------------------------------------------------

#[test]
fn arity_partial_info_preserved_before_unknown() {
    // `{ + UNKNOWN }` — `+` fixes input arity as 2, rest is unknown.
    let mut interp = make_interp();
    let tokens = vec![sym("+"), sym("SOME_UNKNOWN_WORD")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(
        qb.input_arity,
        QuantizedArity::Fixed(2),
        "input arity should be preserved from prefix analysis"
    );
    assert_eq!(
        qb.output_arity,
        QuantizedArity::Variable,
        "output arity is indeterminate after unknown op"
    );
}

#[test]
fn arity_fully_known_still_fixed() {
    // Regression check: fully-known plan still returns Fixed on both sides.
    let mut interp = make_interp();
    let tokens = vec![num("1"), sym("+")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.input_arity, QuantizedArity::Fixed(1));
    assert_eq!(qb.output_arity, QuantizedArity::Fixed(1));
}

// ---------------------------------------------------------------------------
// Virtual Tensor Unit (VTU) classification
// ---------------------------------------------------------------------------

#[test]
fn vtu_hint_map_unary_is_strong_candidate() {
    // `[ 1 ] +` is the const-vector pattern detected as MapUnaryPure.
    let mut interp = make_interp();
    let tokens = vec![Token::VectorStart, num("1"), Token::VectorEnd, sym("+")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.kernel_kind, KernelKind::MapUnaryPure);
    assert_eq!(qb.vtu_hint.suitability, VtuSuitability::StrongCandidate);
    assert!(qb.vtu_hint.is_candidate());
}

#[test]
fn vtu_hint_predicate_unary_is_strong_candidate() {
    let mut interp = make_interp();
    let tokens = vec![sym("NOT")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.kernel_kind, KernelKind::PredicateUnaryPure);
    assert_eq!(qb.vtu_hint.suitability, VtuSuitability::StrongCandidate);
}

#[test]
fn vtu_hint_fold_binary_is_weak_candidate() {
    // A bare `+` with no const-vector wrapper is FoldBinaryPure.
    let mut interp = make_interp();
    let tokens = vec![sym("+")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.kernel_kind, KernelKind::FoldBinaryPure);
    // Reductions are weak candidates because parallel reduction needs an
    // Approx boundary; Ajisai is exact-by-default.
    assert_eq!(qb.vtu_hint.suitability, VtuSuitability::WeakCandidate);
}

#[test]
fn vtu_hint_generic_pure_is_weak_candidate() {
    // Two pushes plus DUP — quantizable but no kernel pattern, so generic.
    let mut interp = make_interp();
    let tokens = vec![num("1"), num("2"), sym("DUP")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert_eq!(qb.kernel_kind, KernelKind::GenericCompiled);
    assert_eq!(qb.purity, QuantizedPurity::Pure);
    assert_eq!(qb.vtu_hint.suitability, VtuSuitability::WeakCandidate);
}

#[test]
fn vtu_hint_map_unary_includes_sparse_tensor_loop_without_guard_effect() {
    let mut interp = make_interp();
    let tokens = vec![Token::VectorStart, num("1"), Token::VectorEnd, sym("+")];
    let qb = quantize_code_block(&tokens, &mut interp).unwrap();
    assert!(qb
        .vtu_hint
        .backend_candidates
        .contains(&VtuBackendCandidate::SparseTensorLoop));
    assert_eq!(qb.guard_signature.kernel_kind, KernelKind::MapUnaryPure);
}

#[test]
fn vtu_sparse_candidate_metrics_increment_for_dense_tensor_unary() {
    let mut metrics = RuntimeMetrics::default();
    let mut data = vec![Fraction::from(0_i64); 64];
    data[7] = Fraction::from(5_i64);
    let value = Value::from_tensor(data, vec![64]);

    let _ = apply_unary_flat_with_metrics(&value, |f| f.clone(), Some(&mut metrics))
        .expect("unary tensor operation should succeed");

    assert_eq!(metrics.vtu_sparse_candidate_count, 1);
    assert_eq!(metrics.vtu_sparse_candidate_elements, 64);
    assert_eq!(metrics.vtu_sparse_candidate_nonzero_elements, 1);
    assert_eq!(metrics.vtu_sparse_skippable_zero_elements, 63);
}

#[test]
fn vtu_sparse_candidate_metrics_skip_shape_mismatch_and_nil_rejection() {
    let mut sparse_data = vec![Fraction::from(0_i64); 64];
    sparse_data[3] = Fraction::from(2_i64);
    let sparse = Value::from_tensor(sparse_data, vec![8, 8]);
    let mismatched = Value::from_tensor(vec![Fraction::from(1_i64); 9], vec![3, 3]);

    let mut shape_metrics = RuntimeMetrics::default();
    assert!(apply_binary_broadcast_with_metrics(
        &sparse,
        &mismatched,
        |a, b| Ok(a.add(b)),
        Some(&mut shape_metrics),
    )
    .is_err());
    assert_eq!(shape_metrics.vtu_sparse_candidate_count, 0);

    let mut nil_metrics = RuntimeMetrics::default();
    assert!(apply_binary_broadcast_with_metrics(
        &Value::nil(),
        &sparse,
        |a, b| Ok(a.add(b)),
        Some(&mut nil_metrics),
    )
    .is_err());
    assert_eq!(nil_metrics.vtu_sparse_candidate_count, 0);
}

#[test]
fn vtu_metrics_increment_on_quantize() {
    let mut interp = make_interp();
    let baseline = interp.runtime_metrics().vtu_candidate_block_count;
    let tokens = vec![sym("NOT")];
    let _ = quantize_code_block(&tokens, &mut interp).unwrap();
    let after = interp.runtime_metrics().vtu_candidate_block_count;
    assert_eq!(after, baseline + 1);
}

#[test]
fn vtu_default_metrics_are_zero() {
    let interp = make_interp();
    let m = interp.runtime_metrics();
    assert_eq!(m.vtu_tensor_flatten_count, 0);
    assert_eq!(m.vtu_tensor_flattened_elements, 0);
    assert_eq!(m.vtu_tensor_rebuild_count, 0);
    assert_eq!(m.vtu_tensor_rebuilt_elements, 0);
    assert_eq!(m.vtu_broadcast_count, 0);
    assert_eq!(m.vtu_unary_flat_count, 0);
    assert_eq!(m.vtu_allocated_elements, 0);
    assert_eq!(m.vtu_same_shape_elementwise_count, 0);
    assert_eq!(m.vtu_projected_broadcast_count, 0);
    assert_eq!(m.vtu_simd_kernel_use_count, 0);
    assert_eq!(m.vtu_sparse_candidate_count, 0);
    assert_eq!(m.vtu_sparse_candidate_elements, 0);
    assert_eq!(m.vtu_sparse_candidate_nonzero_elements, 0);
    assert_eq!(m.vtu_sparse_skippable_zero_elements, 0);
    assert_eq!(m.vtu_candidate_block_count, 0);
    assert_eq!(m.vtu_rejected_block_count, 0);
    assert_eq!(m.vtu_fusion_candidate_count, 0);
    assert_eq!(m.vtu_bulk_kernel_use_count, 0);
}

#[test]
fn vtu_same_shape_broadcast_increments_counters() {
    // `[ 1 2 3 ] [ 4 5 6 ] +` -> same-shape elementwise.
    let interp = run_code("[ 1 2 3 ] [ 4 5 6 ] +");
    let m = interp.runtime_metrics();
    assert!(m.vtu_broadcast_count >= 1, "broadcast count should fire");
    assert!(
        m.vtu_same_shape_elementwise_count >= 1,
        "same-shape fast path should fire"
    );
    assert_eq!(m.vtu_projected_broadcast_count, 0);
    assert!(m.vtu_tensor_flatten_count >= 2, "two operands flattened");
    assert!(m.vtu_allocated_elements >= 3);
}

#[test]
fn vtu_projected_broadcast_increments_counters() {
    // Scalar broadcast over a 3-vector exercises the projection path.
    let interp = run_code("[ 1 2 3 ] [ 10 ] +");
    let m = interp.runtime_metrics();
    assert!(m.vtu_broadcast_count >= 1);
    assert!(
        m.vtu_projected_broadcast_count >= 1,
        "projection path should fire when shapes differ"
    );
}

#[test]
fn vtu_unary_flat_increments_counter() {
    // FLOOR over a vector exercises apply_unary_flat_with_metrics.
    let interp = run_code("[ 1/2 3/2 5/2 ] FLOOR");
    let m = interp.runtime_metrics();
    assert!(m.vtu_unary_flat_count >= 1, "unary flat path should fire");
    assert!(m.vtu_tensor_flatten_count >= 1);
    // VTU Phase II: outputs are now Tensor directly, no rebuild needed.
    assert_eq!(
        m.vtu_tensor_rebuild_count, 0,
        "rebuild path should be retired after Phase II producer switch"
    );
    assert!(m.vtu_allocated_elements >= 3);
}

// ---------------------------------------------------------------------------
// VTU Phase III bulk fast path: 1-D dense Tensor inputs to MAP/FILTER/FOLD/
// ANY/ALL/COUNT iterate the underlying `&[Fraction]` directly.
// ---------------------------------------------------------------------------

#[test]
fn vtu_phase_iii_map_bulk_kernel_fires_and_matches_per_element() {
    let bulk = run_code("[ 1 2 3 4 ] { [ 2 ] * } MAP");
    let plain = run_code("{ [ 2 ] * } 'DBL' DEF [ 1 2 3 4 ] 'DBL' MAP");
    assert_eq!(bulk.get_stack(), plain.get_stack());
    let m = bulk.runtime_metrics();
    assert!(
        m.vtu_bulk_kernel_use_count >= 1,
        "MAP over dense Tensor + fast unary kernel should take the bulk path"
    );
}

#[test]
fn vtu_phase_iii_filter_bulk_kernel_matches_per_element() {
    let bulk = run_code("[ 1 2 3 4 ] { [ 2 ] = } FILTER");
    let plain = run_code("{ [ 2 ] = } 'EQ2' DEF [ 1 2 3 4 ] 'EQ2' FILTER");
    assert_eq!(bulk.get_stack(), plain.get_stack());
    assert!(bulk.runtime_metrics().vtu_bulk_kernel_use_count >= 1);
}

#[test]
fn vtu_phase_iii_fold_bulk_kernel_matches_per_element() {
    let bulk = run_code("[ 1 2 3 4 ] [ 0 ] { + } FOLD");
    let plain = run_code("{ + } 'PLUS' DEF [ 1 2 3 4 ] [ 0 ] 'PLUS' FOLD");
    assert_eq!(bulk.get_stack(), plain.get_stack());
    assert!(bulk.runtime_metrics().vtu_bulk_kernel_use_count >= 1);
}

#[test]
fn vtu_phase_iii_any_bulk_kernel_matches_per_element() {
    let bulk = run_code("[ 1 2 3 ] { [ 2 ] = } ANY");
    let plain = run_code("{ [ 2 ] = } 'EQ2' DEF [ 1 2 3 ] 'EQ2' ANY");
    assert_eq!(bulk.get_stack(), plain.get_stack());
    assert!(bulk.runtime_metrics().vtu_bulk_kernel_use_count >= 1);
}

#[test]
fn vtu_phase_iii_all_bulk_kernel_matches_per_element() {
    // Predicate `{ [ 0 ] = }` -> elements equal to 0; none in [1 2 3] so ALL=FALSE.
    let bulk = run_code("[ 1 2 3 ] { [ 0 ] = } ALL");
    let plain = run_code("{ [ 0 ] = } 'EQ0' DEF [ 1 2 3 ] 'EQ0' ALL");
    assert_eq!(bulk.get_stack(), plain.get_stack());
    assert!(bulk.runtime_metrics().vtu_bulk_kernel_use_count >= 1);
}

#[test]
fn vtu_phase_iii_count_bulk_kernel_matches_per_element() {
    let bulk = run_code("[ 1 2 3 4 ] { [ 2 ] = } COUNT");
    let plain = run_code("{ [ 2 ] = } 'EQ2' DEF [ 1 2 3 4 ] 'EQ2' COUNT");
    assert_eq!(bulk.get_stack(), plain.get_stack());
    assert!(bulk.runtime_metrics().vtu_bulk_kernel_use_count >= 1);
}

#[test]
fn vtu_phase_iii_map_bulk_division_by_zero_bubbles() {
    // A zero divisor is the generic route's case: the kernel declines and
    // the Bubble Rule yields a NIL bubble per element, exactly as the same
    // division would outside MAP. Route equivalence is pinned in
    // `fast_kernel_route_tests.rs`.
    let interp =
        run_code_result("[ 1 2 3 ] { [ 0 ] / } MAP").expect("division by zero must not error");
    let result = interp.get_stack().last().cloned().expect("MAP result");
    assert_eq!(result.len(), 3, "MAP must keep the element count");
    for i in 0..3 {
        assert!(
            result.child(i).map(|e| e.is_nil()).unwrap_or(false),
            "element {i} must be a NIL bubble"
        );
    }
}

#[test]
fn vtu_phase_iii_bulk_metric_zero_for_user_word_kernel() {
    // User word does not match a fast unary kernel -> per-element path,
    // bulk metric stays at 0.
    let interp = run_code("{ [ 2 ] * } 'DBL' DEF [ 1 2 3 ] 'DBL' MAP");
    assert_eq!(
        interp.runtime_metrics().vtu_bulk_kernel_use_count,
        0,
        "user-word kernel should not take the bulk path"
    );
}

// ---------------------------------------------------------------------------
// VTU Phase III consumer ops: SHAPE / RANK / RESHAPE / TRANSPOSE / FILL /
// JSON-STRINGIFY / JOIN / SORT must all accept a dense Tensor input and
// produce the same observable result they produced for nested Vector inputs
// before the producer switch.
// ---------------------------------------------------------------------------

#[test]
fn vtu_phase_iii_shape_accepts_tensor_input() {
    // [ 1 2 3 ] is now a dense Tensor under Phase II. SHAPE must still
    // return [ 3 ].
    let interp = run_code("[ 1 2 3 ] SHAPE");
    let top = stack_top(&interp);
    assert!(top.is_vector(), "SHAPE result should be a vector");
    assert_eq!(top.len(), 1);
    let dim = top
        .child(0)
        .and_then(|c| c.as_scalar().and_then(|f| f.to_i64()));
    assert_eq!(dim, Some(3));
}

#[test]
fn vtu_phase_iii_rank_accepts_tensor_input() {
    let interp = run_code("[ 1 2 3 ] RANK");
    let top = stack_top(&interp);
    let rank = top.as_scalar().and_then(|f| f.to_i64()).or_else(|| {
        top.child(0)
            .and_then(|c| c.as_scalar().and_then(|f| f.to_i64()))
    });
    assert_eq!(rank, Some(1));
}

#[test]
fn vtu_phase_iii_reshape_dense_to_2d_matches_nested() {
    let dense = run_code("[ 1 2 3 4 ] [ 2 2 ] RESHAPE");
    // SHAPE of the result should be [ 2 2 ].
    let mut sh = run_code("[ 1 2 3 4 ] [ 2 2 ] RESHAPE SHAPE");
    let shape_top = sh.stack.pop().unwrap();
    assert!(shape_top.is_vector());
    assert_eq!(shape_top.len(), 2);
    // The reshaped value should round-trip through TRANSPOSE -> TRANSPOSE
    // back to itself.
    let twice = run_code("[ 1 2 3 4 ] [ 2 2 ] RESHAPE TRANSPOSE TRANSPOSE");
    assert_eq!(dense.get_stack(), twice.get_stack());
}

#[test]
fn vtu_phase_iii_transpose_accepts_tensor_2d() {
    // Transposing [[1 2 3] [4 5 6]] should yield [[1 4] [2 5] [3 6]].
    let interp = run_code("[ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE");
    let top = stack_top(&interp);
    assert!(top.is_vector());
    assert_eq!(top.shape(), vec![3, 2]);
}

#[test]
fn vtu_phase_iii_fill_produces_dense_tensor() {
    let interp = run_code("[ 3 0 ] FILL");
    let top = stack_top(&interp);
    assert_eq!(top.shape(), vec![3]);
    for i in 0..3 {
        let elem = top.child(i).unwrap();
        assert_eq!(elem.as_scalar().and_then(|f| f.to_i64()), Some(0));
    }
}

#[test]
fn vtu_phase_iii_join_accepts_tensor_of_codepoints() {
    // [ 65 66 67 ] is a dense Tensor. JOIN must still treat its elements as
    // Unicode code points and produce 'ABC'.
    let interp = run_code("[ 65 66 67 ] JOIN");
    let top = stack_top(&interp);
    assert_eq!(format!("{}", top), "'ABC'");
}

#[test]
fn vtu_phase_iii_sort_accepts_tensor_input() {
    // [ 3 1 2 ] is a dense Tensor under Phase II producers. SORT must
    // accept it via the new boundary-helper code path.
    let interp = run_code("'algo' IMPORT [ 3 1 2 ] SORT");
    let top = stack_top(&interp);
    assert_eq!(top.shape(), vec![3]);
    let collected: Vec<i64> = (0..top.len())
        .map(|i| top.child(i).unwrap().as_scalar().unwrap().to_i64().unwrap())
        .collect();
    assert_eq!(collected, vec![1, 2, 3]);
}

// ---------------------------------------------------------------------------
// VTU Phase III consumer ops: COMPARE / LOGIC / CAST must accept a dense
// Tensor input and behave identically to the nested-Vector input path.
// ---------------------------------------------------------------------------

#[test]
fn vtu_phase_iii_eq_dense_tensor_singleton_matches_scalar() {
    // [ 5 ] becomes a Tensor[1] under Phase II producers. EQ between a
    // scalar and a singleton tensor must collapse to TRUE / FALSE.
    let true_case = run_code("[ 5 ] [ 5 ] =");
    let false_case = run_code("[ 5 ] [ 6 ] =");
    let t = stack_top(&true_case);
    let f = stack_top(&false_case);
    let truthy = |v: &Value| -> bool {
        v.as_truth()
            .or_else(|| v.as_scalar().map(|f| !f.is_zero()))
            .or_else(|| {
                v.child(0)
                    .and_then(|c| c.as_truth().or_else(|| c.as_scalar().map(|f| !f.is_zero())))
            })
            .unwrap_or(false)
    };
    assert!(truthy(t), "5 = 5 should be truthy");
    assert!(!truthy(f), "5 = 6 should be falsy");
}

#[test]
fn vtu_phase_iii_lt_dense_tensor_against_scalar() {
    let interp = run_code("[ 1 ] [ 2 ] <");
    let t = stack_top(&interp);
    let truthy = t
        .as_truth()
        .or_else(|| t.as_scalar().map(|f| !f.is_zero()))
        .or_else(|| {
            t.child(0)
                .and_then(|c| c.as_truth().or_else(|| c.as_scalar().map(|f| !f.is_zero())))
        })
        .unwrap_or(false);
    assert!(truthy, "1 < 2 should be truthy");
}

#[test]
fn vtu_phase_iii_not_over_dense_tensor() {
    // NOT applied to [ 0 1 0 ] should produce [ 1 0 1 ] regardless of layout.
    let interp = run_code("[ 0 1 0 ] NOT");
    let top = stack_top(&interp);
    assert_eq!(top.shape(), vec![3]);
    let collected: Vec<i64> = (0..top.len())
        .map(|i| top.child(i).unwrap().as_scalar().unwrap().to_i64().unwrap())
        .collect();
    assert_eq!(collected, vec![1, 0, 1]);
}

#[test]
fn vtu_phase_iii_and_or_truthiness_on_dense_tensor() {
    // AND/OR consume from the stack; the operands are Tensor[1] under Phase II.
    // [ 1 ] AND [ 1 ] -> 1, [ 0 ] AND [ 1 ] -> 0, [ 0 ] OR [ 1 ] -> 1
    let and_true = run_code("[ 1 ] [ 1 ] AND");
    let and_false = run_code("[ 0 ] [ 1 ] AND");
    let or_true = run_code("[ 0 ] [ 1 ] OR");
    let truthy = |interp: &Interpreter| -> bool {
        let t = stack_top(interp);
        t.as_scalar()
            .map(|f| !f.is_zero())
            .or_else(|| t.child(0).and_then(|c| c.as_scalar().map(|f| !f.is_zero())))
            .unwrap_or(false)
    };
    assert!(truthy(&and_true), "[1] AND [1] should be truthy");
    assert!(!truthy(&and_false), "[0] AND [1] should be falsy");
    assert!(truthy(&or_true), "[0] OR [1] should be truthy");
}

#[test]
fn vtu_phase_iii_chars_decomposes_string_independent_of_target_layout() {
    // CHARS expects a string and produces the code-point sequence. The string
    // representation is preserved by Display::String hint, but verify the
    // round-trip CHARS -> JOIN over the produced sequence matches the input
    // even if intermediates pass through Tensor producers.
    let interp = run_code("'ABC' CHARS JOIN");
    let top = stack_top(&interp);
    assert_eq!(format!("{}", top), "'ABC'");
}
