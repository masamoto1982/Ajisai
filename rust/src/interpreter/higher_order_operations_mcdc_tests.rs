// AQ-VER-006: Interpreter execution-semantics MC/DC tests for QL-A boolean
// decisions.
//
// Scope: interpreter dispatch — boolean decisions whose atomic conditions can
// each cause an incorrect mode selection, cache invalidation, or quantized
// block admission if mis-evaluated.
//
// Each submodule below documents:
//   * DUT (decision under test) — file:line and the boolean expression
//   * Conditions — atomic predicates A, B, ...
//   * MC/DC table — rows that demonstrate each condition independently
//     flipping the outcome, along with the specific pair used
//
// Trace: docs/quality/TRACEABILITY_MATRIX.md, requirement AQ-REQ-006.

#![cfg(test)]

use crate::elastic::ElasticMode;
use crate::interpreter::quantized_block::is_quantizable_block;
use crate::interpreter::Interpreter;
use crate::types::Token;

// ---------------------------------------------------------------------------
// AQ-VER-006-A
// DUT: rust/src/interpreter/execute-builtin.rs:33-38 in `Interpreter::is_hedged_mode`
//
//     matches!(
//         self.elastic_mode(),
//         ElasticMode::HedgedSafe | ElasticMode::HedgedTrace
//     )
//
// Conditions:
//   A = (elastic_mode == HedgedSafe)
//   B = (elastic_mode == HedgedTrace)
//
// The enum ElasticMode has four variants and at most one is active at a time,
// so the combination (A=T, B=T) is structurally impossible. MC/DC for the
// disjunction A||B therefore uses the remaining three rows:
//
//   row 1: (A=T, B=F) - HedgedSafe    -> true
//   row 2: (A=F, B=T) - HedgedTrace   -> true
//   row 3: (A=F, B=F) - Greedy        -> false
//   row 4: (A=F, B=F) - FastGuarded   -> false (same equivalence class as row 3)
//
// Independent effect:
//   Pair (1, 3): A flips T->F, B held F -> outcome flips T->F (A independent).
//   Pair (2, 3): B flips T->F, A held F -> outcome flips T->F (B independent).
// ---------------------------------------------------------------------------
mod hedged_mode_classifier {
    use super::*;

    #[test]
    fn aq_ver_006_a_row1_hedged_safe_is_hedged() {
        // (A=T, B=F) -> true
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::HedgedSafe);
        assert!(
            interp.is_hedged_mode(),
            "HedgedSafe must classify as hedged"
        );
    }

    #[test]
    fn aq_ver_006_a_row2_hedged_trace_is_hedged() {
        // (A=F, B=T) -> true
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::HedgedTrace);
        assert!(
            interp.is_hedged_mode(),
            "HedgedTrace must classify as hedged"
        );
    }

    #[test]
    fn aq_ver_006_a_row3_greedy_is_not_hedged() {
        // (A=F, B=F) — Greedy — completes independence pair (1,3) and (2,3).
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::Greedy);
        assert!(
            !interp.is_hedged_mode(),
            "Greedy must not classify as hedged"
        );
    }

    #[test]
    fn aq_ver_006_a_row4_fast_guarded_is_not_hedged() {
        // (A=F, B=F) — FastGuarded — same equivalence class as Greedy.
        // Included to show the non-hedged arm handles all non-Hedged variants,
        // not only the default Greedy.
        let mut interp = Interpreter::new();
        interp.set_elastic_mode(ElasticMode::FastGuarded);
        assert!(
            !interp.is_hedged_mode(),
            "FastGuarded must not classify as hedged"
        );
    }
}

// ---------------------------------------------------------------------------
// AQ-VER-006-B
// DUT: rust/src/interpreter/quantized-block.rs:266-271 in `is_quantizable_block`
//
//     !tokens.is_empty()
//         && !tokens.iter().any(|t| matches!(t, Token::LineBreak | Token::SafeMode))
//
// Outer decision A && B where:
//   A = !tokens.is_empty()
//   B = !tokens.iter().any(|t| matches!(t, Token::LineBreak | Token::SafeMode))
//
// MC/DC for A&&B:
//   row 1: (A=T, B=T) — non-empty, no blocker -> true
//   row 2: (A=F, B=T) — empty slice            -> false
//           (When A=F the iterator short-circuits in `any`, so B has no
//           meaningful T/F distinction for an empty slice; treated as T by
//           virtue of the vacuous truth of `!any(...)` over an empty list.)
//   row 3: (A=T, B=F) — has LineBreak or SafeMode -> false
//
// Independent effect:
//   Pair (1, 2): A flips T->F, B held T -> outcome flips T->F (A independent).
//   Pair (1, 3): B flips T->F, A held T -> outcome flips T->F (B independent).
//
// Inner disjunction inside the `any` closure is a classifier over Token
// variants, handled separately in submodule `is_quantizable_block_inner_match`.
// ---------------------------------------------------------------------------
mod is_quantizable_block_outer {
    use super::*;

    #[test]
    fn aq_ver_006_b_row1_nonempty_no_blocker_is_quantizable() {
        // (A=T, B=T) -> true
        let tokens = vec![
            Token::VectorStart,
            Token::Number("2".into()),
            Token::VectorEnd,
            Token::Symbol("*".into()),
        ];
        assert!(
            is_quantizable_block(&tokens),
            "non-empty token sequence without LineBreak/SafeMode must quantize"
        );
    }

    #[test]
    fn aq_ver_006_b_row2_empty_is_not_quantizable() {
        // (A=F, B=T) -> false — pairs with row 1 to prove A independent.
        let tokens: Vec<Token> = Vec::new();
        assert!(
            !is_quantizable_block(&tokens),
            "empty token sequence must not quantize"
        );
    }

    #[test]
    fn aq_ver_006_b_row3_nonempty_with_linebreak_is_not_quantizable() {
        // (A=T, B=F) -> false — pairs with row 1 to prove B independent.
        let tokens = vec![
            Token::Number("1".into()),
            Token::LineBreak,
            Token::Number("2".into()),
        ];
        assert!(
            !is_quantizable_block(&tokens),
            "token sequence containing LineBreak must not quantize"
        );
    }
}

// ---------------------------------------------------------------------------
// AQ-VER-006-B (inner)
// DUT: rust/src/interpreter/quantized-block.rs:270 — closure `|t| matches!(t,
// Token::LineBreak | Token::SafeMode)`
//
// Conditions (inside the `any` predicate):
//   A' = (t == Token::LineBreak)
//   B' = (t == Token::SafeMode)
//
// Variants are mutually exclusive so (A'=T, B'=T) is structurally impossible.
//
// MC/DC rows (per-token truth of the inner `matches!`):
//   row 1: A'=T, B'=F (LineBreak)  -> true  -> sequence is not quantizable
//   row 2: A'=F, B'=T (SafeMode)   -> true  -> sequence is not quantizable
//   row 3: A'=F, B'=F (e.g. Number) -> false -> sequence IS quantizable (if non-empty)
//
// Independent effect for the inner disjunction:
//   Pair (1, 3): A' flips T->F, B' held F -> outcome flips T->F (A' independent).
//   Pair (2, 3): B' flips T->F, A' held F -> outcome flips T->F (B' independent).
// ---------------------------------------------------------------------------
mod is_quantizable_block_inner_match {
    use super::*;

    #[test]
    fn aq_ver_006_b_inner_row1_linebreak_blocks_quantization() {
        // A'=T, B'=F
        let tokens = vec![Token::LineBreak];
        assert!(
            !is_quantizable_block(&tokens),
            "LineBreak token alone must block quantization"
        );
    }

    #[test]
    fn aq_ver_006_b_inner_row2_safemode_blocks_quantization() {
        // A'=F, B'=T
        let tokens = vec![Token::SafeMode];
        assert!(
            !is_quantizable_block(&tokens),
            "SafeMode token alone must block quantization"
        );
    }

    #[test]
    fn aq_ver_006_b_inner_row3_number_does_not_block_quantization() {
        // A'=F, B'=F — pairs with rows 1 and 2 to prove each variant independent.
        let tokens = vec![Token::Number("1".into())];
        assert!(
            is_quantizable_block(&tokens),
            "Number token alone must permit quantization"
        );
    }
}

// ---------------------------------------------------------------------------
// AQ-VER-006-C
// DUT: rust/src/interpreter/execute-builtin.rs:327-330 in
// `Interpreter::get_execution_plan_set` (the `quant_valid` closure)
//
//     q.guard_signature.dictionary_epoch == self.dictionary_epoch        // A
//         && q.guard_signature.module_epoch == self.module_epoch          // B
//
// The outer disjunction on line 332 `compiled_valid || quant_valid` admits
// a cache hit. `compiled_valid` is computed by `is_plan_valid`
// (rust/src/interpreter/compiled-plan.rs:42-45) which checks the SAME two
// epoch fields. Because both evaluators observe the same source-of-truth
// (`interp.dictionary_epoch` and `interp.module_epoch`), bumping either
// epoch flips both `compiled_valid` and `quant_valid` in lockstep, and the
// disjunction reduces to the conjunction A && B on the bump path.
//
// MC/DC for A && B:
//   row 1: A=T, B=T — no epoch bumped since last build -> quant_valid=T
//                   -> cache hit (Δmiss=0 over one MAP)
//   row 2: A=F, B=T — bump_dictionary_epoch only       -> quant_valid=F
//                   -> cache miss (Δmiss=+1 on first element; subsequent
//                      elements in the MAP hit the rebuilt plan)
//   row 3: A=T, B=F — bump_module_epoch only           -> quant_valid=F
//                   -> cache miss (Δmiss=+1)
//
// Independent effect:
//   Pair (1, 2) with B held T: A flips T->F -> quant_valid flips T->F.
//   Pair (1, 3) with A held T: B flips T->F -> quant_valid flips T->F.
//
// Observed baseline (recorded 2026-04-24 via a prior probe run):
//   after DEF:                 miss=0 hit=0 build=0
//   after MAP #1 (build):      miss=1 hit=2 build=1   (row 1 pre-state build)
//   after MAP #2 (all hits):   miss=1 hit=5 build=1   (Δmiss=0 : ROW 1)
//   after bump_dict + MAP:     miss=2 hit=7 build=2   (Δmiss=+1: ROW 2)
//   after bump_module + MAP:   miss=3 hit=9 build=3   (Δmiss=+1: ROW 3)
//
// (Hit counts go up by 2 rather than 3 on bump rows because the first
// element of the MAP triggers the miss+rebuild and the two remaining
// elements hit the freshly-stored plan. This is expected behaviour and is
// not part of the MC/DC claim.)
//
// Note: the companion assertion pattern where `compiled_valid=T` but
// `quant_valid=F` (multi-line word where quantization is skipped) is covered
// structurally by `get_execution_plan_set` omitting quant when
// `def.lines.len() != 1`; exercising that path would test a different
// decision (the multi-line short-circuit) rather than this conjunction and
// is out of scope here.
// ---------------------------------------------------------------------------
mod compiled_plan_cache_guard {
    use super::*;

    /// Helper: define DBL, prime the cache, then return the interpreter and
    /// the (miss, hit) snapshot after the priming MAP (row 1 pre-state).
    async fn build_primed_interpreter() -> (Interpreter, u64, u64) {
        let mut interp = Interpreter::new();

        // Define DBL as a single-line, pure, quantizable block.
        // NOTE: execute_reset() clears user_dictionaries so cannot be used
        // between calls here; stack is cleared manually instead.
        interp.execute("{ [2] * } 'DBL' DEF").await.unwrap();
        interp.stack.clear();

        // Priming MAP: builds and stores plan_set under the current epochs.
        interp.execute("[ 1 2 3 ] 'DBL' MAP").await.unwrap();
        interp.stack.clear();

        let m = interp.runtime_metrics();
        (
            interp,
            m.compiled_plan_cache_miss_count,
            m.compiled_plan_cache_hit_count,
        )
    }

    #[tokio::test]
    async fn aq_ver_006_c_row1_no_bump_is_cache_hit() {
        // Row 1: A=T, B=T -> quant_valid=T -> cache hit.
        let (mut interp, miss_before, hit_before) = build_primed_interpreter().await;

        interp.execute("[ 1 2 3 ] 'DBL' MAP").await.unwrap();
        let m = interp.runtime_metrics();

        assert_eq!(
            m.compiled_plan_cache_miss_count - miss_before,
            0,
            "row 1 (A=T, B=T): no epoch bumped, cache must not miss"
        );
        assert_eq!(
            m.compiled_plan_cache_hit_count - hit_before,
            3,
            "row 1 (A=T, B=T): all 3 MAP elements must hit the cache"
        );
    }

    #[tokio::test]
    async fn aq_ver_006_c_row2_dict_bump_causes_cache_miss() {
        // Row 2: A=F (dict mismatch), B=T (module unchanged) -> quant_valid=F.
        // Pair (1, 2) with B held T: A flips T->F, decision flips T->F.
        let (mut interp, miss_before, hit_before) = build_primed_interpreter().await;

        interp.bump_dictionary_epoch();
        interp.execute("[ 1 2 3 ] 'DBL' MAP").await.unwrap();
        let m = interp.runtime_metrics();

        assert_eq!(
            m.compiled_plan_cache_miss_count - miss_before,
            1,
            "row 2 (A=F, B=T): first MAP element must miss due to dict_epoch mismatch"
        );
        assert_eq!(
            m.compiled_plan_cache_hit_count - hit_before,
            2,
            "row 2 (A=F, B=T): remaining 2 MAP elements hit the rebuilt plan"
        );
    }

    #[tokio::test]
    async fn aq_ver_006_c_row3_module_bump_causes_cache_miss() {
        // Row 3: A=T (dict unchanged), B=F (module mismatch) -> quant_valid=F.
        // Pair (1, 3) with A held T: B flips T->F, decision flips T->F.
        let (mut interp, miss_before, hit_before) = build_primed_interpreter().await;

        interp.bump_module_epoch();
        interp.execute("[ 1 2 3 ] 'DBL' MAP").await.unwrap();
        let m = interp.runtime_metrics();

        assert_eq!(
            m.compiled_plan_cache_miss_count - miss_before,
            1,
            "row 3 (A=T, B=F): first MAP element must miss due to module_epoch mismatch"
        );
        assert_eq!(
            m.compiled_plan_cache_hit_count - hit_before,
            2,
            "row 3 (A=T, B=F): remaining 2 MAP elements hit the rebuilt plan"
        );
    }

    #[tokio::test]
    async fn aq_ver_006_c_post_miss_rebuild_recovers_hit_path() {
        // Ancillary coverage: after a miss-triggered rebuild the plan_set is
        // stored with fresh epoch fields, so a subsequent MAP returns to
        // (A=T, B=T) and all 3 elements hit. This proves the rebuild path
        // restores invariants rather than leaving the cache in a degraded
        // state.
        let (mut interp, _miss0, _hit0) = build_primed_interpreter().await;

        interp.bump_dictionary_epoch();
        interp.execute("[ 1 2 3 ] 'DBL' MAP").await.unwrap();
        let after_bump = interp.runtime_metrics();
        interp.stack.clear();

        interp.execute("[ 1 2 3 ] 'DBL' MAP").await.unwrap();
        let after_recovery = interp.runtime_metrics();

        assert_eq!(
            after_recovery.compiled_plan_cache_miss_count
                - after_bump.compiled_plan_cache_miss_count,
            0,
            "post-rebuild MAP must not miss again"
        );
        assert_eq!(
            after_recovery.compiled_plan_cache_hit_count - after_bump.compiled_plan_cache_hit_count,
            3,
            "post-rebuild MAP must hit all 3 elements"
        );
    }
}

// ---------------------------------------------------------------------------
// AQ-VER-006-D
// DUT: rust/src/interpreter/quantized-block.rs `is_quantizable_block`
//      Phase 1-C purity gate (third conjunct).
//
//     !tokens.is_empty()                                          // A
//         && !tokens.iter().any(|t| matches!(t, LineBreak | SafeMode)) // B
//         && !tokens.iter().any(token_is_impure_builtin)          // C
//
// AQ-VER-006-B already pairs row 1 (A=T, B=T, C=T -> true) with row 2
// (A=F -> false) and row 3 (A=T, B=F -> false). This row pairs with that
// shared row 1 to prove the third conjunct C independently flips the
// outcome:
//   row 4: A=T, B=T, C=F (block contains an impure builtin) -> false.
//
// Independent effect:
//   Pair (006-B row 1, 006-D row 4): C flips T->F, A and B held T
//                                    -> outcome flips T->F (C independent).
// ---------------------------------------------------------------------------
mod is_quantizable_block_purity_gate {
    use super::*;

    #[test]
    fn aq_ver_006_d_row4_impure_builtin_blocks_quantization() {
        // (A=T, B=T, C=F) -> false — pairs with AQ-VER-006-B row 1 to
        // prove the impure-builtin gate independent.
        let tokens = vec![Token::Symbol("PRINT".into())];
        assert!(
            !is_quantizable_block(&tokens),
            "block containing an impure builtin must not quantize"
        );
    }

    // ── Inner-predicate truth table for `token_is_impure_builtin` ────────
    //
    //   Symbol-Variant: A''  (token is `Token::Symbol`)
    //   PurityKnown:    B''  (`purity_by_name` returns `Some`)
    //   PurityImpure:   C''  (`info.purity == Purity::Impure`)
    //
    //   row I-1: A''=F                 -> false (Number / Vector marker etc.)
    //   row I-2: A''=T, B''=F          -> false (unknown user word)
    //   row I-3: A''=T, B''=T, C''=F   -> false (pure builtin)
    //   row I-4: A''=T, B''=T, C''=T   -> true  (impure builtin)
    //
    // Pair (I-1, I-3): A'' flips F->T (with B''=T, C''=F) -> outcome held F
    //                 — but the predicate result is still false in both rows,
    //                 so we instead pair (I-4, I-1) for A'' independence:
    // Pair (I-4, I-1): A'' flips T->F (others held: structurally vacuous on
    //                  non-Symbol path) -> outcome flips T->F.
    // Pair (I-4, I-2): B'' flips T->F (A''=T held, C'' vacuous) -> outcome
    //                  flips T->F.
    // Pair (I-4, I-3): C'' flips T->F (A''=T, B''=T held) -> outcome
    //                  flips T->F.

    #[test]
    fn aq_ver_006_d_inner_row1_non_symbol_does_not_block() {
        // A''=F: a Number token cannot be impure → quantization permitted.
        let tokens = vec![Token::Number("1".into())];
        assert!(is_quantizable_block(&tokens));
    }

    #[test]
    fn aq_ver_006_d_inner_row2_unknown_symbol_does_not_block() {
        // A''=T, B''=F: an unknown user word has no purity entry → not Impure
        // for the gate's purposes (deeper purity propagation is the
        // analyzer's job, not the gate's).
        let tokens = vec![Token::Symbol("DROP".into())];
        assert!(is_quantizable_block(&tokens));
    }

    #[test]
    fn aq_ver_006_d_inner_row3_pure_builtin_does_not_block() {
        // A''=T, B''=T, C''=F: ADD is Pure → gate accepts.
        let tokens = vec![Token::Symbol("ADD".into())];
        assert!(is_quantizable_block(&tokens));
    }

    #[test]
    fn aq_ver_006_d_inner_row4_impure_builtin_blocks() {
        // A''=T, B''=T, C''=T: PRINT is Impure → gate rejects.
        let tokens = vec![Token::Symbol("PRINT".into())];
        assert!(!is_quantizable_block(&tokens));
    }
}
