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
async fn const_arithmetic_word_is_const_steps_and_exact_linear_numeric() {
    // `ADD` never loops internally (const, exact steps) and is the meter's
    // own reason to exist on the numeric axis (linear, exact numeric); it
    // touches no collection (const, exact collection).
    let cost = cost_of("{ 1 2 ADD } 'S' DEF", "S").await;
    assert_eq!(cost.steps, (CostClass::Const, true));
    assert_eq!(cost.numeric, (CostClass::Linear, true));
    assert_eq!(cost.collection, (CostClass::Const, true));
}

#[tokio::test]
async fn higher_order_word_is_unbounded_on_every_axis_but_not_exact() {
    // MAP runs a caller-supplied body a data-dependent number of times: the
    // sound upper bound is unbounded on all three axes, but never a proven
    // witness (the body could be trivial) — a note, never a false error.
    let cost = cost_of("{ [ 1 ] MAP } 'M' DEF", "M").await;
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
    let cost = cost_of("{ SORT } 'S2' DEF", "S2").await;
    assert_eq!(cost.collection, (CostClass::Linear, true));
    assert_eq!(cost.numeric.0, CostClass::Const);
    assert_eq!(cost.steps.0, CostClass::Const);
}

#[tokio::test]
async fn user_word_chain_composes_a_dependency_bound() {
    // A user word wrapping SORT inherits its cost; a word calling that one
    // stays at the same bound (the dependency's bound composes unchanged —
    // Phase 5 does not refine per call site, unlike `word_space`).
    let cost = cost_of("{ SORT } 'BASE' DEF { BASE } 'WRAP' DEF", "WRAP").await;
    assert_eq!(cost.collection, (CostClass::Linear, true));
    assert_eq!(cost.numeric.0, CostClass::Const);
}

#[tokio::test]
async fn recursion_is_conservative_unbounded_on_every_axis() {
    let cost = cost_of("{ REC } 'REC' DEF", "REC").await;
    for axis in [cost.steps, cost.numeric, cost.collection] {
        assert_eq!(axis, (CostClass::Unbounded, false));
    }
}

#[tokio::test]
async fn unresolved_dependency_degrades_to_conservative_cost() {
    let cost = cost_of("{ [ 0 10 ] DUP RANGE } 'U' DEF", "U").await;
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
