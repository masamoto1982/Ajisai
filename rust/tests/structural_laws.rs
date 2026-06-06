//! Phase 5 — structural data (vector / tensor) algebraic laws (executable).
//!
//! Encodes `docs/dev/ajisai-formalization-expansion-roadmap.md` Phase 5: the
//! vector vocabulary of SPEC §7.1 is a free monoid under `CONCAT` with an
//! involutive `REVERSE`, and the tensor vocabulary of §7.2 is the reshape group
//! acting on `Tensor ≅ (data: V*, shape)` — `TRANSPOSE` is an involution on 2-D
//! tensors, `RESHAPE` round-trips, and `SHAPE`/`RANK` read off the index
//! structure. `SORT` (canonical home `ALGO`, §9.1) is idempotent and
//! permutation-invariant on the decidable rational sub-domain (§7.4.3).
//!
//! Observation matches the conformance runner: whole-stack `Value::to_string`.

use ajisai_core::interpreter::Interpreter;
use proptest::prelude::*;

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

fn assert_law(name: &str, lhs: &str, rhs: &str) {
    let l = eval(lhs);
    let r = eval(rhs);
    assert_eq!(
        l, r,
        "law `{name}` broken:\n  {lhs:?} => {l}\n  {rhs:?} => {r}"
    );
}

fn small() -> impl Strategy<Value = i64> {
    -50i64..=50
}
fn vlit(xs: &[i64]) -> String {
    let body = xs
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    format!("[ {body} ]")
}
fn vec_ne() -> impl Strategy<Value = Vec<i64>> {
    prop::collection::vec(small(), 1..=6)
}
fn triple() -> impl Strategy<Value = (i64, i64, i64)> {
    (small(), small(), small())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// `REVERSE` is an involution: `REVERSE ∘ REVERSE = id`.
    #[test]
    fn reverse_is_involution(xs in vec_ne()) {
        let v = vlit(&xs);
        assert_law("reverse-involution", &format!("{v} REVERSE REVERSE"), &v);
    }

    /// `TAKE n` of the whole length is the identity.
    #[test]
    fn take_full_is_identity(xs in vec_ne()) {
        let v = vlit(&xs);
        let n = xs.len();
        assert_law("take-full", &format!("{v} {n} TAKE"), &v);
    }

    /// Identity `REORDER` is the identity; reversed indices equal `REVERSE`.
    #[test]
    fn reorder_identity_and_reverse((a, b, c) in triple()) {
        let v = format!("[ {a} {b} {c} ]");
        assert_law("reorder-id", &format!("{v} [ 0 1 2 ] REORDER"), &v);
        assert_law(
            "reorder-reverse",
            &format!("{v} [ 2 1 0 ] REORDER"),
            &format!("{v} REVERSE"),
        );
    }
}

// ── Free-monoid laws of CONCAT / REVERSE (fixed operands) ──

#[test]
fn concat_is_associative() {
    // (a ++ b) ++ c == a ++ (b ++ c) == concat[a, b, c].
    assert_law(
        "concat-assoc",
        "[ 1 2 ] [ 3 4 ] 2 CONCAT [ 5 6 ] 2 CONCAT",
        "[ 1 2 ] [ 3 4 ] [ 5 6 ] 3 CONCAT",
    );
}

#[test]
fn reverse_is_anti_homomorphism() {
    // reverse(a ++ b) == reverse(b) ++ reverse(a).
    assert_law(
        "reverse-concat",
        "[ 1 2 3 ] [ 4 5 ] 2 CONCAT REVERSE",
        "[ 4 5 ] REVERSE [ 1 2 3 ] REVERSE 2 CONCAT",
    );
}

#[test]
fn split_then_concat_round_trips() {
    assert_law(
        "split-concat-roundtrip",
        "[ 1 2 3 4 ] [ 2 2 ] SPLIT 2 CONCAT",
        "[ 1 2 3 4 ]",
    );
}

#[test]
fn point_updates_and_collect_are_sequence_transforms() {
    assert_law("insert-point", "[ 1 3 ] [ 1 2 ] INSERT", "[ 1 2 3 ]");
    assert_law("replace-point", "[ 1 2 3 ] [ 1 9 ] REPLACE", "[ 1 9 3 ]");
    assert_law("remove-point", "[ 1 2 3 ] 1 REMOVE", "[ 1 3 ]");
    assert_law("collect-stack", "1 2 3 3 COLLECT", "[ 1 2 3 ]");
}

// ── Tensor / reshape-group laws ──

#[test]
fn transpose_is_involution_2d() {
    for m in ["[ [ 1 2 3 ] [ 4 5 6 ] ]", "[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ]"] {
        assert_law(
            "transpose-involution",
            &format!("{m} TRANSPOSE TRANSPOSE"),
            m,
        );
    }
}

#[test]
fn reshape_round_trips() {
    assert_law(
        "reshape-roundtrip",
        "[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE [ 6 ] RESHAPE",
        "[ 1 2 3 4 5 6 ]",
    );
}

#[test]
fn shape_and_rank_read_index_structure() {
    assert_law("shape-2x3", "[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE", "[ 2 3 ]");
    assert_law("rank-2", "[ [ 1 2 3 ] [ 4 5 6 ] ] RANK", "2");
    // FILL builds a tensor of the requested shape: shape∘fill = id on the shape.
    assert_law("fill-shape", "[ 2 2 7 ] FILL SHAPE", "[ 2 2 ]");
    assert_law("range-literal", "[ 0 5 ] RANGE", "[ 0 1 2 3 4 5 ]");
}

// ── SORT (ALGO) on the decidable rational sub-domain (§7.4.3) ──

#[test]
fn sort_is_idempotent_and_permutation_invariant() {
    assert_law(
        "sort-idempotent",
        "'ALGO' IMPORT [ 3 1 2 ] SORT SORT",
        "'ALGO' IMPORT [ 3 1 2 ] SORT",
    );
    // Sorting is invariant under any prior permutation of the input.
    assert_law(
        "sort-permutation-invariant",
        "'ALGO' IMPORT [ 3 1 2 ] SORT",
        "'ALGO' IMPORT [ 3 1 2 ] REVERSE SORT",
    );
    assert_law(
        "sort-rationals",
        "'ALGO' IMPORT [ 3 1 2 ] SORT",
        "[ 1 2 3 ]",
    );
}
