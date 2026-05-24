//! Tests for the GUI-facing module catalog and detailed import-state restore
//! helpers added for selector/word activation toggling.

#[cfg(test)]
mod tests {
    use crate::interpreter::modules::{
        available_module_names, module_catalog_words, restore_import_entry,
    };
    use crate::interpreter::Interpreter;

    #[test]
    fn available_modules_cover_all_specced_modules() {
        let names = available_module_names();
        for expected in ["MUSIC", "JSON", "IO", "TIME", "CRYPTO", "ALGO", "MATH", "SERIAL"] {
            assert!(
                names.contains(&expected),
                "available module list should include {}",
                expected
            );
        }
    }

    #[test]
    fn catalog_lists_words_regardless_of_import_state() {
        // JSON has never been imported here, yet its full catalog is available.
        let catalog = module_catalog_words("JSON").expect("JSON catalog");
        let names: Vec<&str> = catalog.iter().map(|w| w.short_name).collect();
        assert!(names.contains(&"PARSE"));
        assert!(names.contains(&"STRINGIFY"));
        assert!(!catalog.iter().any(|w| w.is_sample && w.short_name == "PARSE"));
    }

    #[test]
    fn catalog_unknown_module_is_none() {
        assert!(module_catalog_words("NOPE").is_none());
    }

    #[tokio::test]
    async fn restore_import_entry_reinstates_partial_import() {
        let mut interp = Interpreter::new();
        // Simulate a persisted partial import: only PARSE is active.
        assert!(restore_import_entry(
            &mut interp,
            "JSON",
            false,
            vec!["PARSE".to_string()],
            vec![],
        ));

        let entry = interp.import_table.modules.get("JSON").expect("JSON entry");
        assert!(!entry.import_all_public);
        assert!(entry.imported_words.contains("PARSE"));
        assert!(!entry.imported_words.contains("STRINGIFY"));

        assert!(
            interp.execute("'[1]' JSON@PARSE").await.is_ok(),
            "restored active word should resolve"
        );
    }

    #[test]
    fn restore_import_entry_uppercases_selectors() {
        let mut interp = Interpreter::new();
        assert!(restore_import_entry(
            &mut interp,
            "json",
            false,
            vec!["parse".to_string()],
            vec![],
        ));
        let entry = interp.import_table.modules.get("JSON").expect("JSON entry");
        assert!(entry.imported_words.contains("PARSE"));
    }

    #[test]
    fn restore_import_entry_unknown_module_returns_false() {
        let mut interp = Interpreter::new();
        assert!(!restore_import_entry(&mut interp, "NOPE", true, vec![], vec![]));
    }
}
