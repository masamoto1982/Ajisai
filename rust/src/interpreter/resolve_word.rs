use crate::types::WordDefinition;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use super::Interpreter;

impl Interpreter {
    /// Resolve a name against the dictionary LANG.DICTIONARY.RESOLUTION
    /// describes: "The dictionary has two tiers. **Core** holds the 69
    /// canonical Words and is sealed ... **User** holds definitions made by
    /// `DEF`. Resolution is a deterministic function of the normalized name and
    /// the current dictionary, and User never shadows Core." And: "Those two
    /// tiers are the whole dictionary."
    ///
    /// It used to be more than two. A `user_dictionaries` map held named user
    /// dictionaries; an `active_user_dictionary` decided which one `DEF` wrote
    /// to; an `owning_dictionary_context` gave a word's own dictionary priority;
    /// and a bare name fell through three stages, collapsing cross-dictionary
    /// matches by content identity and reporting an `Ambiguous` outcome when
    /// they disagreed. `DICT@WORD`, `USER@D@WORD` and `DICT@USER@D@WORD` paths
    /// addressed those tiers.
    ///
    /// None of it was reachable from the language: no Word changes the active
    /// dictionary, so every `DEF` wrote to the same one ("EXAMPLE"), and
    /// `user_words` was already maintained as a flat mirror of it. The tiers
    /// were structure without a way to observe them, and the clause says there
    /// are two.
    pub(crate) fn resolve_short_name(&self, name: &str) -> Option<(String, Arc<WordDefinition>)> {
        let upper = name.to_uppercase();

        // Core first: User never shadows Core.
        if let Some(def) = self.core_vocabulary.get(&upper) {
            return Some((upper, def.clone()));
        }

        self.user_words.get(&upper).map(|def| (upper, def.clone()))
    }

    /// A bare name is never ambiguous: there is one User tier, so a name is in
    /// it or it is not. Retained as the single place the answer is stated.
    pub(crate) fn check_ambiguity(&self, _name: &str) -> Vec<String> {
        vec![]
    }

    pub(crate) fn resolve_word_entry_readonly(
        &self,
        name: &str,
    ) -> Option<(String, Arc<WordDefinition>)> {
        let canonical_name = crate::core_word_aliases::canonicalize_core_word_name(name);
        self.resolve_short_name(canonical_name.as_ref())
    }

    pub(crate) fn resolve_word_entry(
        &mut self,
        name: &str,
    ) -> Option<(String, Arc<WordDefinition>)> {
        let canonical_name = crate::core_word_aliases::canonicalize_core_word_name(name);
        let name = canonical_name.as_ref();
        if let Some(cached_name) = self.lookup_resolve_cache(name) {
            if let Some(def) = self.core_vocabulary.get(&cached_name).cloned() {
                return Some((cached_name, def));
            }
            if let Some(def) = self.user_words.get(&cached_name).cloned() {
                return Some((cached_name, def));
            }
        }

        let resolved = self.resolve_word_entry_readonly(name);
        if let Some((resolved_name, def)) = resolved.clone() {
            self.store_resolve_cache(name, &resolved_name, def.registration_order);
        }
        resolved
    }

    pub(crate) fn resolve_word(&self, name: &str) -> Option<Arc<WordDefinition>> {
        self.resolve_word_entry_readonly(name).map(|(_, def)| def)
    }

    pub(crate) fn word_exists(&self, name: &str) -> bool {
        self.resolve_word(name).is_some()
    }

    pub fn rebuild_dependencies(&mut self) -> crate::error::Result<()> {
        // Quiescent recompute point (also reached after import, which does not
        // bump the dictionary epoch on its own). Invalidate the resolve cache so
        // a name that was previously cached as a single resolution cannot be
        // revived once a divergent same-named word is introduced and the bare
        // name becomes ambiguous (Section 8.6). Entries written by the scan
        // below are tagged with the fresh epoch and stay valid.
        self.bump_dictionary_epoch();

        self.dependents.clear();

        let all_words: Vec<(String, Arc<WordDefinition>)> = self
            .user_words
            .iter()
            .map(|(name, def)| (name.clone(), Arc::clone(def)))
            .collect();

        for (word_name, word_def) in &all_words {
            let mut dependencies = HashSet::new();
            for line in word_def.lines.iter() {
                for token in line.body_tokens.iter() {
                    if let crate::types::Token::Symbol(s) = token {
                        let upper_s = crate::core_word_aliases::canonicalize_core_word_name(s);
                        if let Some((resolved_name, resolved_def)) =
                            self.resolve_word_entry(&upper_s)
                        {
                            // Only User Words are dependencies: Core is sealed,
                            // so nothing can invalidate a reference to it.
                            if !resolved_def.is_builtin {
                                dependencies.insert(resolved_name.clone());
                                self.dependents
                                    .entry(resolved_name)
                                    .or_default()
                                    .insert(word_name.clone());
                            }
                        }
                    }
                }
            }
            if let Some(def) = self.user_words.get_mut(word_name) {
                Arc::make_mut(def).dependencies = dependencies;
            }
        }

        self.recompute_word_identities();
        self.gc_body_store();
        Ok(())
    }

    /// Words that directly reference `word_name`, as fully-qualified names.
    ///
    /// This reads the maintained reverse-dependency index (`self.dependents`) —
    /// an inverted index from a word to the set of words that depend on it,
    /// which `DEF`, `DEL`, and `rebuild_dependencies` keep in sync. It replaces
    /// the previous O(N) rescan of every user dictionary with an O(1) index
    /// lookup; for a redefinition or deletion that touches a word referenced
    /// across a large dictionary this turns a full-corpus walk into a single
    /// map probe.
    ///
    /// In debug builds a `debug_assert_eq!` cross-checks the index against the
    /// authoritative full scan (`collect_dependents_by_scan`) on every call, so
    /// any drift between the maintained index and ground truth is caught by the
    /// existing test suite at zero release-build cost.
    pub fn collect_dependents(&self, word_name: &str) -> HashSet<String> {
        let from_index = self.dependents.get(word_name).cloned().unwrap_or_default();
        debug_assert_eq!(
            from_index,
            self.collect_dependents_by_scan(word_name),
            "dependents index diverged from full scan for {}",
            word_name
        );
        from_index
    }

    /// The direct dependents of `word_name` *other than itself*.
    ///
    /// This is the set that decides whether a word may be redefined or deleted.
    /// A recursive word depends on itself, and once that self-edge is in the
    /// index (`rebuild_dependencies` records it, and so does a second `DEF` of
    /// an already-defined recursive word), the plain dependents set is never
    /// empty. `DEF` and `DEL` then refused with "referenced by FIB — delete
    /// those words first", naming the very word being deleted: a recursive word
    /// could not be corrected or removed by any Word in the vocabulary, only by
    /// discarding the whole User dictionary. That contradicts
    /// LANG.DICTIONARY.MUTATION, under which a User Word is redefinable.
    ///
    /// The refusal exists to protect *other* words from losing the definition
    /// they call, and a word cannot be its own such victim: redefining it
    /// replaces the body the self-call resolves through, and deleting it removes
    /// caller and callee together. So the self-edge is kept in the index — it is
    /// real, and word identity depends on it — and excluded here.
    pub fn collect_external_dependents(&self, word_name: &str) -> HashSet<String> {
        let mut dependents = self.collect_dependents(word_name);
        dependents.remove(word_name);
        dependents
    }

    /// Authoritative full-scan computation of the direct dependents of
    /// `word_name`. This is the ground truth the maintained `dependents` index
    /// mirrors; it is retained only as the debug cross-check for
    /// `collect_dependents` and is dead-code-eliminated from the release hot
    /// path.
    fn collect_dependents_by_scan(&self, word_name: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        for (name, def) in &self.user_words {
            if def.dependencies.contains(word_name) {
                result.insert(name.clone());
            }
        }
        result
    }

    /// Transitive closure of `collect_dependents`: every word that depends on
    /// `word_name` directly or through a chain of intermediate words, as
    /// fully-qualified names. Built by breadth-first traversal of the
    /// reverse-dependency index. The starting word itself is not included unless
    /// it participates in a dependency cycle. This is the impact set that a
    /// redefinition or deletion of `word_name` can affect, and the scope a later
    /// stage uses to invalidate dependent cached artifacts.
    pub fn collect_transitive_dependents(&self, word_name: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        let mut queue: VecDeque<String> = self
            .dependents
            .get(word_name)
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        while let Some(current) = queue.pop_front() {
            if !result.insert(current.clone()) {
                continue;
            }
            if let Some(next) = self.dependents.get(&current) {
                for dep in next {
                    if !result.contains(dep) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }
        result
    }
}
