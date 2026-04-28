use crate::interpreter::quantized_block::{
    is_quantizable_block, quantize_code_block, KernelKind, QuantizedArity, QuantizedPurity,
};
use crate::interpreter::Interpreter;
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
        if let Some(child) = top.get_child(0) {
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
    if let Some(f) = top.as_scalar() {
        return !f.is_zero();
    }
    if top.len() == 1 {
        if let Some(child) = top.get_child(0) {
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

#[test]
fn safemode_token_makes_block_non_quantizable() {
    // Token::SafeMode is the safe-mode sentinel
    let tokens = vec![Token::SafeMode, sym("+")];
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
    let extract = |v: &Value| -> i64 {
        if let Some(f) = v.as_scalar() {
            return f.to_i64().expect("scalar should be i64");
        }
        if v.len() == 1 {
            if let Some(child) = v.get_child(0) {
                if let Some(f) = child.as_scalar() {
                    return f.to_i64().expect("inner scalar should be i64");
                }
            }
        }
        panic!("SCAN element is not a numeric scalar or single-element vector");
    };
    let vals: Vec<i64> = (0..3).map(|i| extract(top.get_child(i).unwrap())).collect();
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
