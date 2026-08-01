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
    async fn user_defined_word_is_contrib_tier() {
        let mut interp = Interpreter::new();
        interp.execute("{ 1 } 'X' DEF").await.unwrap();
        let def = interp.user_words.get("X").unwrap();
        assert_eq!(def.tier, Tier::Contrib);
        assert_eq!(def.stability, Stability::Stable);
    }

    #[test]
    fn now_is_not_in_builtin_specs() {
        assert!(builtin_specs().iter().all(|s| s.name != "NOW"));
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
