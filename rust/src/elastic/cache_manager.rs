use crate::types::Value;
/// Pure-result cache for the Elastic Engine (M4).
///
/// Only values produced by words with `purity == Purity::Pure` are stored.
/// Impure and unknown words always bypass the cache.
///
/// # Cache key format
/// ```text
/// "<word_name>|<args_repr>|<mode>"
/// ```
/// where `args_repr` is the `Debug` representation of the argument list and
/// `mode` is the execution mode string (`"elastic-safe"` etc.).
use std::collections::HashMap;

// ── Internal entry ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct CacheEntry {
    value: Value,
    hit_count: u64,
}

// ── Public struct ─────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct CacheManager {
    store: HashMap<String, CacheEntry>,
    hit_count: u64,
    miss_count: u64,
}

impl CacheManager {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Key construction ──────────────────────────────────────────────────

    /// Build a canonical cache key.
    ///
    /// `args_repr` should be a stable, deterministic string representation
    /// of the word's input arguments (e.g. `format!("{:?}", args)`).
    pub fn build_key(word_name: &str, args_repr: &str, mode: &str) -> String {
        Self::build_key_with_context(word_name, args_repr, mode, None, 0, 0)
    }

    pub fn build_key_with_context(
        word_name: &str,
        args_repr: &str,
        mode: &str,
        strategy_label: Option<&str>,
        dictionary_epoch: u64,
        module_epoch: u64,
    ) -> String {
        let strategy = strategy_label.unwrap_or("none");
        format!(
            "{}|{}|{}|{}|dict:{}|mod:{}",
            word_name, args_repr, mode, strategy, dictionary_epoch, module_epoch
        )
    }

    // ── Read ──────────────────────────────────────────────────────────────

    /// Attempt a cache lookup.
    ///
    /// Returns `(Some(value), true)` on hit, `(None, false)` on miss.
    /// The `hit_count` on the entry is incremented on every hit.
    pub fn fetch(&mut self, key: &str) -> (Option<Value>, bool) {
        if let Some(entry) = self.store.get_mut(key) {
            entry.hit_count += 1;
            self.hit_count += 1;
            if crate::elastic::tracer::is_enabled() {
                eprintln!("[cache]   HIT  key={}", key);
            }
            (Some(entry.value.clone()), true)
        } else {
            self.miss_count += 1;
            if crate::elastic::tracer::is_enabled() {
                eprintln!("[cache]   MISS key={}", key);
            }
            (None, false)
        }
    }

    // ── Write ─────────────────────────────────────────────────────────────

    /// Store a result.
    ///
    /// The `pure` flag is a **runtime safety gate** — if `pure` is `false`
    /// the value is silently dropped and never stored.  This prevents
    /// accidental caching of impure or unknown-purity results.
    pub fn store(&mut self, key: String, value: Value, pure: bool) {
        if !pure {
            return;
        }
        self.store.insert(
            key,
            CacheEntry {
                value,
                hit_count: 0,
            },
        );
    }

    // ── Invalidation ──────────────────────────────────────────────────────

    /// Remove all entries whose key starts with `prefix`.
    ///
    /// Useful when a dictionary mutation could invalidate cached results
    /// that depend on a now-changed word definition.
    pub fn invalidate_prefix(&mut self, prefix: &str) {
        self.store.retain(|k, _| !k.starts_with(prefix));
    }

    /// Flush the entire cache.
    pub fn clear(&mut self) {
        self.store.clear();
    }

    // ── Statistics ────────────────────────────────────────────────────────

    pub fn hit_count(&self) -> u64 {
        self.hit_count
    }

    pub fn miss_count(&self) -> u64 {
        self.miss_count
    }

    pub fn cached_key_count(&self) -> usize {
        self.store.len()
    }

    /// Hit rate in [0.0, 1.0].  Returns `0.0` when no accesses have occurred.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            0.0
        } else {
            self.hit_count as f64 / total as f64
        }
    }
}
