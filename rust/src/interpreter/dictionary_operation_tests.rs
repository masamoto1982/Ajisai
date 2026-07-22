//! Test suite for interpreter dictionary operations.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_cannot_override_builtin_word() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
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
        interp.execute("'music' IMPORT").await.unwrap();

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

    /// Section 8.6: a word group imported into its own dictionary must be
    /// self-referential. When the same names already exist in another dictionary
    /// (e.g. the bundled Example Words), references inside the imported group
    /// must bind to the group's own words for both dependency tracking and
    /// execution — not to the earlier-loaded dictionary.
    #[tokio::test]
    async fn test_imported_dictionary_is_self_referential() {
        let mut interp = Interpreter::new();

        // EXAMPLE dictionary: SAY pushes [ 1 ], GREET calls SAY.
        interp.execute("{ [ 1 ] } 'SAY' DEF").await.unwrap();
        interp.execute("{ SAY } 'GREET' DEF").await.unwrap();

        // TEST dictionary duplicates the names; here SAY pushes [ 2 ].
        interp.active_user_dictionary = "TEST".to_string();
        interp.execute("{ [ 2 ] } 'SAY' DEF").await.unwrap();
        interp.execute("{ SAY } 'GREET' DEF").await.unwrap();

        interp.rebuild_dependencies().unwrap();

        // Dependency graph: TEST@GREET depends on TEST@SAY (not EXAMPLE@SAY),
        // so TEST@SAY is shown as referenced rather than unreferenced.
        let test_say_deps = interp.dependents.get("TEST@SAY");
        assert!(
            test_say_deps.is_some_and(|d| d.contains("TEST@GREET")),
            "TEST@SAY should be referenced by TEST@GREET; got {:?}",
            test_say_deps
        );
        assert!(
            !test_say_deps.is_some_and(|d| d.contains("EXAMPLE@GREET")),
            "EXAMPLE@GREET must not depend on TEST@SAY"
        );
        let example_say_deps = interp.dependents.get("EXAMPLE@SAY");
        assert!(
            example_say_deps.is_some_and(|d| d.contains("EXAMPLE@GREET")),
            "EXAMPLE@SAY should be referenced by EXAMPLE@GREET; got {:?}",
            example_say_deps
        );

        // Execution: TEST@GREET runs TEST@SAY ([ 2 ]), not EXAMPLE@SAY ([ 1 ]).
        interp.stack.clear();
        interp.execute("TEST@GREET").await.unwrap();
        let top = interp.stack.last().expect("result present");
        let only = top.child(0).expect("vector has one child");
        assert_eq!(only.as_scalar().unwrap().to_i64().unwrap(), 2);
    }

    /// Section 8.6: identical content yields one identity regardless of name or
    /// dictionary (the basis for automatic deduplication on import).
    #[tokio::test]
    async fn test_identical_content_shares_identity() {
        let mut interp = Interpreter::new();
        interp.active_user_dictionary = "A".to_string();
        interp.execute("{ [ 1 ] } 'LEAF' DEF").await.unwrap();
        interp.active_user_dictionary = "B".to_string();
        interp.execute("{ [ 1 ] } 'LEED' DEF").await.unwrap();
        interp.execute("{ [ 2 ] } 'OTHER' DEF").await.unwrap();
        interp.rebuild_dependencies().unwrap();

        let a_leaf = interp.word_identity("A@LEAF").cloned();
        let b_leed = interp.word_identity("B@LEED").cloned();
        let b_other = interp.word_identity("B@OTHER").cloned();
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
        interp.active_user_dictionary = "A".to_string();
        interp.execute("{ [ 1 ] } 'LEAF' DEF").await.unwrap();
        interp.execute("{ LEAF } 'USE' DEF").await.unwrap();
        interp.active_user_dictionary = "B".to_string();
        interp.execute("{ [ 1 ] } 'LEED' DEF").await.unwrap();
        interp.execute("{ LEED } 'USE' DEF").await.unwrap();
        interp.rebuild_dependencies().unwrap();

        let a_use = interp.word_identity("A@USE").cloned();
        let b_use = interp.word_identity("B@USE").cloned();
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
            interp.active_user_dictionary = "R".to_string();
            interp.execute("{ REC } 'REC' DEF").await.unwrap();
            interp.rebuild_dependencies().unwrap();
            // The self-reference must be recorded, then hashed as a cycle.
            assert!(
                interp
                    .dependents
                    .get("R@REC")
                    .is_some_and(|d| d.contains("R@REC")),
                "recursive self-reference should be tracked"
            );
            interp.word_identity("R@REC").cloned()
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
        interp.active_user_dictionary = "A".to_string();
        interp.execute("{ MISSING } 'CALLER' DEF").await.unwrap();
        let before = interp
            .word_identity("A@CALLER")
            .cloned()
            .expect("identity should be computed for caller");

        interp.execute("{ [ 1 ] } 'MISSING' DEF").await.unwrap();

        let after = interp
            .word_identity("A@CALLER")
            .cloned()
            .expect("identity should remain computed for caller");
        assert_eq!(
            before, after,
            "a later definition must not recapture a previously free symbol"
        );
        assert!(
            !interp
                .dependents
                .get("A@MISSING")
                .is_some_and(|deps| deps.contains("A@CALLER")),
            "the existing caller must not become dependent on the later word"
        );
    }

    /// Section 8.6 content store: textually identical bodies defined under
    /// different names/dictionaries share a single stored body in memory rather
    /// than being duplicated.
    #[tokio::test]
    async fn test_identical_bodies_share_one_stored_body() {
        let mut interp = Interpreter::new();
        interp.active_user_dictionary = "A".to_string();
        interp.execute("{ [ 1 ] } 'LEAF' DEF").await.unwrap();
        interp.active_user_dictionary = "B".to_string();
        interp.execute("{ [ 1 ] } 'TWIN' DEF").await.unwrap();
        interp.execute("{ [ 2 ] } 'OTHER' DEF").await.unwrap();

        let a_leaf = interp.user_dictionaries["A"].words["LEAF"].lines.clone();
        let b_twin = interp.user_dictionaries["B"].words["TWIN"].lines.clone();
        let b_other = interp.user_dictionaries["B"].words["OTHER"].lines.clone();

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
        interp.active_user_dictionary = "A".to_string();

        interp.defer_identity_recompute = true;
        interp.execute("{ [ 1 ] } 'LEAF' DEF").await.unwrap();
        assert!(
            interp.word_identity("A@LEAF").is_none(),
            "identity recompute should be deferred"
        );

        interp.defer_identity_recompute = false;
        interp.rebuild_dependencies().unwrap();
        assert!(
            interp.word_identity("A@LEAF").is_some(),
            "identity should be computed once after the batch"
        );
    }

    /// Section 8.6 content store: bodies orphaned by a redefine are reclaimed,
    /// while bodies still shared by a live definition are kept.
    #[tokio::test]
    async fn test_body_store_gc() {
        let mut interp = Interpreter::new();
        interp.active_user_dictionary = "A".to_string();
        interp.execute("{ [ 1 ] } 'X' DEF").await.unwrap();
        assert_eq!(interp.body_store.len(), 1);

        // Identical body in another dictionary shares one store entry.
        interp.active_user_dictionary = "B".to_string();
        interp.execute("{ [ 1 ] } 'Y' DEF").await.unwrap();
        assert_eq!(
            interp.body_store.len(),
            1,
            "identical bodies share one entry"
        );

        // Redefining A@X keeps [1] (still used by B@Y) and adds [9].
        interp.active_user_dictionary = "A".to_string();
        interp.execute("{ [ 9 ] } 'X' DEF").await.unwrap();
        assert_eq!(interp.body_store.len(), 2, "shared [1] kept, [9] added");

        // Redefining B@Y away orphans [1]; it is reclaimed, leaving [9] and [8].
        interp.active_user_dictionary = "B".to_string();
        interp.execute("{ [ 8 ] } 'Y' DEF").await.unwrap();
        assert_eq!(interp.body_store.len(), 2, "orphaned [1] reclaimed");
    }

    #[tokio::test]
    async fn test_cannot_override_other_builtin_words() {
        let mut interp = Interpreter::new();

        let builtin_words = vec!["INSERT", "REPLACE", "MAP", "FILTER", "PRINT"];

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
    async fn test_def_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        let result = interp.execute("{ [ 2 ] * } 'DOUBLE' .. DEF").await;
        assert!(result.is_err(), "DEF should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("DEF") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for DEF, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_del_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();

        let result = interp.execute("'DOUBLE' .. DEL").await;
        assert!(result.is_err(), "DEL should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("DEL") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for DEL, got: {}",
            err_msg
        );
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
        let loaded = interp
            .definition_to_load
            .take()
            .expect("definition_to_load should be set");
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
    async fn test_lookup_module_word_renders_four_section_template() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();
        let result = interp.execute("'MUSIC@PLAY' ?").await;
        assert!(
            result.is_ok(),
            "LOOKUP on module word should succeed: {:?}",
            result.err()
        );
        let loaded = interp
            .definition_to_load
            .take()
            .expect("definition_to_load should be set");
        for section in [
            "# MUSIC@PLAY",
            "Category:",
            "Summary:",
            "Role:",
            "Stack Effect:",
        ] {
            assert!(
                loaded.contains(section),
                "Module LOOKUP must include '{}' section, got: {}",
                section,
                loaded
            );
        }
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

    #[tokio::test]
    async fn test_lookup_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        interp.execute("{ [ 2 ] * } 'DOUBLE' DEF").await.unwrap();

        let result = interp.execute("'DOUBLE' .. ?").await;
        assert!(result.is_err(), "LOOKUP should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("LOOKUP") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for LOOKUP, got: {}",
            err_msg
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

        let result = interp.execute("! 'C4' DEL").await;
        assert!(result.is_ok(), "Should force delete C4: {:?}", result.err());
        assert!(!interp.user_words.contains_key("C4"));
    }

    #[tokio::test]
    async fn test_del_sample_user_words_with_fqn() {
        let mut interp = Interpreter::new();

        let example_words = vec![
            ("C4", "264", "純正律 C4"),
            ("D4", "C4 9 * 8 /", "純正律 D4"),
            ("E4", "C4 5 * 4 /", "純正律 E4"),
        ];
        restore_example_words(&mut interp, &example_words);

        assert!(interp.user_words.contains_key("D4"));

        let result = interp.execute("'EXAMPLE@D4' DEL").await;
        assert!(
            result.is_ok(),
            "Should delete D4 via FQN: {:?}",
            result.err()
        );
        assert!(!interp.user_words.contains_key("D4"));

        let result = interp.execute("'EXAMPLE@NONEXISTENT' DEL").await;
        assert!(result.is_err(), "Should error for non-existent FQN word");

        let result = interp.execute("'EXAMPLE@C4' DEL").await;
        assert!(
            result.is_err(),
            "Should not delete C4 via FQN (has dependents)"
        );

        let result = interp.execute("! 'EXAMPLE@C4' DEL").await;
        assert!(
            result.is_ok(),
            "Should force delete C4 via FQN: {:?}",
            result.err()
        );
        assert!(!interp.user_words.contains_key("C4"));
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
    async fn test_numeric_vector_literal_play_after_music_sample_reset() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute("[ 264 297 330 ] MUSIC@SEQ MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "[ 264 297 330 ] MUSIC@SEQ MUSIC@PLAY should succeed: {:?}",
            result.err()
        );

        let output = interp.collect_output();

        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command, got: {}",
            output
        );
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
    async fn test_user_word_resolved_in_nested_vector() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        interp.execute("{ 264 } 'C4' DEF").await.unwrap();
        interp.execute("{ 330 } 'E4' DEF").await.unwrap();
        interp.execute("{ 396 } 'G4' DEF").await.unwrap();
        let _ = interp.collect_output();

        let result = interp
            .execute("[ [ C4 E4 G4 ] ] MUSIC@SIM MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "Nested vector with user words should work: {:?}",
            result.err()
        );

        let output = interp.collect_output();
        assert!(output.contains("AUDIO:"), "Should contain AUDIO command");
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
    async fn test_def_without_music_sample_collision_warning() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute("{ [ 999 ] } 'C4' DEF").await;
        assert!(
            result.is_ok(),
            "DEF should succeed after MUSIC sample dictionary reset: {:?}",
            result.err()
        );
        let output = interp.collect_output();
        assert!(
            !output.contains("MUSIC@C4"),
            "No MUSIC@C4 collision should be reported after sample reset: {}",
            output
        );
    }

    #[tokio::test]
    async fn test_import_keeps_user_word_unambiguous_after_music_sample_reset() {
        let mut interp = Interpreter::new();

        interp.execute("{ [ 999 ] } 'C4' DEF").await.unwrap();
        assert!(interp.user_words.contains_key("C4"));

        interp.execute("'music' IMPORT").await.unwrap();
        let output = interp.collect_output();

        assert!(
            interp.user_words.contains_key("C4"),
            "User word C4 should remain in EXAMPLE after IMPORT"
        );
        assert!(
            !output.contains("MUSIC@C4"),
            "MUSIC has no C4 sample after reset, so no conflict warning is expected: {}",
            output
        );

        let result = interp.execute("C4").await;
        assert!(
            result.is_ok(),
            "C4 should resolve to the user word after MUSIC sample reset: {:?}",
            result.err()
        );
        if let Some(val) = interp.stack.last() {
            let scalar_owned = val
                .as_scalar()
                .cloned()
                .or_else(|| val.child(0).and_then(|c| c.as_scalar().cloned()));
            let scalar = scalar_owned.expect("C4 should resolve to a numeric value");
            assert_eq!(
                scalar.to_i64().unwrap(),
                999,
                "C4 should remain the user-defined value"
            );
        }

        let result = interp.execute("MUSIC@C4").await;
        assert!(
            result.is_err(),
            "Qualified MUSIC@C4 should not exist after sample reset"
        );
    }

    #[tokio::test]
    async fn test_music_sample_dictionary_is_reset() {
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let _ = interp.collect_output();

        let result = interp.execute("MUSIC@C4").await;
        assert!(
            result.is_err(),
            "MUSIC@C4 should not exist after resetting Example Words"
        );

        let result = interp.execute("MUSIC@SEQ").await;
        assert!(
            result.is_ok(),
            "MUSIC built-in words should remain available: {:?}",
            result.err()
        );
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
}
