//! Cross-reset compiled-artifact cache (Phase 5).
//!
//! Session state (stack, dictionaries, output, effects, epochs) is ephemeral:
//! the GUI worker rebuilds it from a snapshot on every run and clears it with a
//! session reset in between. The artifacts *derived* from a word body — chiefly
//! its `CompiledPlan` — are semantically invariant. They depend only on the
//! word's content identity (Section 8.6), which already folds in the body tokens
//! and the content identities of every dependency, together with the compile
//! feature flags and the plan schema version. Keying reuse on that identity lets
//! an unchanged word survive a session reset without recompilation, while any
//! change to the body, a dependency, a compile flag, or the schema yields a
//! different key and never reuses a stale plan.
//!
//! The store is a bounded LRU cache. Because it is a cache, evicting a live
//! entry only forces a later rebuild and never changes a result; likewise,
//! disabling reuse entirely (`AJISAI_NO_ARTIFACT_REUSE`) is observationally
//! transparent — the same plan is simply recompiled on demand.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use super::compiled_plan::CompiledPlan;

/// Default number of distinct compiled artifacts kept alive across resets.
/// Comfortably covers a GUI dictionary while bounding a long-lived worker's
/// memory; excess entries are evicted in least-recently-used order.
pub const DEFAULT_ARTIFACT_STORE_CAPACITY: usize = 1024;

/// Compile-time feature flags that change the *shape* of a `CompiledPlan`. Two
/// interpreters that disagree on any of these lower the same body to different
/// ops, so the flags are part of the artifact key. Purely observational
/// counters and runtime state are deliberately excluded.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CompileFlags {
    pub cond_dispatch: bool,
    pub vector_literal: bool,
    pub compiled_clause: bool,
}

/// Identity of a reusable compiled artifact. `content_identity` is the word's
/// Section 8.6 content identity (body ⊕ dependency identities); the remaining
/// fields pin the compilation environment so a plan built under different
/// assumptions is never reused.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ArtifactKey {
    pub content_identity: String,
    pub flags: CompileFlags,
    pub schema_version: u32,
}

impl ArtifactKey {
    pub fn new(
        content_identity: impl Into<String>,
        flags: CompileFlags,
        schema_version: u32,
    ) -> Self {
        Self {
            content_identity: content_identity.into(),
            flags,
            schema_version,
        }
    }
}

/// Observational counters for the artifact cache. Never part of any value or
/// result; safe to expose to the Playground as cost-model metrics.
#[derive(Debug, Clone, Copy, Default)]
pub struct ArtifactMetrics {
    /// Plans compiled and inserted into the store (one per cache miss that was
    /// actually rebuilt).
    pub build_count: u64,
    /// Reuses served from the store instead of recompiling.
    pub hit_count: u64,
    /// Lookups that found no stored plan for the key.
    pub miss_count: u64,
    /// Entries dropped to keep the store within `capacity`.
    pub eviction_count: u64,
}

/// Bounded, content-addressed store of compiled plans that outlives a session
/// reset.
pub struct ArtifactStore {
    plans: HashMap<ArtifactKey, Arc<CompiledPlan>>,
    /// Access order for LRU eviction; front = least recently used.
    order: VecDeque<ArtifactKey>,
    capacity: usize,
    metrics: ArtifactMetrics,
}

impl Default for ArtifactStore {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_ARTIFACT_STORE_CAPACITY)
    }
}

impl ArtifactStore {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            plans: HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
            metrics: ArtifactMetrics::default(),
        }
    }

    /// Look up a stored plan, counting a hit or miss and refreshing LRU order on
    /// a hit.
    pub fn get(&mut self, key: &ArtifactKey) -> Option<Arc<CompiledPlan>> {
        match self.plans.get(key) {
            Some(plan) => {
                let plan = plan.clone();
                self.metrics.hit_count += 1;
                self.touch(key);
                Some(plan)
            }
            None => {
                self.metrics.miss_count += 1;
                None
            }
        }
    }

    /// Store a freshly compiled plan under `key`, counting a build and evicting
    /// the least-recently-used entry if the store is now over capacity.
    pub fn insert(&mut self, key: ArtifactKey, plan: Arc<CompiledPlan>) {
        self.metrics.build_count += 1;
        if self.plans.insert(key.clone(), plan).is_some() {
            // Overwriting an existing entry (rare: same key recompiled); keep a
            // single order record and move it to the back.
            self.touch(&key);
        } else {
            self.order.push_back(key);
            self.evict_if_needed();
        }
    }

    fn touch(&mut self, key: &ArtifactKey) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key.clone());
    }

    fn evict_if_needed(&mut self) {
        while self.plans.len() > self.capacity {
            match self.order.pop_front() {
                Some(oldest) => {
                    if self.plans.remove(&oldest).is_some() {
                        self.metrics.eviction_count += 1;
                    }
                }
                None => break,
            }
        }
    }

    /// Drop every stored plan. Used by the full (non-session) reset. Metrics are
    /// preserved so cumulative build/hit/miss/eviction totals stay monotonic.
    pub fn clear(&mut self) {
        self.plans.clear();
        self.order.clear();
    }

    pub fn metrics(&self) -> ArtifactMetrics {
        self.metrics
    }

    pub fn len(&self) -> usize {
        self.plans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plans.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Adjust the capacity, evicting immediately if the new bound is smaller.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        self.evict_if_needed();
    }
}
