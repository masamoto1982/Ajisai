//! Test suite for interpreter dictionary operations.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_cannot_override_builtin_word() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        let result = interp.execute("{ [ 1 ] + } 'GET' DEF").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Cannot redefine built-in word"),
            "Expected error message to contain 'Cannot redefine built-in word', got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_can_override_user_word() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();

        let result1 = interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await;
        assert!(result1.is_ok(), "First definition should succeed");

        let result2 = interp.execute("{ [ 3 ] * } 'DOUBLE' DEF").await;
        assert!(result2.is_ok(), "Overriding user word should succeed");

        let result3 = interp.execute("[ 5 ] DOUBLE").await;
        assert!(result3.is_ok(), "Executing redefined word should succeed");

        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            assert!(val.is_vector(), "Expected vector result");
            assert_eq!(val.len(), 1, "Result should have one element");
            let only = val.child(0).expect("len==1 implies child(0) exists");
            {
                if let Some(f) = only.as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 15, "Result should be 15");
                } else {
                    panic!("Expected scalar inside vector");
                }
            }
        }
    }

    /// A redefinition rebinds the one User entry, and dependents follow it.
    ///
    /// This used to test that a word resolved its references through its *own*
    /// dictionary, so two dictionaries could each hold a `SAY`/`GREET` pair
    /// without their edges crossing. There is one User tier, so the second
    /// definition of a name replaces the first and the dependency edge moves
    /// with it.
    #[tokio::test]
    async fn test_redefinition_rebinds_the_single_user_entry() {
        let mut interp = Interpreter::new();

        interp.execute("{ [ 1 ] } 'SAY' DEF").await.unwrap();
        interp.execute("{ SAY } 'GREET' DEF").await.unwrap();
        interp.rebuild_dependencies().unwrap();

        assert!(
            interp
                .dependents
                .get("SAY")
                .is_some_and(|d| d.contains("GREET")),
            "GREET depends on SAY"
        );

        interp.execute("GREET").await.unwrap();
        assert_eq!(
            format!("{}", interp.get_stack().last().expect("a result")),
            "[ 1/1 ]"
        );
    }

    /// Section 8.6: identical content yields one identity regardless of name or
    /// dictionary (the basis for automatic deduplication on import).
    #[tokio::test]
    async fn test_identical_content_shares_identity() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'LEAF' DEF").await.unwrap();
        interp.execute("{ [ 1 ] } 'LEED' DEF").await.unwrap();
        interp.execute("{ [ 2 ] } 'OTHER' DEF").await.unwrap();
        interp.rebuild_dependencies().unwrap();

        let a_leaf = interp.word_identity("LEAF").cloned();
        let b_leed = interp.word_identity("LEED").cloned();
        let b_other = interp.word_identity("OTHER").cloned();
        assert!(a_leaf.is_some(), "identity should be computed");
        assert_eq!(a_leaf, b_leed, "identical bodies must share an identity");
        assert_ne!(a_leaf, b_other, "different bodies must differ");
    }

    /// Section 8.6: a word's identity depends on the content of its dependency,
    /// not on the dependency's name. Two words that call differently-named but
    /// identical helpers must share an identity.
    #[tokio::test]
    async fn test_identity_is_name_independent() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'LEAF' DEF").await.unwrap();
        interp.execute("{ LEAF } 'USE' DEF").await.unwrap();
        interp.execute("{ [ 1 ] } 'LEED' DEF").await.unwrap();
        interp.execute("{ LEED } 'USE' DEF").await.unwrap();
        interp.rebuild_dependencies().unwrap();

        let a_use = interp.word_identity("USE").cloned();
        let b_use = interp.word_identity("USE").cloned();
        assert!(a_use.is_some());
        assert_eq!(
            a_use, b_use,
            "callers of identical helpers must share an identity regardless of the helper's name"
        );
    }

    /// Section 8.6: a recursive word's identity is well-defined (cycle hashing)
    /// and reproducible across independent interpreters.
    #[tokio::test]
    async fn test_recursive_identity_is_stable() {
        async fn rec_id() -> Option<String> {
            let mut interp = Interpreter::new();
            interp.execute("{ REC } 'REC' DEF").await.unwrap();
            interp.rebuild_dependencies().unwrap();
            // The self-reference must be recorded, then hashed as a cycle.
            assert!(
                interp
                    .dependents
                    .get("REC")
                    .is_some_and(|d| d.contains("REC")),
                "recursive self-reference should be tracked"
            );
            interp.word_identity("REC").cloned()
        }

        let first = rec_id().await;
        let second = rec_id().await;
        assert!(first.is_some(), "recursive word should have an identity");
        assert_eq!(first, second, "recursive identity must be reproducible");
    }

    /// Section 8.6: adding a later word with the same spelling as a formerly
    /// unresolved reference must not recapture the existing body or change its
    /// content identity. Dependencies are fixed at definition time.
    #[tokio::test]
    async fn test_unresolved_reference_identity_is_not_recaptured() {
        let mut interp = Interpreter::new();
        interp.execute("{ MISSING } 'CALLER' DEF").await.unwrap();
        let before = interp
            .word_identity("CALLER")
            .cloned()
            .expect("identity should be computed for caller");

        interp.execute("{ [ 1 ] } 'MISSING' DEF").await.unwrap();

        let after = interp
            .word_identity("CALLER")
            .cloned()
            .expect("identity should remain computed for caller");
        assert_eq!(
            before, after,
            "a later definition must not recapture a previously free symbol"
        );
        assert!(
            !interp
                .dependents
                .get("MISSING")
                .is_some_and(|deps| deps.contains("CALLER")),
            "the existing caller must not become dependent on the later word"
        );
    }

    /// Content store: textually identical bodies defined under different names
    /// share a single stored body in memory rather than being duplicated.
    #[tokio::test]
    async fn test_identical_bodies_share_one_stored_body() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'LEAF' DEF").await.unwrap();
        interp.execute("{ [ 1 ] } 'TWIN' DEF").await.unwrap();
        interp.execute("{ [ 2 ] } 'OTHER' DEF").await.unwrap();

        let a_leaf = interp.user_words["LEAF"].lines.clone();
        let b_twin = interp.user_words["TWIN"].lines.clone();
        let b_other = interp.user_words["OTHER"].lines.clone();

        assert!(
            std::sync::Arc::ptr_eq(&a_leaf, &b_twin),
            "identical bodies must share one interned stored body"
        );
        assert!(
            !std::sync::Arc::ptr_eq(&a_leaf, &b_other),
            "different bodies must not share a stored body"
        );
    }

    /// Deferring identity recomputation during a bulk operation skips per-word
    /// recomputes; a single recompute at the end (here via rebuild) restores
    /// correctness. This is what makes import O(N) rather than O(N^2).
    #[tokio::test]
    async fn test_deferred_identity_recompute() {
        let mut interp = Interpreter::new();

        interp.defer_identity_recompute = true;
        interp.execute("{ [ 1 ] } 'LEAF' DEF").await.unwrap();
        assert!(
            interp.word_identity("LEAF").is_none(),
            "identity recompute should be deferred"
        );

        interp.defer_identity_recompute = false;
        interp.rebuild_dependencies().unwrap();
        assert!(
            interp.word_identity("LEAF").is_some(),
            "identity should be computed once after the batch"
        );
    }

    /// Section 8.6 content store: bodies orphaned by a redefine are reclaimed,
    /// while bodies still shared by a live definition are kept.
    #[tokio::test]
    async fn test_body_store_gc() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 1 ] } 'X' DEF").await.unwrap();
        assert_eq!(interp.body_store.len(), 1);

        // Identical body in another dictionary shares one store entry.
        interp.execute("{ [ 1 ] } 'Y' DEF").await.unwrap();
        assert_eq!(
            interp.body_store.len(),
            1,
            "identical bodies share one entry"
        );

        // Redefining A@X keeps [1] (still used by B@Y) and adds [9].
        interp.execute("{ [ 9 ] } 'X' DEF").await.unwrap();
        assert_eq!(interp.body_store.len(), 2, "shared [1] kept, [9] added");

        // Redefining B@Y away orphans [1]; it is reclaimed, leaving [9] and [8].
        interp.execute("{ [ 8 ] } 'Y' DEF").await.unwrap();
        assert_eq!(interp.body_store.len(), 2, "orphaned [1] reclaimed");
    }

    #[tokio::test]
    async fn test_cannot_override_other_builtin_words() {
        let mut interp = Interpreter::new();

        let builtin_words = vec!["TAKE", "REVERSE", "MAP", "FILTER", "PRINT"];

        for word in builtin_words {
            let code = format!("{{ [ 1 ] + }} '{}' DEF", word);
            let result = interp.execute(&code).await;
            assert!(
                result.is_err(),
                "Should not be able to override builtin word: {}",
                word
            );
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("Cannot redefine built-in word"),
                "Expected error for {}, got: {}",
                word,
                err_msg
            );
        }
    }

    #[tokio::test]
    async fn test_lookup_builtin_renders_four_section_template() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'GET' ?").await;
        assert!(
            result.is_ok(),
            "LOOKUP on built-in GET should succeed: {:?}",
            result.err()
        );
        // A Core Word's entry is reference text, not editable source, so it
        // arrives on the documentation channel — the host shows it rather than
        // loading it over whatever is in the editor.
        assert!(
            interp.definition_to_load.is_none(),
            "a Core Word's LOOKUP must not be offered as a definition to load"
        );
        let loaded = interp
            .documentation_to_show
            .take()
            .expect("documentation_to_show should be set");
        for section in ["# GET", "Category:", "Summary:", "Role:", "Stack Effect:"] {
            assert!(
                loaded.contains(section),
                "Built-in LOOKUP must include '{}' section, got: {}",
                section,
                loaded
            );
        }
        assert!(
            !loaded.contains("] DEF"),
            "Built-in LOOKUP should not produce a DEF expression, got: {}",
            loaded
        );
    }
    #[tokio::test]
    async fn test_lookup_user_word_loads_def_source() {
        let mut interp = Interpreter::new();
        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();
        let _ = interp.collect_output();
        let result = interp.execute("'DOUBLE' ?").await;
        assert!(
            result.is_ok(),
            "LOOKUP on user word should succeed: {:?}",
            result.err()
        );
        let loaded = interp
            .definition_to_load
            .take()
            .expect("definition_to_load should be set");
        assert!(
            loaded.contains("DEF") && loaded.contains("'DOUBLE'"),
            "User-word LOOKUP should reconstruct DEF source, got: {}",
            loaded
        );
        assert!(
            !loaded.contains("placeholder"),
            "User-word LOOKUP should not load placeholder text, got: {}",
            loaded
        );
    }

    fn restore_example_words(interp: &mut Interpreter, example_words: &[(&str, &str, &str)]) {
        use crate::tokenizer;

        for (name, definition, _description) in example_words {
            let tokens = tokenizer::tokenize(definition)
                .unwrap_or_else(|e| panic!("Failed to tokenize {}: {}", name, e));

            crate::interpreter::execute_def::op_def_inner(interp, name, &tokens)
                .unwrap_or_else(|e| panic!("Failed to define {}: {}", name, e));
        }

        interp
            .rebuild_dependencies()
            .expect("Failed to rebuild dependencies");
    }

    #[tokio::test]
    async fn test_del_sample_user_words() {
        let mut interp = Interpreter::new();

        let example_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
            ("E4", "C4 5 * 4 /", "純正律 E4"),
        ];
        restore_example_words(&mut interp, &example_words);

        assert!(interp.user_words.contains_key("C4"));
        assert!(interp.user_words.contains_key("D4"));
        assert!(interp.user_words.contains_key("E4"));

        let result = interp.execute("'D4' DEL").await;
        assert!(result.is_ok(), "Should delete D4: {:?}", result.err());
        assert!(!interp.user_words.contains_key("D4"));

        let result = interp.execute("'C4' DEL").await;
        assert!(result.is_err(), "Should not delete C4 (has dependents)");

        // There is no force modifier: DEL fails while any definition still
        // depends on the target (LANG.DICTIONARY.MUTATION). Removing the last
        // dependent is what makes the delete legal.
        interp
            .execute("'E4' DEL")
            .await
            .expect("Should delete the remaining dependent E4");
        let result = interp.execute("'C4' DEL").await;
        assert!(
            result.is_ok(),
            "Should delete C4 once nothing depends on it: {:?}",
            result.err()
        );
        assert!(!interp.user_words.contains_key("C4"));
    }

    /// A qualified path is not a name.
    ///
    /// `DEL` used to accept `DICT@WORD` and delete through the named
    /// dictionary. LANG.DICTIONARY.RESOLUTION gives two tiers, so a Word's name
    /// is its whole address and a path addresses nothing.
    #[tokio::test]
    async fn test_del_rejects_a_qualified_path() {
        let mut interp = Interpreter::new();

        let example_words = vec![("D4", "264", "test word")];
        restore_example_words(&mut interp, &example_words);
        assert!(interp.user_words.contains_key("D4"));

        let result = interp.execute("'EXAMPLE@D4' DEL").await;
        assert!(result.is_err(), "a qualified path names nothing");
        assert!(
            interp.user_words.contains_key("D4"),
            "the word is untouched"
        );

        interp
            .execute("'D4' DEL")
            .await
            .expect("its name deletes it");
        assert!(!interp.user_words.contains_key("D4"));
    }

    #[tokio::test]
    async fn test_execute_restored_example_words() {
        let mut interp = Interpreter::new();

        let example_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
        ];
        restore_example_words(&mut interp, &example_words);

        let result = interp.execute("C4").await;
        assert!(
            result.is_ok(),
            "Executing C4 should succeed: {:?}",
            result.err()
        );
        assert_eq!(interp.stack.len(), 1);

        let result = interp.execute("D4").await;
        assert!(
            result.is_ok(),
            "Executing D4 should succeed: {:?}",
            result.err()
        );
        assert_eq!(interp.stack.len(), 2);
    }
    #[tokio::test]
    async fn test_example_words_scalar_output() {
        let mut interp = Interpreter::new();

        let example_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
        ];
        restore_example_words(&mut interp, &example_words);
        let _ = interp.collect_output();

        interp.execute("C4").await.unwrap();
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            assert!(
                val.as_scalar().is_some(),
                "C4 should push a scalar, not a vector"
            );
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 264);
        }

        interp.execute("D4").await.unwrap();
        assert_eq!(interp.stack.len(), 2);
        if let Some(val) = interp.stack.last() {
            assert!(
                val.as_scalar().is_some(),
                "D4 should push a scalar, not a vector"
            );
            assert_eq!(val.as_scalar().unwrap().to_i64().unwrap(), 297);
        }
    }

    #[tokio::test]
    async fn test_builtin_symbols_remain_strings_in_vector() {
        let mut interp = Interpreter::new();

        let result = interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await;
        assert!(
            result.is_ok(),
            "Code block DEF should work: {:?}",
            result.err()
        );

        let result = interp.execute("[ 5 ] DOUBLE").await;
        assert!(
            result.is_ok(),
            "Executing DOUBLE should succeed: {:?}",
            result.err()
        );
        if let Some(val) = interp.stack.last() {
            assert!(val.is_vector(), "Expected vector result");
            assert_eq!(val.len(), 1);
            let only = val.child(0).expect("len==1 implies child(0) exists");
            assert_eq!(only.as_scalar().unwrap().to_i64().unwrap(), 10);
        }
    }
    #[tokio::test]
    async fn test_def_with_vector_duality() {
        let mut interp = Interpreter::new();

        let result = interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await;
        assert!(
            result.is_ok(),
            "DEF with vector should succeed: {:?}",
            result
        );

        let result = interp.execute("[ 5 ] DOUBLE").await;
        assert!(
            result.is_ok(),
            "Executing DOUBLE should succeed: {:?}",
            result
        );

        assert_eq!(interp.stack.len(), 1, "Stack should have 1 element");
        if let Some(val) = interp.stack.last() {
            assert!(val.is_vector(), "Expected vector result");
            assert_eq!(val.len(), 1, "Result should have one element");
            let only = val.child(0).expect("len==1 implies child(0) exists");
            {
                if let Some(f) = only.as_scalar() {
                    assert_eq!(f.to_i64().unwrap(), 10, "Result should be 10");
                } else {
                    panic!("Expected scalar inside vector");
                }
            }
        }
    }
    #[tokio::test]
    async fn test_module_first_builtin_still_protected() {
        let mut interp = Interpreter::new();
        let result = interp.execute("{ [ 1 ] } 'GET' DEF").await;
        assert!(
            result.is_err(),
            "Should not be able to override built-in GET"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Cannot redefine built-in word"),
            "Expected BuiltinProtection error, got: {}",
            err_msg
        );
    }

    // ── A word's own self-reference must not lock it ──────────────────────
    // A recursive word depends on itself. Once that self-edge is in the
    // dependency index, the guard that protects *callers* from losing a
    // definition fired on the word itself: `DEF` and `DEL` both refused with
    // "referenced by FIB — delete those words first", naming the word being
    // deleted. There was then no Word in the vocabulary that could correct or
    // remove a recursive definition, so writing one and fixing it — the most
    // ordinary development loop there is — was impossible without discarding
    // the whole User dictionary.

    const SELF_RECURSIVE: &str = "{\n\
         { 0 LTE | 0 * }\n\
         { IDLE | 1 - SELFW }\n\
         COND } 'SELFW' DEF";

    const SELF_RECURSIVE_V2: &str = "{\n\
         { 0 LTE | 0 * }\n\
         { IDLE | 2 - SELFW }\n\
         COND } 'SELFW' DEF";

    #[tokio::test]
    async fn a_self_recursive_word_can_be_redefined() {
        let mut interp = Interpreter::new();
        interp.execute(SELF_RECURSIVE).await.unwrap();
        // The second DEF is what records the self-edge (SELFW now resolves
        // while its own body is scanned), so the third is the one that used to
        // be refused.
        interp.execute(SELF_RECURSIVE_V2).await.unwrap();
        let result = interp.execute(SELF_RECURSIVE).await;
        assert!(
            result.is_ok(),
            "a word's own self-reference must not block redefining it: {result:?}"
        );
    }

    #[tokio::test]
    async fn a_self_recursive_word_can_be_deleted() {
        let mut interp = Interpreter::new();
        interp.execute(SELF_RECURSIVE).await.unwrap();
        interp.execute(SELF_RECURSIVE_V2).await.unwrap();
        let result = interp.execute("'SELFW' DEL").await;
        assert!(
            result.is_ok(),
            "a word's own self-reference must not block deleting it: {result:?}"
        );
        assert!(interp.execute("3 SELFW").await.is_err(), "SELFW is gone");
    }

    #[tokio::test]
    async fn another_word_still_locks_a_self_recursive_word() {
        // The protection that matters is unchanged: a *different* word holding
        // a reference still refuses the redefinition and the deletion.
        let mut interp = Interpreter::new();
        interp.execute(SELF_RECURSIVE).await.unwrap();
        interp.execute("{ SELFW } 'CALLER' DEF").await.unwrap();

        let err = interp
            .execute(SELF_RECURSIVE_V2)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("CALLER"),
            "redefinition must still name the external dependent: {err}"
        );

        let err = interp.execute("'SELFW' DEL").await.unwrap_err().to_string();
        assert!(
            err.contains("CALLER"),
            "deletion must still name the external dependent: {err}"
        );
    }

    // ── LOOKUP must round-trip ────────────────────────────────────────────
    // Loading a user word, editing it, and defining it again is the ordinary
    // way to correct a definition. That only works if what LOOKUP loads is
    // source `DEF` accepts. It used to wrap the body in `[ ]`: `DEF` rejects a
    // Vector body outright, and a `|` clause inside `[ ]` does not even
    // tokenize, so a COND word could not be reloaded at all.

    async fn lookup_source(interp: &mut Interpreter, name: &str) -> String {
        interp.execute(&format!("'{name}' ?")).await.unwrap();
        let _ = interp.collect_output();
        interp
            .definition_to_load
            .take()
            .expect("LOOKUP must load a definition")
    }

    #[tokio::test]
    async fn lookup_of_a_user_word_round_trips_through_def() {
        let mut interp = Interpreter::new();
        interp.execute("{ 2 MUL } 'DBL' DEF").await.unwrap();
        let loaded = lookup_source(&mut interp, "DBL").await;
        assert!(
            loaded.starts_with('{'),
            "the body must be a code block, not a vector: {loaded}"
        );

        // Running the loaded text redefines the word, and it still computes.
        interp.execute(&loaded).await.unwrap();
        interp.execute("5 DBL").await.unwrap();
        assert_eq!(format!("{}", interp.stack.last().unwrap()), "10/1");
    }

    #[tokio::test]
    async fn lookup_of_a_cond_word_round_trips_through_def() {
        let mut interp = Interpreter::new();
        interp
            .execute("{\n{ 5 LT | 'small' }\n{ IDLE | 'big' }\nCOND } 'SIZE' DEF")
            .await
            .unwrap();
        let loaded = lookup_source(&mut interp, "SIZE").await;
        assert!(
            loaded.contains('|'),
            "the clause separator must survive the round trip: {loaded}"
        );

        interp.execute(&loaded).await.unwrap();
        interp.execute("7 SIZE").await.unwrap();
        assert_eq!(format!("{}", interp.stack.last().unwrap()), "'big'");
    }
}
