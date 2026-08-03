//! Resolution laws for `crate::interpreter::resolve_word`.
//!
//! LANG.DICTIONARY.RESOLUTION: "The dictionary has two tiers. **Core** holds
//! the 57 canonical Words and is sealed: a Core name cannot be redefined or
//! deleted. **User** holds definitions made by `DEF`. Resolution is a
//! deterministic function of the normalized name and the current dictionary,
//! and User never shadows Core." And: "Those two tiers are the whole
//! dictionary: a name resolves in Core or in User."
//!
//! This file used to test a different dictionary. It had named user
//! dictionaries addressed by `DICT@WORD`, `USER@DICT@WORD` and
//! `DICT@USER@DICT@WORD`; a bare name fell through three stages, and a name
//! held by several dictionaries was collapsed by content identity or reported
//! ambiguous. None of it was reachable from the language — no Word changes the
//! active dictionary, so every `DEF` wrote to the same one — and the clause
//! says there are two tiers. The laws below are what is left once there are.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    fn define(interp: &mut Interpreter, name: &str, definition: &str) {
        let tokens = crate::tokenizer::tokenize(definition)
            .unwrap_or_else(|e| panic!("failed to tokenize {name}: {e}"));
        crate::interpreter::execute_def::op_def_inner(interp, name, &tokens)
            .unwrap_or_else(|e| panic!("failed to define {name}: {e}"));
    }

    #[tokio::test]
    async fn a_core_name_resolves_to_core() {
        let interp = Interpreter::new();
        let (name, def) = interp
            .resolve_word_entry_readonly("ADD")
            .expect("ADD is a Core Word");
        assert_eq!(name, "ADD");
        assert!(def.is_builtin);
    }

    #[tokio::test]
    async fn a_user_name_resolves_to_user_by_its_bare_name() {
        let mut interp = Interpreter::new();
        define(&mut interp, "INC", "1 ADD");

        let (name, def) = interp
            .resolve_word_entry_readonly("INC")
            .expect("INC was defined");
        assert_eq!(name, "INC", "a name is the whole address");
        assert!(!def.is_builtin);
    }

    #[tokio::test]
    async fn resolution_is_case_insensitive() {
        let mut interp = Interpreter::new();
        define(&mut interp, "INC", "1 ADD");
        for spelling in ["INC", "inc", "Inc"] {
            assert_eq!(
                interp
                    .resolve_word_entry_readonly(spelling)
                    .map(|(n, _)| n)
                    .as_deref(),
                Some("INC"),
                "{spelling} must normalize to INC"
            );
        }
    }

    #[tokio::test]
    async fn user_never_shadows_core() {
        // The seal is enforced at definition time, so the shadowing case is
        // unconstructible rather than merely losing the lookup.
        let mut interp = Interpreter::new();
        let tokens = crate::tokenizer::tokenize("1 ADD").expect("tokenize");
        let result = crate::interpreter::execute_def::op_def_inner(&mut interp, "ADD", &tokens);
        assert!(result.is_err(), "Core is sealed against redefinition");

        let (_, def) = interp.resolve_word_entry_readonly("ADD").expect("ADD");
        assert!(def.is_builtin, "ADD still resolves to Core");
    }

    #[tokio::test]
    async fn a_qualified_path_is_not_a_name() {
        // `DICT@WORD` addressed a tier that no longer exists. It is now just a
        // name that nothing holds.
        let mut interp = Interpreter::new();
        define(&mut interp, "INC", "1 ADD");

        for path in [
            "EXAMPLE@INC",
            "USER@EXAMPLE@INC",
            "CORE@ADD",
            "DICT@CORE@ADD",
        ] {
            assert!(
                interp.resolve_word_entry_readonly(path).is_none(),
                "{path} must not resolve"
            );
        }
    }

    #[tokio::test]
    async fn a_bare_name_is_never_ambiguous() {
        // Ambiguity was a consequence of several dictionaries holding a name.
        // With one User tier a name is held or it is not.
        let mut interp = Interpreter::new();
        define(&mut interp, "INC", "1 ADD");
        assert!(interp.check_ambiguity("INC").is_empty());
        assert!(interp.check_ambiguity("ADD").is_empty());
        assert!(interp.check_ambiguity("NOPE").is_empty());
    }

    #[tokio::test]
    async fn an_unknown_name_does_not_resolve() {
        let interp = Interpreter::new();
        assert!(interp.resolve_word_entry_readonly("NO-SUCH-WORD").is_none());
    }

    #[tokio::test]
    async fn a_redefinition_replaces_the_user_entry() {
        let mut interp = Interpreter::new();
        define(&mut interp, "INC", "1 ADD");
        define(&mut interp, "INC", "2 ADD");

        interp.execute("5 INC").await.expect("INC runs");
        assert_eq!(
            format!("{}", interp.get_stack().last().expect("a result")),
            "7/1",
            "the second definition is the one that resolves"
        );
    }
}
