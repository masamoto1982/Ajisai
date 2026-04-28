#[cfg(test)]
mod tests {
    use crate::builtins::builtin_specs;
    use crate::interpreter::Interpreter;
    use crate::types::{Capabilities, Stability, Tier};

    #[tokio::test]
    async fn core_words_have_expected_attributes() {
        let interp = Interpreter::new();
        let add = interp.core_vocabulary.get("ADD").unwrap();
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

        assert!(!interp.core_vocabulary.contains_key("'"));
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

    #[test]
    fn math_words_are_not_in_builtin_specs() {
        for name in [
            "SQRT", "SQRT_EPS", "SQRT-EPS", "INTERVAL", "LOWER", "UPPER", "WIDTH", "IS_EXACT",
            "IS-EXACT",
        ] {
            assert!(
                builtin_specs().iter().all(|s| s.name != name),
                "{} unexpectedly present in BUILTIN_SPECS",
                name
            );
        }
    }

    #[tokio::test]
    async fn deprecated_sqrt_eps_alias_redirects_to_hyphen_form() {
        let mut interp = Interpreter::new();
        interp.execute("2 1/100 SQRT_EPS").await.unwrap();
        let warning = interp.collect_output();
        assert!(warning.contains("Warning: 'SQRT_EPS' is deprecated."));
        assert!(warning.contains("MATH@SQRT-EPS"));

        let from_alias = interp.stack.pop().unwrap();
        interp.execute("'math' IMPORT").await.unwrap();
        interp.execute("2 1/100 MATH@SQRT-EPS").await.unwrap();
        let from_qualified = interp.stack.pop().unwrap();
        assert_eq!(from_alias, from_qualified);
    }

    #[tokio::test]
    async fn deprecated_is_exact_alias_redirects_to_hyphen_form() {
        let mut interp = Interpreter::new();
        interp.execute("4 IS_EXACT").await.unwrap();
        let warning = interp.collect_output();
        assert!(warning.contains("Warning: 'IS_EXACT' is deprecated."));
        assert!(warning.contains("MATH@IS-EXACT"));

        let from_alias = interp.stack.pop().unwrap();
        interp.execute("'math' IMPORT").await.unwrap();
        interp.execute("4 MATH@IS-EXACT").await.unwrap();
        let from_qualified = interp.stack.pop().unwrap();
        assert_eq!(from_alias, from_qualified);
    }

    #[tokio::test]
    async fn deprecated_sqrt_alias_warns_and_matches_math_sqrt() {
        let mut interp = Interpreter::new();
        interp.execute("4 SQRT").await.unwrap();
        let warning = interp.collect_output();
        assert!(warning.contains("Warning: 'SQRT' is deprecated."));

        let sqrt_from_alias = interp.stack.pop().unwrap();
        interp.execute("'math' IMPORT").await.unwrap();
        interp.execute("4 MATH@SQRT").await.unwrap();
        let sqrt_qualified = interp.stack.pop().unwrap();
        assert_eq!(sqrt_from_alias, sqrt_qualified);
    }

    #[tokio::test]
    async fn imported_math_does_not_warn() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 4 SQRT 2 MATH@SQRT")
            .await
            .unwrap();
        let out = interp.collect_output();
        assert!(!out.contains("deprecated"));
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
