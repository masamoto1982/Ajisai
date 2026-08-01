//! Generated Word registry — the runtime's supply of Word contract metadata.
//!
//! [`word_registry`] is generated from `spec/words.json` by
//! `scripts/generate-word-registry.mjs` (CI drift-checks it with
//! `npm run word-registry:check`). It is no longer a parallel projection kept
//! honest by equivalence tests: the runtime *reads* it. Dispatch keys off
//! [`WordId`], the NIL guard reads [`GeneratedWord::nil_policy`], and the
//! Coreword registry takes stack arity, purity and determinism from here, so
//! `spec/words.json` is the only place those facts are written down.
//!
//! What is still hand-written on `BuiltinSpec` is prose and runtime-local
//! classification — documentation text, `category`, `partiality`,
//! `safety_level`, `safe_preview` — none of which the specification declares.
//! No canonical fact is written down twice.

mod word_registry;

pub use word_registry::{
    Arity, Consumption, Determinism, Family, GeneratedWord, NilPolicy, Partiality, Purity, WordId,
    GENERATED_WORDS,
};

/// The declared contract for a Word, by canonical name.
///
/// Built-in Words form a single flat namespace, so this is an exact match on
/// the canonical (post-alias) name. Callers holding a raw source token should
/// canonicalize first.
pub fn generated_word(name: &str) -> Option<&'static GeneratedWord> {
    GENERATED_WORDS.iter().find(|word| word.name == name)
}

/// The contract vocabularies serialize as the canonical spec strings, so the
/// wire form of a Word's contract is the specification's own spelling rather
/// than a Rust-side renaming of it. `as_spec_str` is generated from the schema
/// enum, so a new admitted value reaches the wire without a second edit here.
macro_rules! serialize_as_spec_str {
    ($($ty:ty),+ $(,)?) => {
        $(impl serde::Serialize for $ty {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_spec_str())
            }
        })+
    };
}

serialize_as_spec_str!(
    Family,
    Consumption,
    NilPolicy,
    Partiality,
    Purity,
    Determinism
);

#[cfg(test)]
mod tests {
    use super::{generated_word, GENERATED_WORDS};
    use crate::builtins::builtin_specs;
    use crate::core_word_aliases::canonicalize_core_word_name;
    use std::collections::BTreeSet;

    #[test]
    fn generated_inventory_matches_the_runtime_builtin_registration() {
        let generated: BTreeSet<&str> = GENERATED_WORDS.iter().map(|word| word.name).collect();
        let runtime: BTreeSet<&str> = builtin_specs().iter().map(|spec| spec.name).collect();
        assert_eq!(generated, runtime);
    }

    #[test]
    fn generated_aliases_resolve_to_their_canonical_word() {
        let mut alias_count = 0;
        for word in GENERATED_WORDS {
            for &alias in word.aliases {
                alias_count += 1;
                assert_eq!(
                    canonicalize_core_word_name(alias).as_ref(),
                    word.name,
                    "alias {alias} should canonicalize to {}",
                    word.name
                );
            }
        }
        assert_eq!(alias_count, 16, "spec/words.json declares 16 aliases");
    }

    /// The executor-key equivalence test this module used to carry is gone: it
    /// compared the generated key against a hand-written `BuiltinExecutorKey`,
    /// and that enum no longer exists — the runtime dispatches on `WordId`
    /// directly, so a Word without an executor arm is a compile error rather
    /// than a test failure. What remains worth asserting is that name lookup,
    /// the path both the guard and the registry take, reaches every Word.
    #[test]
    fn every_generated_word_is_reachable_by_name() {
        for word in GENERATED_WORDS {
            let found = generated_word(word.name)
                .unwrap_or_else(|| panic!("{} is not reachable by name", word.name));
            assert_eq!(found.id, word.id);
        }
        assert!(generated_word("__AJISAI_NO_SUCH_WORD__").is_none());
    }
}
