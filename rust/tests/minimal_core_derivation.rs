//! Minimal Core derivability witness (SPECIFICATION.html §2.6).
//!
//! §2.6 declares the *Ajisai Minimal Core* — the `identity` and `flow`
//! `core_tier`s of `docs/formalization-coverage.json` — and states that the
//! `material` tier is derivable library that stays bound by the Minimal Core's
//! propagation disciplines. This file is an executable *witness* of that
//! derivability claim for one material word: it re-implements the `material`
//! word `MATH@SIGN` (tier `material`, derived from `algebra.exact-real.budgeted-order`
//! and `algebra.k3.domain`) as the user word `SIGN2`, written using **only**
//! Minimal Core words, and asserts the two agree over `MATH@SIGN`'s domain.
//!
//! `SIGN2` uses exactly these words, all Minimal Core:
//!   - `NIL?`   — identity tier (Bubble/NIL observation, §7.15)
//!   - `LT` `GT`— identity tier (budgeted comparison producing a TruthValue, §7.4)
//!   - `COND` `IDLE` — flow tier (state-transformer composition/identity, §7.7)
//!   - `NIL` and numeric literals — identity / sugar
//! No `material`-tier word (no arithmetic, no vector word, no module word)
//! appears in the definition, so a green run witnesses that `MATH@SIGN`'s
//! observable contract is reconstructible from the Minimal Core alone.
//!
//! Scope. The equivalence is asserted over `MATH@SIGN`'s actual domain: the
//! admitted *rational* values (§4.2.7) and NIL. It is deliberately not asserted
//! over lazy irrationals, because the two diverge there — and that divergence is
//! itself a finding this witness surfaced, recorded in
//! `minimal_core_sign_extends_builtin_note` below: the derived `SIGN2` correctly
//! signs `2 SQRT` (→ `1`) via Core comparison's admitted-domain totality (§7.4),
//! whereas the built-in `MATH@SIGN` rejects the lazy operand with
//! `SIGN: expected a number`. The Minimal Core derivation is thus a *total
//! extension* of the current built-in, in the same way the Python port surfaced
//! specification gaps — the derivation acts as an oracle for the material word.

use ajisai_core::interpreter::Interpreter;
use proptest::prelude::*;

/// Run an Ajisai program and render the whole final stack, the same observation
/// the conformance runner and `algebraic_laws.rs` use. Panics on execution error.
fn eval(src: &str) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        interp
            .execute(src)
            .await
            .unwrap_or_else(|e| panic!("program failed: {src:?}: {e}"));
        interp
            .get_stack()
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// The Minimal-Core-only definition of `SIGN2`. Multi-line body: each newline is
/// a statement separator (§3.4), so the `|`-style COND clauses are one per line
/// (§7.7.1).
const SIGN2_DEF: &str = "{
{ NIL? | NIL }
{ 0 LT | -1 }
{ 0 GT | 1 }
{ IDLE | 0 }
COND
} 'SIGN2' DEF
";

/// Derived side: define `SIGN2` from the Minimal Core, then apply it to `x`.
fn derived(x: &str) -> String {
    eval(&format!("{SIGN2_DEF}{x} SIGN2"))
}

/// Built-in side: the `material` word under witness.
fn builtin(x: &str) -> String {
    eval(&format!("'math' IMPORT {x} MATH@SIGN"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Over the rational domain, the Minimal-Core `SIGN2` reproduces `MATH@SIGN`
    /// exactly. `num`/`den` covers integers (den=1), proper fractions, both
    /// signs, and zero (num=0). The input value is *constructed* with `/`
    /// (division builds the operand); `SIGN2` itself uses no arithmetic.
    #[test]
    fn minimal_core_sign_matches_builtin_over_rationals(
        num in -50i64..=50,
        den in 1i64..=50,
    ) {
        let x = format!("{num} {den} /");
        let d = derived(&x);
        let b = builtin(&x);
        prop_assert_eq!(
            &d, &b,
            "SIGN2 (Minimal Core) and MATH@SIGN disagree on {}/{}: derived={}, builtin={}",
            num, den, d, b
        );
    }
}

/// NIL propagation: both the derived word and the built-in pass a NIL operand
/// through to NIL. `SIGN2` inherits this from the `NIL?` guard plus COND's
/// non-firing on the remaining guards, matching `MATH@SIGN`'s NIL-passthrough.
#[test]
fn minimal_core_sign_matches_builtin_on_nil() {
    let x = "1 0 /"; // divisionByZero → NIL
    assert_eq!(
        derived(x),
        builtin(x),
        "SIGN2 and MATH@SIGN disagree on NIL"
    );
    assert_eq!(derived(x), "NIL");
}

/// Decided-value spot checks, independent of the generator, pinning the three
/// signs on both rational integers and rational fractions.
#[test]
fn minimal_core_sign_decided_spot_checks() {
    for (x, want) in [
        ("7", "1/1"),
        ("-7", "-1/1"),
        ("0", "0/1"),
        ("3 4 /", "1/1"),
        ("-3 4 /", "-1/1"),
        ("0 4 /", "0/1"),
    ] {
        assert_eq!(derived(x), want, "SIGN2({x})");
        assert_eq!(builtin(x), want, "MATH@SIGN({x})");
    }
}

/// Oracle finding (documented, not a divergence in the witness above). The
/// Minimal-Core `SIGN2` is a *total extension* of the current built-in: it signs
/// the lazy irrational `2 SQRT` as `1` through Core comparison's admitted-domain
/// totality (§7.4), while the built-in `MATH@SIGN` rejects the same operand.
/// This test pins the current, divergent behavior so that a later fix to
/// `MATH@SIGN` (making it accept the full numeric domain like `MATH@MIN`/`MAX`)
/// will flip the built-in branch and flag this note for update.
#[test]
fn minimal_core_sign_extends_builtin_note() {
    // Derived side handles the lazy irrational and reports the correct sign.
    // `2 SQRT` needs the math module in scope to construct the operand; `SIGN2`
    // itself still uses only Minimal Core words.
    let derived_sqrt = eval(&format!("'math' IMPORT {SIGN2_DEF}2 SQRT SIGN2"));
    assert_eq!(derived_sqrt, "1/1", "SIGN2 should sign 2 SQRT as +1");

    // Built-in side currently errors on the same input. Encode that as an
    // observed error rather than a value, so the note is self-verifying.
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    let builtin_errs = rt.block_on(async {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 2 SQRT MATH@SIGN")
            .await
            .is_err()
    });
    assert!(
        builtin_errs,
        "MATH@SIGN now accepts 2 SQRT; the Minimal Core derivation is no longer a \
         strict extension — update §2.6 witness note in this file"
    );
}
