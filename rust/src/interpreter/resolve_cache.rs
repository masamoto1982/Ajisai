use super::{Interpreter, ResolveCacheEntry};

impl Interpreter {
    pub(crate) fn make_resolve_cache_key(name: &str) -> String {
        crate::core_word_aliases::canonicalize_core_word_name(name).into_owned()
    }

    /// Cache key qualified by the active owning-dictionary context (Section 8.6).
    /// A bare name can resolve to different targets depending on which
    /// dictionary's word is currently executing, so the cache must not share an
    /// entry across contexts.
    fn contextual_resolve_cache_key(&self, name: &str) -> String {
        let base = Self::make_resolve_cache_key(name);
        match &self.owning_dictionary_context {
            Some(ctx) => format!("{}\u{1}{}", ctx, base),
            None => base,
        }
    }

    pub(crate) fn lookup_resolve_cache(&mut self, name: &str) -> Option<String> {
        let key = self.contextual_resolve_cache_key(name);
        let entry = self.resolve_cache.get(&key)?;
        if entry.dictionary_epoch == self.dictionary_epoch
            && entry.module_epoch == self.module_epoch
        {
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
                module_epoch: self.module_epoch,
                registration_order,
            },
        );
    }
}
