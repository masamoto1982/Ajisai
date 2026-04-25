use crate::types::WordDefinition;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::{DictionaryDependencyInfo, Interpreter};

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
            .get("DEMO")
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

    fn is_module_sample_imported(&self, module_name: &str, short_name: &str) -> bool {
        self.import_table
            .modules
            .get(module_name)
            .map(|m| m.import_all_public || m.imported_samples.contains(short_name))
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

        let mut module_matches: Vec<(String, Arc<WordDefinition>, u64)> = Vec::new();
        for (module_name, dict) in &self.module_vocabulary {
            if !self.is_module_sample_imported(module_name, &upper) {
                continue;
            }
            if let Some(def) = dict.sample_words.get(&upper) {
                module_matches.push((
                    format!("{}@{}", module_name, upper),
                    def.clone(),
                    def.registration_order,
                ));
            }
        }

        let mut user_matches: Vec<(String, Arc<WordDefinition>, u64)> = Vec::new();
        for (dict_name, dict) in &self.user_dictionaries {
            if let Some(def) = dict.words.get(&upper) {
                user_matches.push((
                    format!("{}@{}", dict_name, upper),
                    def.clone(),
                    def.registration_order,
                ));
            }
        }

        if !module_matches.is_empty() && !user_matches.is_empty() {
            return None;
        }

        if !module_matches.is_empty() {
            module_matches.sort_by_key(|(_, _, order)| *order);
            let (name, def, _) = module_matches.into_iter().next().unwrap();
            return Some((name, def));
        }

        if !user_matches.is_empty() {
            user_matches.sort_by_key(|(_, _, order)| *order);
            let (name, def, _) = user_matches.into_iter().next().unwrap();
            return Some((name, def));
        }

        None
    }

    pub(crate) fn check_ambiguity(&self, name: &str) -> Vec<String> {
        let upper = name.to_uppercase();

        if self.core_vocabulary.contains_key(&upper) {
            return vec![];
        }

        let mut paths = Vec::new();
        for (module_name, dict) in &self.module_vocabulary {
            if self.is_module_sample_imported(module_name, &upper)
                && dict.sample_words.contains_key(&upper)
            {
                paths.push(format!("{}@{}", module_name, upper));
            }
        }
        for (dict_name, dict) in &self.user_dictionaries {
            if dict.words.contains_key(&upper) {
                paths.push(format!("{}@{}", dict_name, upper));
            }
        }

        if paths.len() > 1 {
            paths
        } else {
            vec![]
        }
    }

    pub(crate) fn resolve_word_entry_readonly(
        &self,
        name: &str,
    ) -> Option<(String, Arc<WordDefinition>)> {
        let canonical_name = crate::core_word_aliases::canonicalize_core_word_name(name);
        let name = canonical_name.as_str();
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
                    if self.is_module_sample_imported(ns, &word) {
                        if let Some(def) = module_dict.sample_words.get(&word) {
                            return Some((format!("{}@{}", ns, word), def.clone()));
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
                        if self.is_module_sample_imported(second, &word) {
                            if let Some(def) = module_dict.sample_words.get(&word) {
                                return Some((format!("{}@{}", second, word), def.clone()));
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
        let name = canonical_name.as_str();
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
                    if let Some(def) = module.sample_words.get(&word).cloned() {
                        return Some((format!("{}@{}", ns, word), def));
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
        self.resolve_word_entry_readonly(name)
            .map(|(_, def)| def)
            .or_else(|| {
                super::deprecated_core_aliases::lookup_deprecated_core_alias(name).and_then(
                    |alias| {
                        self.resolve_word_entry_readonly(alias.replacement_qualified)
                            .map(|(_, def)| def)
                    },
                )
            })
    }

    pub(crate) fn word_exists(&self, name: &str) -> bool {
        self.resolve_word(name).is_some()
    }

    pub fn rebuild_dependencies(&mut self) -> crate::error::Result<()> {
        self.dependents.clear();
        self.dictionary_dependencies.clear();

        let mut all_words: Vec<(String, Arc<WordDefinition>)> = Vec::new();

        for (dict_name, dict) in &self.user_dictionaries {
            for (name, def) in &dict.words {
                all_words.push((format!("{}@{}", dict_name, name), Arc::clone(def)));
            }
        }

        for (module_name, module_dict) in &self.module_vocabulary {
            for (name, def) in &module_dict.sample_words {
                all_words.push((format!("{}@{}", module_name, name), Arc::clone(def)));
            }
        }

        let mut dictionary_edges: HashMap<String, HashSet<String>> = HashMap::new();

        for (word_name, word_def) in &all_words {
            let mut dependencies = HashSet::new();
            for line in word_def.lines.iter() {
                for token in line.body_tokens.iter() {
                    if let crate::types::Token::Symbol(s) = token {
                        let upper_s = crate::core_word_aliases::canonicalize_core_word_name(s);
                        if let Some((resolved_name, resolved_def)) =
                            self.resolve_word_entry(&upper_s)
                        {
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
            if let Some((dict_name, short_name)) = self.split_qualified_name(word_name) {
                if let Some(dict) = self.user_dictionaries.get_mut(&dict_name) {
                    if let Some(def) = dict.words.get_mut(&short_name) {
                        Arc::make_mut(def).dependencies = dependencies.clone();
                    }
                }
                if let Some(module_dict) = self.module_vocabulary.get_mut(&dict_name) {
                    if let Some(def) = module_dict.sample_words.get_mut(&short_name) {
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
        Ok(())
    }

    pub fn collect_dependents(&self, word_name: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        for (dict_name, dict) in &self.user_dictionaries {
            for (name, def) in &dict.words {
                if def.dependencies.contains(word_name) {
                    result.insert(format!("{}@{}", dict_name, name));
                }
            }
        }
        for (module_name, module_dict) in &self.module_vocabulary {
            for (name, def) in &module_dict.sample_words {
                if def.dependencies.contains(word_name) {
                    result.insert(format!("{}@{}", module_name, name));
                }
            }
        }
        result
    }
}
