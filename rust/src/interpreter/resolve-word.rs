use crate::types::WordDefinition;
use std::collections::HashSet;
use std::sync::Arc;

use super::Interpreter;

impl Interpreter {
    /// `@` 区切りのパスを解析して (layers, word) を返す。
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

    pub(crate) fn resolve_short_name(&self, name: &str) -> Option<(String, Arc<WordDefinition>)> {
        let upper = name.to_uppercase();

        // 1. Built-in words (always highest priority, no ambiguity check)
        if let Some(def) = self.core_vocabulary.get(&upper) {
            return Some((upper, def.clone()));
        }

        // 2. Check imported module words (e.g., "PLAY" → "MUSIC@PLAY" in core_vocabulary)
        for module_name in &self.imported_modules {
            let qualified = format!("{}@{}", module_name, upper);
            if let Some(def) = self.core_vocabulary.get(&qualified) {
                return Some((qualified, def.clone()));
            }
        }

        // 3. Collect all module sample matches
        let mut module_matches: Vec<(String, Arc<WordDefinition>, u64)> = Vec::new();
        for (module_name, dict) in &self.module_samples {
            if let Some(def) = dict.sample_words.get(&upper) {
                module_matches.push((format!("{}@{}", module_name, upper), def.clone(), def.registration_order));
            }
        }

        // 4. Collect all custom dictionary matches
        let mut user_matches: Vec<(String, Arc<WordDefinition>, u64)> = Vec::new();
        for (dict_name, dict) in &self.user_dictionaries {
            if let Some(def) = dict.words.get(&upper) {
                user_matches.push((format!("{}@{}", dict_name, upper), def.clone(), def.registration_order));
            }
        }

        // 5. Ambiguity detection
        if !module_matches.is_empty() && !user_matches.is_empty() {
            return None;
        }

        // 6. Return best module match
        if !module_matches.is_empty() {
            module_matches.sort_by_key(|(_, _, order)| *order);
            let (name, def, _) = module_matches.into_iter().next().unwrap();
            return Some((name, def));
        }

        // 7. Return best custom match
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
        for (module_name, dict) in &self.module_samples {
            if dict.sample_words.contains_key(&upper) {
                paths.push(format!("{}@{}", module_name, upper));
            }
        }
        for (dict_name, dict) in &self.user_dictionaries {
            if dict.words.contains_key(&upper) {
                paths.push(format!("{}@{}", dict_name, upper));
            }
        }

        if paths.len() > 1 { paths } else { vec![] }
    }

    pub(crate) fn resolve_word_entry(&self, name: &str) -> Option<(String, Arc<WordDefinition>)> {
        let (layers, word) = Self::split_path(name);

        match layers.len() {
            0 => {
                self.resolve_short_name(name)
            }
            1 => {
                let ns = &layers[0];
                if ns == "CORE" {
                    return self.core_vocabulary.get(&word).cloned().map(|def| (word.clone(), def));
                }
                if let Some(module_dict) = self.module_samples.get(ns.as_str()) {
                    if let Some(def) = module_dict.sample_words.get(&word) {
                        return Some((format!("{}@{}", ns, word), def.clone()));
                    }
                }
                if let Some(user_dict) = self.user_dictionaries.get(ns.as_str()) {
                    if let Some(def) = user_dict.words.get(&word) {
                        return Some((format!("{}@{}", ns, word), def.clone()));
                    }
                }
                let qualified = format!("{}@{}", ns, word);
                self.core_vocabulary.get(&qualified).cloned().map(|def| (qualified, def))
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
                        return self.core_vocabulary.get(&word).cloned().map(|def| (word.clone(), def));
                    }
                    let qualified = format!("{}@{}", second, word);
                    if let Some(def) = self.core_vocabulary.get(&qualified) {
                        return Some((qualified, def.clone()));
                    }
                    if let Some(module_dict) = self.module_samples.get(second.as_str()) {
                        if let Some(def) = module_dict.sample_words.get(&word) {
                            return Some((format!("{}@{}", second, word), def.clone()));
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

    pub(crate) fn resolve_word(&self, name: &str) -> Option<Arc<WordDefinition>> {
        self.resolve_word_entry(name).map(|(_, def)| def)
    }

    pub(crate) fn word_exists(&self, name: &str) -> bool {
        self.resolve_word(name).is_some()
    }

    pub(crate) fn is_user_word(&self, name: &str) -> bool {
        self.resolve_word(name)
            .map(|def| !def.is_builtin)
            .unwrap_or(false)
    }

    pub fn rebuild_dependencies(&mut self) -> crate::error::Result<()> {
        self.dependents.clear();

        let mut all_user_words: Vec<(String, Arc<WordDefinition>)> = Vec::new();

        for (dict_name, dict) in &self.user_dictionaries {
            for (name, def) in &dict.words {
                all_user_words.push((format!("{}@{}", dict_name, name), Arc::clone(def)));
            }
        }

        for (module_name, module_dict) in &self.module_samples {
            for (name, def) in &module_dict.sample_words {
                all_user_words.push((format!("{}@{}", module_name, name), Arc::clone(def)));
            }
        }

        for (word_name, word_def) in &all_user_words {
            let mut dependencies = HashSet::new();
            for line in word_def.lines.iter() {
                for token in line.body_tokens.iter() {
                    if let crate::types::Token::Symbol(s) = token {
                        let upper_s = s.to_uppercase();
                        if let Some((resolved_name, resolved_def)) = self.resolve_word_entry(&upper_s) {
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
                        continue;
                    }
                }
                if let Some(module_dict) = self.module_samples.get_mut(&dict_name) {
                    if let Some(def) = module_dict.sample_words.get_mut(&short_name) {
                        Arc::make_mut(def).dependencies = dependencies;
                    }
                }
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
        for (module_name, module_dict) in &self.module_samples {
            for (name, def) in &module_dict.sample_words {
                if def.dependencies.contains(word_name) {
                    result.insert(format!("{}@{}", module_name, name));
                }
            }
        }
        result
    }
}
