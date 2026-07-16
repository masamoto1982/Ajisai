//! Interned Record shapes (hidden-class-style layout sharing).
//!
//! A Record's key→slot mapping is layout metadata, not data: every Record
//! built from the same keys in the same slots has the same mapping. Storing
//! the mapping per instance made every Record clone copy a `HashMap` and made
//! Record equality compare two `HashMap`s. Interning the mapping in a global
//! table gives all same-layout Records one shared `Arc<RecordShape>`, so
//! clones are pointer bumps and same-layout equality short-circuits on
//! pointer identity.
//!
//! Because Ajisai values are immutable, a shape never transitions after
//! construction — there is no hidden-class transition chain, only a single
//! intern at build time. The table is bounded: once `INTERN_CAP` distinct
//! layouts exist, later layouts get private (uninterned) shapes, which are
//! still fully functional — they just don't share. Observable semantics are
//! unchanged either way: equality falls back to comparing the mappings
//! themselves whenever pointer identity fails.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// The shared, immutable key→slot layout of a Record value.
#[derive(Debug)]
pub struct RecordShape {
    index: HashMap<String, usize>,
}

impl RecordShape {
    /// Slot of `key` within the record's `pairs`, if present.
    pub fn slot(&self, key: &str) -> Option<usize> {
        self.index.get(key).copied()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    /// The full key→slot mapping (for successor-shape construction and the
    /// equality fallback).
    pub fn mapping(&self) -> &HashMap<String, usize> {
        &self.index
    }
}

impl PartialEq for RecordShape {
    fn eq(&self, other: &Self) -> bool {
        // Interned shapes with the same layout are the same allocation, so
        // this deep comparison only runs across the interned/uninterned
        // boundary (or between two uninterned shapes past the cap).
        std::ptr::eq(self, other) || self.index == other.index
    }
}

/// Cap on distinct interned layouts. Prevents adversarial or degenerate
/// workloads (e.g. a JSON stream where every object has unique keys) from
/// growing the process-lifetime table without bound. Past the cap, shapes are
/// simply not shared.
const INTERN_CAP: usize = 1024;

/// Shapes bucketed by an order-independent layout hash. The hash avoids
/// building any sorted/cloned canonical key on the lookup path; bucket
/// collisions resolve by comparing the mappings themselves.
struct InternTable {
    buckets: HashMap<u64, Vec<Arc<RecordShape>>>,
    stored: usize,
}

fn intern_table() -> &'static Mutex<InternTable> {
    static TABLE: OnceLock<Mutex<InternTable>> = OnceLock::new();
    TABLE.get_or_init(|| {
        Mutex::new(InternTable {
            buckets: HashMap::new(),
            stored: 0,
        })
    })
}

/// Order-independent hash of a key→slot mapping: each entry is hashed on its
/// own and the entry hashes are combined commutatively, so no sorted
/// canonical form (with its per-key `String` clones) is ever materialized.
fn layout_hash(index: &HashMap<String, usize>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut acc: u64 = 0x9E37_79B9_7F4A_7C15 ^ (index.len() as u64);
    for (key, &slot) in index {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        slot.hash(&mut hasher);
        acc ^= hasher.finish().wrapping_mul(0x100_0000_01B3);
    }
    acc
}

/// Intern an arbitrary key→slot mapping, returning the shared shape for that
/// layout (or a private shape once the table is at capacity).
pub fn intern_record_shape(index: HashMap<String, usize>) -> Arc<RecordShape> {
    let hash = layout_hash(&index);
    let mut table = match intern_table().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(bucket) = table.buckets.get(&hash) {
        for shape in bucket {
            if shape.index == index {
                return Arc::clone(shape);
            }
        }
    }
    let shape = Arc::new(RecordShape { index });
    if table.stored < INTERN_CAP {
        table
            .buckets
            .entry(hash)
            .or_default()
            .push(Arc::clone(&shape));
        table.stored += 1;
    }
    shape
}

/// Intern the common contiguous layout where the i-th key owns slot i —
/// the layout every ordered-fields record constructor produces.
pub fn record_shape_from_ordered_keys<I, S>(keys: I) -> Arc<RecordShape>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let index: HashMap<String, usize> = keys
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k.into(), i))
        .collect();
    intern_record_shape(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_layout_interns_to_same_allocation() {
        let a = record_shape_from_ordered_keys(["x", "y"]);
        let b = record_shape_from_ordered_keys(["x", "y"]);
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn different_layouts_do_not_share() {
        let a = record_shape_from_ordered_keys(["x", "y"]);
        let b = record_shape_from_ordered_keys(["y", "x"]);
        assert!(!Arc::ptr_eq(&a, &b));
        assert_ne!(*a, *b);
    }

    #[test]
    fn equality_falls_back_to_mapping_comparison() {
        let mut m = HashMap::new();
        m.insert("k".to_string(), 0);
        let interned = intern_record_shape(m.clone());
        let private = RecordShape { index: m };
        assert_eq!(*interned, private);
    }

    #[test]
    fn slot_lookup_matches_mapping() {
        let shape = record_shape_from_ordered_keys(["a", "b", "c"]);
        assert_eq!(shape.slot("b"), Some(1));
        assert_eq!(shape.slot("missing"), None);
        assert!(shape.contains_key("c"));
    }
}
