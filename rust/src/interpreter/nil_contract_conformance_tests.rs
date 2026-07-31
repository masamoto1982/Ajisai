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
//! An audit of all 69 Words against `spec/words.json` found six Words whose
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
const KNOWN_DIVERGENCES: &[(&str, Outcome, &str)] = &[
    // Declared `rejectNil`; returns 0 for the length of a NIL. Decision: error.
    ("LENGTH", Outcome::Value, "returns 0 instead of rejecting"),
    // Declared `rejectNil`; absorbs the NIL as an ordinary element instead
    // (`[ 1 ] NIL CONCAT` -> `[ 1 NIL ]`), so a bubble is silently buried
    // inside a collection. Found by this gate, not by manual review.
    // Decision: error.
    ("CONCAT", Outcome::Value, "absorbs NIL as a vector element"),
    // Declared `rejectNil`; yields a reason-less NIL. Decision: error.
    ("EXEC", Outcome::NilWithoutReason, "swallows the NIL reason"),
    // Declared `passthrough`; raises an error instead. Decision: pass through.
    (
        "SORT",
        Outcome::Error,
        "rejects a NIL it should pass through",
    ),
    // Declared `passthrough`; yields a NIL whose reason was destroyed, so the
    // bubble stops being diagnosable. Decision: pass through, preserving reason.
    ("STR", Outcome::NilWithoutReason, "destroys the NIL reason"),
];

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

/// The baseline is a ratchet: it may only shrink. Pinning the count makes
/// re-adding a divergence a deliberate, reviewable edit rather than a quiet one.
#[test]
fn divergence_baseline_does_not_grow() {
    assert!(
        KNOWN_DIVERGENCES.len() <= 5,
        "KNOWN_DIVERGENCES grew to {}; a new Word may not diverge from its declared contract",
        KNOWN_DIVERGENCES.len()
    );
}

/// `rejectNil` is declared once per Word, so it binds **every** operand
/// position. The blanket probe above fills all operands with NIL, which for the
/// search Words trips the vector-operand rejection first and hides the needle
/// position — so that position gets its own probe.
///
/// `CONTAINS` is documented as "true if a vector contains an element **equal
/// to** the given value", yet a NIL needle currently yields `FALSE`, asserting
/// that nothing equals NIL. `EQ` disagrees: `NIL EQ NIL` is NIL, not TRUE, so
/// the aggregate answer is *unknown*, not false. `INDEX-OF` is the same
/// operation and answers `0` for the same input.
///
/// Decision: `rejectNil` applies to the needle too, so both must error.
#[test]
fn search_words_reject_a_nil_needle() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime");

    // Recorded divergence, not endorsement: both should be `Outcome::Error`.
    for (program, current) in [
        ("[ 1 ] 1 0 DIV CONTAINS", Outcome::Value),
        ("[ 1 ] 1 0 DIV INDEX-OF", Outcome::NilWithReason),
    ] {
        assert_eq!(
            runtime.block_on(observe(program)),
            current,
            "`{program}` changed behavior; if it now errors, the divergence is fixed — \
             delete this case"
        );
    }
}
