use super::{Interpreter, ResolveCacheEntry};

impl Interpreter {
    pub(crate) fn make_resolve_cache_key(name: &str) -> String {
        crate::core_word_aliases::canonicalize_core_word_name(name)
    }

    pub(crate) fn lookup_resolve_cache(&mut self, name: &str) -> Option<String> {
        let key = Self::make_resolve_cache_key(name);
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
        let key = Self::make_resolve_cache_key(input_name);
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
