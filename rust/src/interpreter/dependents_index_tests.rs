//! Tests for the reverse-dependency index reads (`collect_dependents` /
//! `collect_transitive_dependents`).
//!
//! `collect_dependents` now reads the maintained `dependents` inverted index
//! instead of rescanning every user dictionary. Its body carries a
//! `debug_assert_eq!` that cross-checks the index against the authoritative
//! full scan on every call; because the test suite runs with debug assertions
//! enabled, every `collect_dependents` call in these scenarios — and in the
//! rest of the suite — also verifies that the maintained index has not drifted
//! from ground truth. The assertions below additionally pin the *values*
//! returned across DEF / redefine / DEL sequences.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use std::collections::HashSet;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// A word that references another user word records a direct dependency, so
    /// the referenced word's `collect_dependents` reports the referrer.
    #[tokio::test]
    async fn direct_dependent_is_reported() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'A' DEF").await.unwrap();
        interp.execute("{ A } 'B' DEF").await.unwrap();

        assert_eq!(
            interp.collect_dependents("EXAMPLE@A"),
            set(&["EXAMPLE@B"]),
            "B references A, so A's dependents must be {{B}}"
        );
        assert!(
            interp.collect_dependents("EXAMPLE@B").is_empty(),
            "nothing references B, so B has no dependents"
        );
    }

    /// `collect_transitive_dependents` walks the whole reverse chain, while
    /// `collect_dependents` reports only the direct referrers.
    #[tokio::test]
    async fn transitive_chain_is_followed() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'A' DEF").await.unwrap();
        interp.execute("{ A } 'B' DEF").await.unwrap();
        interp.execute("{ B } 'C' DEF").await.unwrap();

        assert_eq!(
            interp.collect_dependents("EXAMPLE@A"),
            set(&["EXAMPLE@B"]),
            "direct dependents of A is just B"
        );
        assert_eq!(
            interp.collect_transitive_dependents("EXAMPLE@A"),
            set(&["EXAMPLE@B", "EXAMPLE@C"]),
            "transitive dependents of A reach C through B"
        );
        assert_eq!(
            interp.collect_transitive_dependents("EXAMPLE@B"),
            set(&["EXAMPLE@C"]),
            "transitive dependents of B is just C"
        );
        assert!(
            interp.collect_transitive_dependents("EXAMPLE@C").is_empty(),
            "C is a leaf in the dependency chain"
        );
    }

    /// A word may be referenced by several others; all of them appear.
    #[tokio::test]
    async fn multiple_direct_dependents() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'A' DEF").await.unwrap();
        interp.execute("{ A } 'B' DEF").await.unwrap();
        interp.execute("{ A } 'C' DEF").await.unwrap();

        assert_eq!(
            interp.collect_dependents("EXAMPLE@A"),
            set(&["EXAMPLE@B", "EXAMPLE@C"]),
            "both B and C reference A"
        );
    }

    /// Redefining a word so it no longer references its former dependency drops
    /// the reverse edge. The `debug_assert_eq!` inside `collect_dependents`
    /// guarantees the maintained index still matches a full scan after the
    /// redefinition's incremental edge removal.
    #[tokio::test]
    async fn redefine_drops_stale_reverse_edge() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'A' DEF").await.unwrap();
        interp.execute("{ A } 'B' DEF").await.unwrap();
        assert_eq!(interp.collect_dependents("EXAMPLE@A"), set(&["EXAMPLE@B"]));

        // B no longer references A. B has no dependents, so no force is needed.
        interp.execute("{ [ 2 ] } 'B' DEF").await.unwrap();

        assert!(
            interp.collect_dependents("EXAMPLE@A").is_empty(),
            "after redefining B without A, A must have no dependents"
        );
    }

    /// Deleting a referrer removes it from the referenced word's dependents.
    #[tokio::test]
    async fn delete_referrer_clears_reverse_edge() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'A' DEF").await.unwrap();
        interp.execute("{ A } 'B' DEF").await.unwrap();
        assert_eq!(interp.collect_dependents("EXAMPLE@A"), set(&["EXAMPLE@B"]));

        // B is a leaf (nothing depends on it), so a plain DEL is allowed.
        interp.execute("'B' DEL").await.unwrap();

        assert!(
            interp.collect_dependents("EXAMPLE@A").is_empty(),
            "after deleting B, A must have no dependents"
        );
    }

    /// A word with no dependents reports the empty set (index miss path), which
    /// must agree with the full scan via the in-call debug assertion.
    #[tokio::test]
    async fn unknown_word_has_no_dependents() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'A' DEF").await.unwrap();

        assert!(
            interp.collect_dependents("EXAMPLE@NOPE").is_empty(),
            "a name nothing references has no dependents"
        );
        assert!(
            interp
                .collect_transitive_dependents("EXAMPLE@NOPE")
                .is_empty(),
            "an unreferenced name has no transitive dependents"
        );
    }
}
