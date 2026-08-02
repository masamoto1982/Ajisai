use super::{Interpreter, ResolveCacheEntry};

impl Interpreter {
    pub(crate) fn make_resolve_cache_key(name: &str) -> String {
        crate::core_word_aliases::canonicalize_core_word_name(name).into_owned()
    }

    /// The cache key is the name.
    ///
    /// It used to be qualified by the executing word's owning dictionary,
    /// because a bare name could resolve to different targets depending on
    /// which dictionary's word was running. LANG.DICTIONARY.RESOLUTION makes
    /// resolution "a deterministic function of the normalized name and the
    /// current dictionary" — with two tiers there is no context to vary, so a
    /// name has one answer and one cache entry.
    fn contextual_resolve_cache_key(&self, name: &str) -> String {
        Self::make_resolve_cache_key(name)
    }

    pub(crate) fn lookup_resolve_cache(&mut self, name: &str) -> Option<String> {
        let key = self.contextual_resolve_cache_key(name);
        let entry = self.resolve_cache.get(&key)?;
        if entry.dictionary_epoch == self.dictionary_epoch {
            self.runtime_metrics.resolve_cache_hit_count += 1;
            Some(entry.resolved_name.clone())
        } else {
            self.runtime_metrics.resolve_cache_miss_count += 1;
            None
        }
    }

    pub(crate) fn store_resolve_cache(
        &mut self,
        input_name: &str,
        resolved_name: &str,
        registration_order: u64,
    ) {
        let key = self.contextual_resolve_cache_key(input_name);
        self.resolve_cache.insert(
            key,
            ResolveCacheEntry {
                resolved_name: resolved_name.to_string(),
                dictionary_epoch: self.dictionary_epoch,
                registration_order,
            },
        );
    }
}
