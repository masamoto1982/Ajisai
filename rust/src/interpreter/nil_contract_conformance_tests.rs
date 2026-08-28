//! Specification-driven NIL contract conformance.
//!
//! # Why this suite exists
//!
//! `nil_conformance_tests` derives what it checks from the *implementation's
//! own* `nil_policy` label: it enumerates the registry, selects the Words the
//! implementation calls `Passthrough`, and probes those. That coupling has a
//! blind spot which is not incidental but structural — **the label decides what
//! gets tested, so a wrong label removes the Word from its own test.** A Word
//! mislabelled `RejectsNil` is never probed for passthrough, and a Word
//! mislabelled `Passthrough` in an uncovered category is never probed at all.
//!
//! An audit of every Word against `spec/words.json` found six Words whose
//! runtime behavior contradicts the policy *both* sources declare, every one of
//! them invisible to the existing suite for exactly that reason.
//!
//! This suite closes the loop by taking its obligations from the **canonical**
//! contract in the generated registry (projected from `spec/words.json`) rather
//! than from any hand-written label. Every fixed-arity Word whose declared
//! policy makes a NIL operand observable is probed, and the outcome is compared
//! against what the specification says must happen.
//!
//! # The divergence baseline
//!
//! `KNOWN_DIVERGENCES` records the Words whose runtime does not yet honor the
//! declared policy, mirroring the `docs/quality/file-size-baseline.json`
//! pattern already used in this repository: the list is a ratchet, not a
//! permission. A Word not on the list must conform, so no *new* divergence can
//! be introduced; entries are removed as the executors are corrected. The test
//! also fails if a listed Word starts conforming, so the list cannot go stale.
//!
//! **The list is now empty.** Every audited Word honors its declared policy,
//! because the dispatch guard decides both directions from the declaration
//! instead of leaving each executor to decide for itself. The machinery stays
//! so that a regression has to be admitted explicitly rather than committed
//! quietly.

use crate::interpreter::Interpreter;
use crate::kernel::generated::{Arity, NilPolicy, GENERATED_WORDS};

/// What a NIL operand produced, observed through the public outcome only.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Outcome {
    /// Evaluation raised a channel error (malformed use).
    Error,
    /// A NIL reached the stack top carrying a reason.
    NilWithReason,
    /// A NIL reached the stack top with its reason lost.
    NilWithoutReason,
    /// An ordinary value reached the stack top.
    Value,
}

/// Words whose runtime does not yet honor the policy `spec/words.json`
/// declares. Each entry records the declared policy, the observed behavior, and
/// the decision taken. Remove an entry when its executor is corrected.
/// Empty, and it stays that way: the dispatch guard in `execute_builtin` now
/// settles both directions of the contract from the declaration — `rejectNil`
/// refuses to run, `passthrough` yields the bubble — so no executor is left to
/// disagree with the canon. The last two entries were `SORT` (raised an error
/// on a NIL it declares it passes through) and `STR` (passed the NIL but
/// destroyed its reason, making the bubble undiagnosable). Both conform now.
const KNOWN_DIVERGENCES: &[(&str, Outcome, &str)] = &[];

fn divergence(name: &str) -> Option<Outcome> {
    KNOWN_DIVERGENCES
        .iter()
        .find(|(word, _, _)| *word == name)
        .map(|(_, outcome, _)| *outcome)
}

async fn observe(program: &str) -> Outcome {
    let mut interp = Interpreter::new();
    if interp.execute(program).await.is_err() {
        return Outcome::Error;
    }
    match interp.get_stack().last() {
        Some(value) if value.is_nil() => {
            if value.nil_reason().is_some() {
                Outcome::NilWithReason
            } else {
                Outcome::NilWithoutReason
            }
        }
        _ => Outcome::Value,
    }
}

/// The outcome the specification's `nilPolicy` requires for a NIL operand.
///
/// Only the policies that constrain a NIL *input* are probed. `createsNil`,
/// `consumeNil` and `inspectNil` describe what the Word does with non-NIL
/// operands or how it inspects NIL-ness, so they place no obligation here.
fn required(policy: NilPolicy) -> Option<Outcome> {
    match policy {
        // A bubble flows downstream and stays diagnosable, so the reason must
        // survive (SPEC §7.12).
        NilPolicy::Passthrough | NilPolicy::PassthroughThenProject | NilPolicy::PreserveReason => {
            Some(Outcome::NilWithReason)
        }
        NilPolicy::RejectNil => Some(Outcome::Error),
        NilPolicy::CreatesNil | NilPolicy::ConsumeNil | NilPolicy::InspectNil => None,
        // `kleeneAbsorbing` (strong-Kleene `AND`/`OR`) only decides to a
        // definite value when a dominating operand (FALSE/TRUE) is present;
        // this blanket probe fills every operand position with NIL, so no
        // dominating operand exists and the result must still be a reasoned
        // NIL standing for UNKNOWN (LANG.VALUES.TRUTH).
        NilPolicy::KleeneAbsorbing => Some(Outcome::NilWithReason),
    }
}

/// A program that puts `arity` NIL operands on the stack and applies `word`.
/// `1 0 DIV` is the canonical reasoned bubble (`divisionByZero`).
fn probe_program(word: &str, arity: u8) -> String {
    let nils = vec!["1 0 DIV"; arity as usize].join(" ");
    format!("{nils} {word}")
}

#[test]
fn declared_nil_policy_is_honored_at_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime");

    let mut violations: Vec<String> = Vec::new();
    let mut stale: Vec<String> = Vec::new();
    let mut probed = 0_usize;

    for word in GENERATED_WORDS {
        let Some(want) = required(word.nil_policy) else {
            continue;
        };
        // Data-dependent arity has no fixed operand count to fill with NIL.
        let (Arity::Fixed(arity), true) = (word.stack_inputs, word.stack_inputs != Arity::Fixed(0))
        else {
            continue;
        };
        // `PRINT` writes and leaves nothing observable on the stack.
        if word.name == "PRINT" {
            continue;
        }

        probed += 1;
        let got = runtime.block_on(observe(&probe_program(word.name, arity)));

        match divergence(word.name) {
            Some(recorded) if got == recorded => {}
            Some(recorded) => stale.push(format!(
                "{}: baseline records {recorded:?} but observed {got:?} — update or remove the \
                 KNOWN_DIVERGENCES entry",
                word.name
            )),
            None if got != want => violations.push(format!(
                "{}: spec/words.json declares `{}`, which requires {want:?}, but observed {got:?}",
                word.name,
                word.nil_policy.as_spec_str()
            )),
            None => {}
        }
    }

    assert!(probed >= 30, "probe set collapsed to {probed} Words");
    assert!(
        stale.is_empty(),
        "KNOWN_DIVERGENCES is stale:\n  {}",
        stale.join("\n  ")
    );
    assert!(
        violations.is_empty(),
        "{} Word(s) do not honor their declared NIL policy:\n  {}",
        violations.len(),
        violations.join("\n  ")
    );
}

/// The baseline is a ratchet: it may only shrink. Now that it is empty the
/// ratchet is at its floor — every Word honors its declared contract, and
/// re-admitting a divergence is a deliberate, reviewable edit rather than a
/// quiet one.
#[test]
#[allow(clippy::const_is_empty)] // Intentional source-level ratchet: adding an allowlist entry must fail CI.
fn divergence_baseline_does_not_grow() {
    assert!(
        KNOWN_DIVERGENCES.is_empty(),
        "KNOWN_DIVERGENCES grew to {}; a Word may not diverge from its declared contract",
        KNOWN_DIVERGENCES.len()
    );
}

/// `rejectNil` is declared once per Word, so it binds **every** operand
/// position, not just the receiver. The blanket probe above fills all operands
/// with NIL, which for the search Words trips the vector-operand rejection
/// first and hides the needle position — so that position gets its own probe.
///
/// `CONTAINS` is documented as "true if a vector contains an element **equal
/// to** the given value", yet answered `FALSE` for a NIL needle, asserting that
/// nothing equals NIL. `EQ` disagrees: `NIL EQ NIL` is NIL, not TRUE, so the
/// aggregate answer is *unknown*, not false — the Word contradicted the
/// operation it is defined in terms of. It now rejects, per its declaration.
///
/// `INDEX-OF` declares `createsNil`, not `rejectNil`, so the guard leaves it
/// alone and it still answers a reasoned NIL for an absent needle. Pinned here
/// so the difference between the two declarations stays visible.
#[test]
fn search_words_reject_a_nil_needle() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime");

    for (program, current) in [
        ("[ 1 ] 1 0 DIV CONTAINS", Outcome::Error),
        ("[ 1 ] 1 0 DIV INDEX-OF", Outcome::NilWithReason),
    ] {
        assert_eq!(
            runtime.block_on(observe(program)),
            current,
            "`{program}` changed behavior; update this case deliberately"
        );
    }
}

/// Rejection is safe by construction — it runs nothing and touches no stack —
/// but passing a bubble through has to *produce* the result and unwind the
/// operands itself, so the guard takes over a duty the executors used to
/// discharge: honoring the consumption mode (SPEC §5.2). `EAT` replaces the
/// declared operand window with the bubble; `KEEP` leaves the window in place
/// and stacks the bubble above it. Both are pinned for a unary and a binary
/// Word, together with the depth of the stack the guard leaves behind.
#[test]
fn passthrough_unwinds_the_operand_window_under_both_consumption_modes() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime");

    for (program, depth) in [
        // Unary: SORT eats its vector, so only the bubble is left.
        ("1 0 DIV SORT", 1),
        ("1 0 DIV KEEP SORT", 2),
        // Binary: ADD eats both operands. Under KEEP both survive.
        ("1 0 DIV 1 ADD", 1),
        ("1 0 DIV 1 KEEP ADD", 3),
        // The bubble need not be the receiver: any operand position carries it.
        ("1 1 0 DIV ADD", 1),
    ] {
        let (observed, left) = runtime.block_on(observe_depth(program));
        assert_eq!(
            observed,
            Outcome::NilWithReason,
            "`{program}` must yield the bubble with its reason intact"
        );
        assert_eq!(
            left, depth,
            "`{program}` left {left} value(s) on the stack, expected {depth}"
        );
    }
}

/// A Word wrapped in a user-defined Word runs through the compiled plan rather
/// than the interpreter's dispatch, and that second path skipped the guard
/// entirely: `[ LENGTH ] 'LEN' DEF 1 0 DIV LEN` answered `0` for the length of
/// a NIL while `1 0 DIV LENGTH` rejected it, and SORT and STR likewise reverted
/// to their pre-guard behavior one call deep.
///
/// Compiling a body is required to be unobservable (LANG.AUTHORITY.FREEDOM), so
/// a declaration enforced on one path and not the other is not a smaller bug
/// than no enforcement at all — it is the same Word with two behaviors. Each
/// case below runs the identical Word directly and through a wrapper, and
/// requires the same outcome from both.
#[test]
fn the_declared_contract_binds_the_compiled_path_too() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime");

    for (word, want) in [
        // rejectNil
        ("LENGTH", Outcome::Error),
        ("CONCAT", Outcome::Error),
        // passthrough
        ("SORT", Outcome::NilWithReason),
        ("STR", Outcome::NilWithReason),
    ] {
        let direct = format!("1 0 DIV {word}");
        let wrapped = format!("[ {word} ] 'WRAP' DEF 1 0 DIV WRAP");

        let direct_outcome = runtime.block_on(observe(&direct));
        let wrapped_outcome = runtime.block_on(observe(&wrapped));

        assert_eq!(
            direct_outcome, want,
            "`{direct}` must honor {word}'s declared contract"
        );
        assert_eq!(
            wrapped_outcome, want,
            "`{wrapped}` must honor {word}'s declared contract on the compiled path too"
        );
    }
}

/// `observe`, plus the depth of the stack the program left behind.
async fn observe_depth(program: &str) -> (Outcome, usize) {
    let mut interp = Interpreter::new();
    if interp.execute(program).await.is_err() {
        return (Outcome::Error, 0);
    }
    let depth = interp.get_stack().len();
    let outcome = match interp.get_stack().last() {
        Some(value) if value.is_nil() => {
            if value.nil_reason().is_some() {
                Outcome::NilWithReason
            } else {
                Outcome::NilWithoutReason
            }
        }
        _ => Outcome::Value,
    };
    (outcome, depth)
}
