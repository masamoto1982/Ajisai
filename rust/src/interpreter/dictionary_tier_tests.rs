//! Test suite for dictionary tier classification.

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

        assert!(!interp.core_vocabulary.contains_key("FRAME"));

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
            .get("EXAMPLE")
            .and_then(|d| d.words.get("X"))
            .unwrap();
        assert_eq!(def.tier, Tier::Contrib);
        assert_eq!(def.stability, Stability::Stable);
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
