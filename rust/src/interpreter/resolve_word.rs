use crate::types::WordDefinition;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use super::{DictionaryDependencyInfo, Interpreter};

/// Outcome of resolving a bare name against the user dictionaries (the final
/// fallback stage of `resolve_short_name`). Section 8.6 makes user words
/// content-addressed, so a bare name that matches several dictionaries is only
/// ambiguous when those matches carry *distinct* content identities.
pub(crate) enum UserBareOutcome {
    /// No user dictionary holds the name.
    None,
    /// Every match collapses to a single content identity — the same word.
    /// Carries the display fq-name (lowest `registration_order`) and its body.
    Unique(String, Arc<WordDefinition>),
    /// Two or more distinct content identities share the name — a true
    /// ambiguity. Carries every matching fq-name for the diagnostic.
    Ambiguous(Vec<String>),
}

impl Interpreter {
    pub(crate) fn split_path(name: &str) -> (Vec<String>, String) {
        let parts: Vec<String> = name.split('@').map(|s| s.to_uppercase()).collect();
        if parts.len() == 1 {
            (vec![], parts[0].clone())
        } else {
            let word = parts.last().unwrap().clone();
            let layers = parts[..parts.len() - 1].to_vec();
            (layers, word)
        }
    }

    pub(crate) fn split_qualified_name(&self, name: &str) -> Option<(String, String)> {
        let (layers, word) = Self::split_path(name);
        if layers.len() == 1 {
            Some((layers[0].clone(), word))
        } else if layers.is_empty() {
            None
        } else {
            Some((layers.last().unwrap().clone(), word))
        }
    }

    pub(crate) fn user_dictionary_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.user_dictionaries.keys().cloned().collect();
        names.sort();
        names
    }

    pub(crate) fn user_dictionary_words(
        &self,
        dictionary_name: &str,
    ) -> Vec<(String, Arc<WordDefinition>)> {
        self.user_dictionaries
            .get(&dictionary_name.to_uppercase())
            .map(|dict| {
                dict.words
                    .iter()
                    .map(|(name, def)| (name.clone(), def.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn sync_user_words_cache(&mut self) {
        self.user_words = self
            .user_dictionaries
            .get("EXAMPLE")
            .map(|dict| dict.words.clone())
            .unwrap_or_default();
    }

    fn is_module_word_imported(&self, module_name: &str, short_name: &str) -> bool {
        self.import_table
            .modules
            .get(module_name)
            .map(|m| m.import_all_public || m.imported_words.contains(short_name))
            .unwrap_or(false)
    }

    pub(crate) fn resolve_short_name(&self, name: &str) -> Option<(String, Arc<WordDefinition>)> {
        let upper = name.to_uppercase();

        if let Some(def) = self.core_vocabulary.get(&upper) {
            return Some((upper, def.clone()));
        }

        for (module_name, module) in &self.module_vocabulary {
            if !self.is_module_word_imported(module_name, &upper) {
                continue;
            }
            let qualified = format!("{}@{}", module_name, upper);
            if let Some(def) = module.words.get(&qualified) {
                return Some((qualified, def.clone()));
            }
        }

        // Section 8.6: a bare name resolves through the owning dictionary's
        // words before any other user dictionary, so an imported word group is
        // self-referential regardless of which other dictionaries are loaded.
        if let Some(owning) = &self.owning_dictionary_context {
            if let Some(dict) = self.user_dictionaries.get(owning) {
                if let Some(def) = dict.words.get(&upper) {
                    return Some((format!("{}@{}", owning, upper), def.clone()));
                }
            }
        }

        // Final fallback: other user dictionaries. Section 8.6 — group matches
        // by content identity so that a name shared across dictionaries resolves
        // when (and only when) the matches are the same word.
        match self.resolve_user_bare(&upper) {
            UserBareOutcome::Unique(name, def) => Some((name, def)),
            // A true ambiguity resolves to None here; the caller's error path
            // (`check_ambiguity`) turns that into the "qualified path" diagnostic.
            UserBareOutcome::Ambiguous(_) => None,
            UserBareOutcome::None => None,
        }
    }

    /// Resolve a bare (already uppercased) name against the user dictionaries by
    /// content identity. Shared by `resolve_short_name`'s final fallback and
    /// `check_ambiguity` so the two never drift apart.
    pub(crate) fn resolve_user_bare(&self, upper: &str) -> UserBareOutcome {
        let mut matches: Vec<(String, Arc<WordDefinition>, u64)> = Vec::new();
        for (dict_name, dict) in &self.user_dictionaries {
            if let Some(def) = dict.words.get(upper) {
                matches.push((
                    format!("{}@{}", dict_name, upper),
                    def.clone(),
                    def.registration_order,
                ));
            }
        }

        if matches.is_empty() {
            return UserBareOutcome::None;
        }

        // Stable display order: the earliest registration represents the group.
        matches.sort_by_key(|(_, _, order)| *order);

        // Collapse the matches by their content identity (Section 8.6).
        let mut distinct: HashSet<String> = HashSet::new();
        let mut any_missing_identity = false;
        for (fq, _, _) in &matches {
            match self.word_identity(fq) {
                Some(id) => {
                    distinct.insert(id.clone());
                }
                // Conservative fallback: a match without a computed identity
                // (e.g. before a quiescent recompute) is not treated as
                // divergent — we fall back to the historical earliest-registered
                // representative rather than raise a spurious ambiguity. At
                // quiescent points identities are fresh, so this is not hit in
                // practice.
                None => any_missing_identity = true,
            }
        }

        if any_missing_identity || distinct.len() <= 1 {
            let (name, def, _) = matches.into_iter().next().unwrap();
            return UserBareOutcome::Unique(name, def);
        }

        UserBareOutcome::Ambiguous(matches.into_iter().map(|(fq, _, _)| fq).collect())
    }

    pub(crate) fn check_ambiguity(&self, name: &str) -> Vec<String> {
        let upper = name.to_uppercase();

        // Core / Module words win the bare-name ladder and are never ambiguous.
        if self.core_vocabulary.contains_key(&upper) {
            return vec![];
        }

        match self.resolve_user_bare(&upper) {
            UserBareOutcome::Ambiguous(paths) => paths,
            UserBareOutcome::Unique(..) | UserBareOutcome::None => vec![],
        }
    }

    pub(crate) fn resolve_word_entry_readonly(
        &self,
        name: &str,
    ) -> Option<(String, Arc<WordDefinition>)> {
        let canonical_name = crate::core_word_aliases::canonicalize_core_word_name(name);
        let name = canonical_name.as_ref();
        let (layers, word) = Self::split_path(name);

        match layers.len() {
            0 => self.resolve_short_name(name),
            1 => {
                let ns = &layers[0];
                if ns == "CORE" {
                    return self
                        .core_vocabulary
                        .get(&word)
                        .cloned()
                        .map(|def| (word.clone(), def));
                }
                if let Some(module_dict) = self.module_vocabulary.get(ns.as_str()) {
                    let qualified = format!("{}@{}", ns, word);
                    if self.is_module_word_imported(ns, &word) {
                        if let Some(def) = module_dict.words.get(&qualified) {
                            return Some((qualified, def.clone()));
                        }
                    }
                    return None;
                }
                if let Some(user_dict) = self.user_dictionaries.get(ns.as_str()) {
                    if let Some(def) = user_dict.words.get(&word) {
                        return Some((format!("{}@{}", ns, word), def.clone()));
                    }
                }
                None
            }
            2 => {
                let first = &layers[0];
                let second = &layers[1];
                if first == "USER" {
                    if let Some(user_dict) = self.user_dictionaries.get(second.as_str()) {
                        if let Some(def) = user_dict.words.get(&word) {
                            return Some((format!("{}@{}", second, word), def.clone()));
                        }
                    }
                } else if first == "DICT" {
                    if second == "CORE" {
                        return self
                            .core_vocabulary
                            .get(&word)
                            .cloned()
                            .map(|def| (word.clone(), def));
                    }
                    if let Some(module_dict) = self.module_vocabulary.get(second.as_str()) {
                        let qualified = format!("{}@{}", second, word);
                        if self.is_module_word_imported(second, &word) {
                            if let Some(def) = module_dict.words.get(&qualified) {
                                return Some((qualified, def.clone()));
                            }
                        }
                    }
                }
                None
            }
            3 => {
                let first = &layers[0];
                let second = &layers[1];
                let third = &layers[2];
                if first == "DICT" && second == "USER" {
                    if let Some(user_dict) = self.user_dictionaries.get(third.as_str()) {
                        if let Some(def) = user_dict.words.get(&word) {
                            return Some((format!("{}@{}", third, word), def.clone()));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn resolve_word_entry(
        &mut self,
        name: &str,
    ) -> Option<(String, Arc<WordDefinition>)> {
        let canonical_name = crate::core_word_aliases::canonicalize_core_word_name(name);
        let name = canonical_name.as_ref();
        if let Some(cached_name) = self.lookup_resolve_cache(name) {
            let (layers, word) = Self::split_path(&cached_name);
            if layers.is_empty() {
                if let Some(def) = self.core_vocabulary.get(&word).cloned() {
                    return Some((word, def));
                }
            } else if layers.len() == 1 {
                let ns = &layers[0];
                if let Some(dict) = self.user_dictionaries.get(ns.as_str()) {
                    if let Some(def) = dict.words.get(&word).cloned() {
                        return Some((format!("{}@{}", ns, word), def));
                    }
                }
                if let Some(module) = self.module_vocabulary.get(ns.as_str()) {
                    let qualified = format!("{}@{}", ns, word);
                    if let Some(def) = module.words.get(&qualified).cloned() {
                        return Some((qualified, def));
                    }
                }
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
        self.dictionary_dependencies.clear();

        let mut all_words: Vec<(String, Arc<WordDefinition>)> = Vec::new();

        for (dict_name, dict) in &self.user_dictionaries {
            for (name, def) in &dict.words {
                all_words.push((format!("{}@{}", dict_name, name), Arc::clone(def)));
            }
        }

        let mut dictionary_edges: HashMap<String, HashSet<String>> = HashMap::new();

        for (word_name, word_def) in &all_words {
            // Section 8.6: scan each word's references through its own dictionary
            // first, so dependents are recorded against the word's own
            // dictionary rather than a same-named word elsewhere.
            self.owning_dictionary_context =
                self.split_qualified_name(word_name).map(|(dict, _)| dict);
            let mut dependencies = HashSet::new();
            for line in word_def.lines.iter() {
                for token in line.body_tokens.iter() {
                    if let crate::types::Token::Symbol(s) = token {
                        let upper_s = crate::core_word_aliases::canonicalize_core_word_name(s);
                        if let Some((resolved_name, resolved_def)) =
                            self.resolve_word_entry(&upper_s)
                        {
                            if !resolved_def.is_builtin || resolved_name.contains('@') {
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
            if let Some((dict_name, short_name)) = self.split_qualified_name(word_name) {
                if let Some(dict) = self.user_dictionaries.get_mut(&dict_name) {
                    if let Some(def) = dict.words.get_mut(&short_name) {
                        Arc::make_mut(def).dependencies = dependencies.clone();
                    }
                }
                let edge_set = dictionary_edges.entry(dict_name.clone()).or_default();
                for dep in &dependencies {
                    if let Some((dep_dict, _)) = self.split_qualified_name(dep) {
                        if dep_dict != dict_name {
                            edge_set.insert(dep_dict);
                        }
                    }
                }
            }
        }

        self.owning_dictionary_context = None;

        for dict in self.user_dictionaries.keys() {
            self.dictionary_dependencies
                .entry(dict.clone())
                .or_default();
        }
        for dict in self.module_vocabulary.keys() {
            self.dictionary_dependencies
                .entry(dict.clone())
                .or_default();
        }
        for (from, tos) in dictionary_edges {
            for to in tos {
                self.dictionary_dependencies
                    .entry(from.clone())
                    .or_insert_with(DictionaryDependencyInfo::default)
                    .depends_on
                    .insert(to.clone());
                self.dictionary_dependencies
                    .entry(to)
                    .or_insert_with(DictionaryDependencyInfo::default)
                    .depended_by
                    .insert(from.clone());
            }
        }

        self.sync_user_words_cache();
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

    /// Authoritative full-scan computation of the direct dependents of
    /// `word_name`. This is the ground truth the maintained `dependents` index
    /// mirrors; it is retained only as the debug cross-check for
    /// `collect_dependents` and is dead-code-eliminated from the release hot
    /// path.
    fn collect_dependents_by_scan(&self, word_name: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        for (dict_name, dict) in &self.user_dictionaries {
            for (name, def) in &dict.words {
                if def.dependencies.contains(word_name) {
                    result.insert(format!("{}@{}", dict_name, name));
                }
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
