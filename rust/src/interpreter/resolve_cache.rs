use super::{Interpreter, ResolveCacheEntry};

impl Interpreter {
    /// The cache key is the canonical name, and **the caller supplies it
    /// already canonical.**
    ///
    /// It used to be qualified by the executing word's owning dictionary,
    /// because a bare name could resolve to different targets depending on
    /// which dictionary's word was running. LANG.DICTIONARY.RESOLUTION makes
    /// resolution "a deterministic function of the normalized name and the
    /// current dictionary" — with two tiers there is no context to vary, so a
    /// name has one answer and one cache entry.
    ///
    /// The cache used to canonicalize the name itself, which meant
    /// `resolve_word_entry` — its only caller, and one that canonicalizes
    /// before it calls — paid for a second linear walk of the alias table and
    /// then a `String` allocation to hold a name it already had. Canonicalizing
    /// is idempotent, so the second pass could only ever return its input; this
    /// is a lookup, and a lookup that allocates to ask its question is not a
    /// saving over the work it avoids. `HashMap<String, _>` borrows `&str` for
    /// `get`, so asking costs nothing now.
    pub(crate) fn lookup_resolve_cache(&mut self, canonical_name: &str) -> Option<String> {
        let entry = self.resolve_cache.get(canonical_name)?;
        if entry.dictionary_epoch == self.dictionary_epoch {
            self.runtime_metrics.resolve_cache_hit_count += 1;
            Some(entry.resolved_name.clone())
        } else {
            self.runtime_metrics.resolve_cache_miss_count += 1;
            None
        }
    }

    /// Record a resolution under its canonical name, which — as above — the
    /// caller has already canonicalized.
    pub(crate) fn store_resolve_cache(
        &mut self,
        canonical_name: &str,
        resolved_name: &str,
        registration_order: u64,
    ) {
        self.resolve_cache.insert(
            canonical_name.to_string(),
            ResolveCacheEntry {
                resolved_name: resolved_name.to_string(),
                dictionary_epoch: self.dictionary_epoch,
                registration_order,
            },
        );
    }
}
