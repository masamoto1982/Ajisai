#[cfg(test)]
mod tests {
    use crate::builtins::builtin_specs;
    use crate::interpreter::Interpreter;
    use crate::types::{Capabilities, Stability, Tier};

    #[tokio::test]
    async fn core_words_have_expected_attributes() {
        let interp = Interpreter::new();
        let add = interp.core_vocabulary.get("+").unwrap();
        assert_eq!(add.tier, Tier::Core);
        assert_eq!(add.stability, Stability::Stable);
        assert_eq!(add.capabilities, Capabilities::PURE);

        let map = interp.core_vocabulary.get("MAP").unwrap();
        assert_eq!(map.tier, Tier::Core);
        assert_eq!(map.stability, Stability::Stable);
        assert_eq!(map.capabilities, Capabilities::PURE);

        let def = interp.core_vocabulary.get("DEF").unwrap();
        assert_eq!(def.tier, Tier::Core);
        assert_eq!(def.stability, Stability::Stable);
        assert_eq!(def.capabilities, Capabilities::MUTATES_DICT);

        let frame = interp.core_vocabulary.get("FRAME").unwrap();
        assert_eq!(
            frame.capabilities,
            Capabilities::PURE.union(Capabilities::INPUT_HELPER)
        );

        let quote = interp.core_vocabulary.get("'").unwrap();
        assert_eq!(quote.capabilities, Capabilities::INPUT_HELPER);
    }

    #[tokio::test]
    async fn standard_module_words_have_expected_attributes() {
        let mut interp = Interpreter::new();
        interp.execute("'time' IMPORT").await.unwrap();
        interp.execute("'crypto' IMPORT").await.unwrap();

        let time_now = interp
            .module_vocabulary
            .get("TIME")
            .and_then(|m| m.words.get("TIME@NOW"))
            .unwrap();
        assert_eq!(time_now.tier, Tier::Standard);
        assert_eq!(time_now.capabilities, Capabilities::TIME);

        let csprng = interp
            .module_vocabulary
            .get("CRYPTO")
            .and_then(|m| m.words.get("CRYPTO@CSPRNG"))
            .unwrap();
        assert_eq!(csprng.tier, Tier::Standard);
        assert_eq!(
            csprng.capabilities,
            Capabilities::RANDOM | Capabilities::CRYPTO
        );
    }

    #[tokio::test]
    async fn user_defined_word_is_contrib_tier() {
        let mut interp = Interpreter::new();
        interp.execute("{ 1 } 'X' DEF").await.unwrap();
        let def = interp
            .user_dictionaries
            .get("DEMO")
            .and_then(|d| d.words.get("X"))
            .unwrap();
        assert_eq!(def.tier, Tier::Contrib);
        assert_eq!(def.stability, Stability::Stable);
    }

    #[tokio::test]
    async fn deprecated_sort_alias_warns_and_matches_algo_sort() {
        let mut interp = Interpreter::new();
        interp.execute("[ 3 1 2 ] SORT").await.unwrap();
        let warning = interp.collect_output();
        assert!(warning.contains("Warning: 'SORT' is deprecated."));

        let sort_from_alias = interp.stack.pop().unwrap();
        interp.execute("'algo' IMPORT").await.unwrap();
        interp.execute("[ 3 1 2 ] ALGO@SORT").await.unwrap();
        let sort_qualified = interp.stack.pop().unwrap();
        assert_eq!(sort_from_alias, sort_qualified);
    }

    #[test]
    fn now_is_not_in_builtin_specs() {
        assert!(builtin_specs().iter().all(|s| s.name != "NOW"));
    }

    #[tokio::test]
    async fn imported_sort_and_qualified_sort_do_not_warn() {
        let mut interp = Interpreter::new();
        interp
            .execute("'algo' IMPORT [ 3 1 2 ] SORT [ 3 1 2 ] ALGO@SORT")
            .await
            .unwrap();
        let out = interp.collect_output();
        assert!(!out.contains("deprecated"));
    }

    #[test]
    fn capabilities_bit_operations_work() {
        assert_eq!(Capabilities::PURE & Capabilities::IO, Capabilities::empty());
        let joined = Capabilities::IO | Capabilities::TIME;
        assert!(joined.contains(Capabilities::IO));
        assert!(joined.contains(Capabilities::TIME));

        let helper = Capabilities::PURE.union(Capabilities::INPUT_HELPER);
        assert!(helper.contains(Capabilities::INPUT_HELPER));
        assert!(helper.contains(Capabilities::PURE));
    }
}
