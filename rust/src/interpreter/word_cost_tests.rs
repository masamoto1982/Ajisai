//! Tests for inferred static cost bounds (Phase 5; `word_cost.rs`). The
//! inference-level tests mirror `word_space_tests.rs`'s style — infer a
//! word's contract and assert both the class *and* the exactness witness,
//! since it is the witness that licenses the declaration checker to raise an
//! `error` rather than a `note` (`docs/dev/cost-contract-design.md` §3). The
//! `join` proptest checks the algebraic property the whole inference walk
//! relies on directly, independent of any interpreter.

use crate::interpreter::word_cost::{CostBound, CostClass};
use crate::interpreter::Interpreter;

async fn cost_of(src: &str, name: &str) -> CostBound {
    let mut interp = Interpreter::new();
    interp.execute(src).await.unwrap();
    interp.infer_word_contract(name).expect("contract").cost
}

#[tokio::test]
async fn input_driven_arithmetic_is_exactly_linear_in_numeric_work() {
    // A bare `ADD` takes its operands from the word's inputs, so the meter's
    // limb-width charge really does grow with input: linear, and provably
    // attained. It never loops internally (const steps) and touches no
    // collection — measured, `[ 0 400 ] RANGE 2 MUL` charges 401 numericWork
    // and zero collectionWork beyond the RANGE itself.
    let cost = cost_of("[ ADD ] 'A' DEF", "A").await;
    assert_eq!(cost.steps, (CostClass::Const, true));
    assert_eq!(cost.numeric, (CostClass::Linear, true));
    assert_eq!(cost.collection, (CostClass::Const, true));
}

#[tokio::test]
async fn literal_operands_refine_a_linear_charge_to_const() {
    // Regression: `{ 1 2 ADD }` consumes nothing and charges a *fixed*
    // constant (measured: numericWork 1). Taking ADD's `linear` class
    // un-refined while keeping its exactness witness made the true
    // declaration `cost numeric=const` a hard `error` — a false error, the
    // one thing this model must never produce. The literal operands pin it.
    let cost = cost_of("[ 1 2 ADD ] 'S' DEF", "S").await;
    assert_eq!(cost.numeric, (CostClass::Const, true));
    assert_eq!(cost.steps, (CostClass::Const, true));
}

#[tokio::test]
async fn value_driven_materializer_is_unbounded_over_input_size() {
    // Regression: RANGE's collectionWork is set by its operand's *value*, not
    // its size — measured, `[ 0 10 ] RANGE` charges 187 and `[ 0 20000 ]
    // RANGE` charges 340,017 against the very same 2-element operand. Calling
    // that `linear` let a false declaration verify. `word_space` classifies
    // this pair the same way for the same reason.
    let cost = cost_of("[ RANGE ] 'MK' DEF", "MK").await;
    assert_eq!(cost.collection, (CostClass::Unbounded, true));

    let fill = cost_of("[ FILL ] 'F' DEF", "F").await;
    assert_eq!(fill.collection, (CostClass::Unbounded, true));
}

#[tokio::test]
async fn a_literal_operand_pins_even_a_value_driven_materializer() {
    // The two fixes have to land together: making RANGE `Unbounded` without
    // the refinement would turn the *true* declaration `cost collection=const`
    // on a literal-driven range into a false error instead.
    let cost = cost_of("[ [ 0 10 ] RANGE ] 'K' DEF", "K").await;
    assert_eq!(cost.collection, (CostClass::Const, true));
}

#[tokio::test]
async fn higher_order_word_is_unbounded_on_every_axis_but_not_exact() {
    // MAP runs a caller-supplied body a data-dependent number of times: the
    // sound upper bound is unbounded on all three axes, but never a proven
    // witness (the body could be trivial) — a note, never a false error.
    let cost = cost_of("[ [ 1 ] MAP ] 'M' DEF", "M").await;
    for axis in [cost.steps, cost.numeric, cost.collection] {
        assert_eq!(axis, (CostClass::Unbounded, false));
    }
}

#[tokio::test]
async fn collection_word_is_linear_collection_but_const_numeric_and_steps() {
    // `SORT` scales collectionWork with element count (linear, exact) but
    // charges no numericWork at all — the two axes are genuinely independent.
    // The numeric/steps *class* is what SORT's own classification proves;
    // their `exact` bit is not asserted here because it is inherited from
    // `CostSim`'s `IDENTITY` seed tying at `Const` (`CostBound::join`'s
    // documented "exact is the OR of the two" rule at an equal class,
    // mirroring `SpaceBound::join`), not a claim about SORT specifically.
    let cost = cost_of("[ SORT ] 'S2' DEF", "S2").await;
    assert_eq!(cost.collection, (CostClass::Linear, true));
    assert_eq!(cost.numeric.0, CostClass::Const);
    assert_eq!(cost.steps.0, CostClass::Const);
}

#[tokio::test]
async fn user_word_chain_composes_a_dependency_bound() {
    // A user word wrapping SORT inherits its cost; a word calling that one
    // stays at the same bound (the dependency's bound composes unchanged —
    // Phase 5 does not refine per call site, unlike `word_space`).
    let cost = cost_of("[ SORT ] 'BASE' DEF [ BASE ] 'WRAP' DEF", "WRAP").await;
    assert_eq!(cost.collection, (CostClass::Linear, true));
    assert_eq!(cost.numeric.0, CostClass::Const);
}

// `recursion_is_conservative_unbounded_on_every_axis` tested that a
// self-recursive word costed Unbounded on every axis. SPEC §8.7's DEF-time
// acyclicity check now refuses `[ REC ] 'REC' DEF` outright, so there is no
// longer a recursive word for cost inference to see.

#[tokio::test]
async fn unresolved_dependency_degrades_to_conservative_cost() {
    let cost = cost_of("[ [ 0 10 ] DUP RANGE ] 'U' DEF", "U").await;
    for axis in [cost.steps, cost.numeric, cost.collection] {
        assert_eq!(axis, (CostClass::Unbounded, false));
    }
}

mod join_proptests {
    use crate::interpreter::word_cost::{CostBound, CostClass};
    use proptest::prelude::*;

    fn cost_class() -> impl Strategy<Value = CostClass> {
        prop_oneof![
            Just(CostClass::Const),
            Just(CostClass::Linear),
            Just(CostClass::Superlinear),
            Just(CostClass::Unbounded),
        ]
    }

    fn axis() -> impl Strategy<Value = (CostClass, bool)> {
        (cost_class(), any::<bool>())
    }

    fn bound() -> impl Strategy<Value = CostBound> {
        (axis(), axis(), axis()).prop_map(|(steps, numeric, collection)| CostBound {
            steps,
            numeric,
            collection,
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        // `join` is a sound upper-bound accumulator: the class on every axis
        // can only widen (or stay put), never narrow, regardless of operand
        // order — the property the whole `CostSim` walk relies on to stay a
        // *sound* bound as it folds a word body left to right.
        #[test]
        fn join_never_narrows_a_class(a in bound(), b in bound()) {
            let mut joined = a;
            joined.join(b);
            prop_assert!(joined.steps.0 >= a.steps.0 && joined.steps.0 >= b.steps.0);
            prop_assert!(joined.numeric.0 >= a.numeric.0 && joined.numeric.0 >= b.numeric.0);
            prop_assert!(
                joined.collection.0 >= a.collection.0 && joined.collection.0 >= b.collection.0
            );
        }

        // `IDENTITY` is the bottom of the lattice (`Const`, exact) — the seed
        // `CostSim::new()` starts from. Joining it in can never raise a
        // class (it is never above any operand's own class) and can only
        // ever *add* an exactness witness at a tied `Const` class, never
        // remove one — it is a no-op only up to that one-directional slack.
        #[test]
        fn identity_join_never_raises_class_or_loses_exactness(a in bound()) {
            let mut joined = a;
            joined.join(CostBound::IDENTITY);
            prop_assert_eq!(joined.steps.0, a.steps.0);
            prop_assert_eq!(joined.numeric.0, a.numeric.0);
            prop_assert_eq!(joined.collection.0, a.collection.0);
            prop_assert!(joined.steps.1 >= a.steps.1);
            prop_assert!(joined.numeric.1 >= a.numeric.1);
            prop_assert!(joined.collection.1 >= a.collection.1);
        }

        // `join` is commutative: the fold order of a word body's dependencies
        // must not matter to the bound it produces.
        #[test]
        fn join_is_commutative(a in bound(), b in bound()) {
            let mut ab = a;
            ab.join(b);
            let mut ba = b;
            ba.join(a);
            prop_assert_eq!(ab, ba);
        }
    }
}

#[test]
fn declared_class_order_is_the_lattice_order() {
    // `CostClass` is generated from spec/words.schema.json's `class` enum and
    // derives `Ord` from that declaration order, which is what `CostBound::join`
    // widens along. Reordering the schema enum would silently reorder the
    // lattice — a change no type error would catch — so the chain is asserted
    // here rather than left implicit in the generated file.
    assert!(CostClass::Const < CostClass::Linear);
    assert!(CostClass::Linear < CostClass::Superlinear);
    assert!(CostClass::Superlinear < CostClass::Unbounded);
}

#[test]
fn every_word_publishes_a_cost_the_inference_reads_back() {
    // The published registry is the only place the classes are written down,
    // so an entry the inference cannot read back is a Word whose declared and
    // inferred cost could disagree.
    use crate::interpreter::word_cost::builtin_cost;
    use crate::kernel::generated::GENERATED_WORDS;
    for word in GENERATED_WORDS {
        let inferred = builtin_cost(word.id);
        assert_eq!(
            inferred.steps,
            (word.cost.steps.class, word.cost.steps.exact),
            "{} steps",
            word.name
        );
        assert_eq!(
            inferred.numeric,
            (word.cost.numeric.class, word.cost.numeric.exact),
            "{} numeric",
            word.name
        );
        assert_eq!(
            inferred.collection,
            (word.cost.collection.class, word.cost.collection.exact),
            "{} collection",
            word.name
        );
    }
}
