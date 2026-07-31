//! Generated Word registry and the spec ↔ runtime equivalence checks that make
//! `spec/words.json` the single source of truth for Word metadata (migration
//! plan Phase 3).
//!
//! [`word_registry`] is generated from `spec/words.json` by
//! `scripts/generate-word-registry.mjs` (CI drift-checks it with
//! `npm run word-registry:check`). This module adds tests proving the generated
//! projection agrees with the runtime's hand-written tables — the builtin
//! registration, the alias resolver, and the executor dispatch — so a drift in
//! either surface fails the build. Nothing consumes the registry at runtime
//! yet: a later phase routes the runtime onto it and deletes the hand-written
//! duplication (the ~1300-line `BUILTIN_SPECS`, the hand-authored
//! `BuiltinExecutorKey` enum, and `core_word_aliases.rs`).

mod word_registry;

pub use word_registry::{
    Arity, Consumption, Determinism, Family, GeneratedWord, NilPolicy, Purity, WordId,
    GENERATED_WORDS,
};

#[cfg(test)]
mod tests {
    use super::GENERATED_WORDS;
    use crate::builtins::{builtin_specs, lookup_builtin_spec};
    use crate::core_word_aliases::canonicalize_core_word_name;
    use std::collections::BTreeSet;

    // The three Words the runtime models as directives/modifiers handled
    // positionally in the execution loop, so they carry no `BuiltinExecutorKey`:
    // the EAT/KEEP consumption modifiers and the VENT lazy fallback. words.json
    // still names their executor form, which is what the generated registry
    // records.
    const DIRECTIVE_EXECUTOR_KEYS: &[&str] = &[
        "SetConsumptionConsume",
        "SetConsumptionKeep",
        "LazyNextUnitFallback",
    ];

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

    #[test]
    fn generated_executor_keys_match_runtime_dispatch() {
        for word in GENERATED_WORDS {
            let spec = lookup_builtin_spec(word.name)
                .unwrap_or_else(|| panic!("runtime has no spec for {}", word.name));
            match spec.executor_key {
                Some(key) => assert_eq!(
                    format!("{key:?}"),
                    word.executor_key,
                    "executor key mismatch for {}",
                    word.name
                ),
                None => assert!(
                    DIRECTIVE_EXECUTOR_KEYS.contains(&word.executor_key),
                    "{} has no runtime executor key but is not a known directive ({})",
                    word.name,
                    word.executor_key
                ),
            }
        }
    }
}
