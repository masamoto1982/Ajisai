//! Test suite for `crate::interpreter::modules` unimport behavior.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn unimport_hides_unreferenced_module_words() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT").await.unwrap();

        interp.execute("'json' UNIMPORT").await.unwrap();

        assert!(
            interp.execute("'[1]' JSON@PARSE").await.is_err(),
            "UNIMPORT should hide unreferenced module words"
        );
        assert!(
            interp.module_vocabulary.contains_key("JSON"),
            "UNIMPORT must not destroy the module dictionary cache"
        );
        assert!(
            !interp.import_table.modules.contains_key("JSON"),
            "UNIMPORT with no references should remove only import-table visibility"
        );
    }

    #[tokio::test]
    async fn unimport_keeps_module_words_referenced_by_user_words() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT").await.unwrap();
        interp
            .execute("{ JSON@PARSE } 'USE-PARSE' DEF")
            .await
            .unwrap();

        interp.execute("'json' UNIMPORT").await.unwrap();

        assert!(
            interp.execute("'[1]' JSON@PARSE").await.is_ok(),
            "directly referenced module word should remain visible"
        );
        assert!(
            interp.execute("'[1]' JSON@STRINGIFY").await.is_err(),
            "unreferenced module word should be hidden"
        );
        let imported = interp.import_table.modules.get("JSON").unwrap();
        assert!(
            !imported.import_all_public,
            "UNIMPORT should shrink a full import to explicit referenced selections"
        );
        assert!(imported.imported_words.contains("PARSE"));
        assert!(!imported.imported_words.contains("STRINGIFY"));
    }

    #[tokio::test]
    async fn unimport_only_hides_selected_unreferenced_words_after_full_import() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT").await.unwrap();

        interp
            .execute("'json' [ 'stringify' ] UNIMPORT-ONLY")
            .await
            .unwrap();

        assert!(interp.execute("'[1]' JSON@PARSE").await.is_ok());
        assert!(interp.execute("'[1]' JSON@STRINGIFY").await.is_err());
        let imported = interp.import_table.modules.get("JSON").unwrap();
        assert!(
            !imported.import_all_public,
            "UNIMPORT-ONLY from a full import should expand to explicit selections"
        );
    }

    #[tokio::test]
    async fn unimport_only_rejects_user_referenced_module_words() {
        let mut interp = Interpreter::new();
        interp.execute("'json' IMPORT").await.unwrap();
        interp
            .execute("{ JSON@PARSE } 'USE-PARSE' DEF")
            .await
            .unwrap();

        let result = interp.execute("'json' [ 'parse' ] UNIMPORT-ONLY").await;

        assert!(result.is_err(), "referenced module word should be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Cannot unimport JSON@PARSE"), "got: {msg}");
        assert!(msg.contains("DEMO@USE-PARSE"), "got: {msg}");
    }

    #[tokio::test]
    async fn del_cannot_destroy_module_dictionary_or_words_even_forced() {
        let mut interp = Interpreter::new();
        let before_import = interp.execute("'JSON' DEL").await;
        assert!(
            before_import.is_err(),
            "known modules cannot be deleted before import"
        );
        assert!(before_import
            .unwrap_err()
            .to_string()
            .contains("Cannot delete module dictionary JSON"));

        interp.execute("'music' IMPORT").await.unwrap();

        for code in ["'MUSIC' DEL", "! 'MUSIC' DEL"] {
            let result = interp.execute(code).await;
            assert!(result.is_err(), "{code} should be rejected");
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Cannot delete module dictionary MUSIC"),
                "{code} should suggest UNIMPORT"
            );
            assert!(interp.module_vocabulary.contains_key("MUSIC"));
        }

        for code in [
            "'MUSIC@PLAY' DEL",
            "! 'MUSIC@PLAY' DEL",
            "'PLAY' DEL",
            "! 'PLAY' DEL",
        ] {
            let result = interp.execute(code).await;
            assert!(result.is_err(), "{code} should be rejected");
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Cannot delete module word MUSIC@PLAY"),
                "{code} should suggest UNIMPORT-ONLY"
            );
            assert!(interp.module_vocabulary.contains_key("MUSIC"));
        }
    }
}
