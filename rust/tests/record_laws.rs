//! Property-based record laws (Phase 9, SPEC §4.4).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-octies I.1 (Phase 9):
//! a record is an **insertion-order-preserving finite map** `Name ⇀ V`. Records
//! are constructed and operated on through the `JSON` module
//! (`JSON@PARSE`/`GET`/`SET`/`KEYS`/`VALUES`/`MERGE`) — there is no record
//! literal syntax (finding I1), so every program imports `json` first.
//!
//! Observation is firewall-clean: a record is read through the Phase 1 axes
//! (`semanticKind = record`, `shape = record`) and the pure `render`; field
//! values are compared by their render. Every law was probe-confirmed first
//! (roadmap §1.2-(T)).

mod test_support;

use proptest::prelude::*;
use test_support::generators::record_abc;
use test_support::observe::{observe_axes, render, run};

/// Whole-stack rendering.
fn obs(src: &str) -> Vec<String> {
    run(src).iter().map(|v| render(v, v.hint)).collect()
}

/// A `{"a":va,"b":vb,"c":vc}` JSON object source.
fn rec(a: i64, b: i64, c: i64) -> String {
    format!("'json' IMPORT '{{\"a\":{a},\"b\":{b},\"c\":{c}}}' JSON@PARSE")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// **A parsed record is observable as a record** (SPEC §4.4 / §2.3):
    /// `semanticKind = record`, `shape = record`.
    #[test]
    fn record_is_observable((a, b, c) in record_abc()) {
        let stack = run(&rec(a, b, c));
        prop_assert_eq!(stack.len(), 1);
        let axes = observe_axes(&stack[0]);
        prop_assert_eq!(axes.semantic_kind, "record");
        prop_assert_eq!(axes.shape, "record");
    }

    /// **`GET` reads the field a key maps to** (the map's defining property):
    /// `{a:va …} 'a' GET = va`.
    #[test]
    fn get_reads_the_mapped_value((a, b, c) in record_abc()) {
        prop_assert_eq!(obs(&format!("{} 'a' JSON@GET", rec(a, b, c))), vec![format!("{a}/1")]);
        prop_assert_eq!(obs(&format!("{} 'b' JSON@GET", rec(a, b, c))), vec![format!("{b}/1")]);
        prop_assert_eq!(obs(&format!("{} 'c' JSON@GET", rec(a, b, c))), vec![format!("{c}/1")]);
    }

    /// **get-after-set on the same key returns the new value** (functional
    /// update): `r 'a' v SET 'a' GET = v`.
    #[test]
    fn get_after_set_same_key((a, b, c) in record_abc(), v in -20i64..=20) {
        prop_assert_eq!(
            obs(&format!("{} 'a' {v} JSON@SET 'a' JSON@GET", rec(a, b, c))),
            vec![format!("{v}/1")]
        );
    }

    /// **set on one key does not disturb another** (pointwise update): setting
    /// `a` leaves `b` unchanged.
    #[test]
    fn set_is_pointwise((a, b, c) in record_abc(), v in -20i64..=20) {
        prop_assert_eq!(
            obs(&format!("{} 'a' {v} JSON@SET 'b' JSON@GET", rec(a, b, c))),
            vec![format!("{b}/1")]
        );
    }

    /// **`KEYS` preserves insertion order, and `SET` of an existing key keeps
    /// the key set / order** (insertion-order finite map, §4.4).
    #[test]
    fn keys_order_is_stable_under_set((a, b, c) in record_abc(), v in -20i64..=20) {
        let base_keys = obs(&format!("{} JSON@KEYS", rec(a, b, c)));
        let after_set = obs(&format!("{} 'a' {v} JSON@SET JSON@KEYS", rec(a, b, c)));
        prop_assert_eq!(base_keys, after_set);
    }

    /// **`VALUES` lists the values in key order**: for `{a,b,c}` it is
    /// `[va vb vc]`.
    #[test]
    fn values_are_in_key_order((a, b, c) in record_abc()) {
        prop_assert_eq!(
            obs(&format!("{} JSON@VALUES", rec(a, b, c))),
            vec![format!("[ {a}/1 {b}/1 {c}/1 ]")]
        );
    }

    /// **`MERGE` is right-biased on overlapping keys** (SPEC §4.4): merging an
    /// overlay that redefines `b` and adds `d` keeps the base's `a`, takes the
    /// overlay's `b`, and adds `d`.
    #[test]
    fn merge_is_right_biased((a, b, c) in record_abc(), b2 in -20i64..=20, d in -20i64..=20) {
        let base = rec(a, b, c);
        let overlay = format!("'{{\"b\":{b2},\"d\":{d}}}' JSON@PARSE");
        let merged = format!("{base} {overlay} JSON@MERGE");
        prop_assert_eq!(obs(&format!("{merged} 'a' JSON@GET")), vec![format!("{a}/1")]);
        prop_assert_eq!(obs(&format!("{merged} 'b' JSON@GET")), vec![format!("{b2}/1")]);
        prop_assert_eq!(obs(&format!("{merged} 'd' JSON@GET")), vec![format!("{d}/1")]);
    }
}

/// **A missing key projects NIL** (`JSON@GET` is total-by-projection, Bubble
/// Rule §11.2): looking up an absent key yields an absence value, not an error.
#[test]
fn get_missing_key_projects_nil() {
    assert_eq!(
        obs("'json' IMPORT '{\"x\":5}' JSON@PARSE 'z' JSON@GET"),
        vec!["NIL".to_string()]
    );
}

/// **PARSE∘STRINGIFY round-trips a canonical object** (the record is the same
/// finite map): `'{"a":1}'` survives a parse/stringify round trip.
#[test]
fn stringify_parse_round_trip() {
    assert_eq!(
        obs("'json' IMPORT '{\"a\":1}' JSON@PARSE JSON@STRINGIFY"),
        vec!["'{\"a\":1}'".to_string()]
    );
}
