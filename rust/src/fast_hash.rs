//! A fast hasher for the interpreter's name-keyed maps.
//!
//! The standard `RandomState` (SipHash-1-3) was a measurable share of every
//! Word dispatch on WebAssembly: each dispatch hashes the Word's name to probe
//! the Core and the User dictionaries. This is the multiply-rotate hash rustc
//! itself uses for its own tables (FxHash): a few instructions per word of
//! key.
//!
//! It is not resistant to chosen collisions, as SipHash is. A program's names
//! are bounded by the source-size ceiling (LANG.MACHINE.LIMITS), so the worst
//! a crafted program can do is make its own dictionary probes linear in a
//! bounded number of names. Iteration order was never something to rely on:
//! `RandomState` already varied it from process to process.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

#[derive(Default, Clone, Copy)]
pub(crate) struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let mut word = [0u8; 8];
            word.copy_from_slice(chunk);
            self.add(u64::from_le_bytes(word));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            let mut word = [0u8; 8];
            word[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(word));
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add(u64::from(i));
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

/// A `HashMap` hashed by [`FxHasher`]; build one with `default()`.
pub(crate) type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

#[cfg(test)]
mod tests {
    use super::FastMap;

    /// Keys that share a prefix, differ only past the first word, or are
    /// empty all stay apart.
    #[test]
    fn distinct_keys_stay_distinct() {
        let keys = ["", "A", "ADD", "ADDITION", "ADDITIONS", "ADDITIONZ", "Z"];
        let mut map: FastMap<String, usize> = FastMap::default();
        for (i, key) in keys.iter().enumerate() {
            map.insert(key.to_string(), i);
        }
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(map.get(*key), Some(&i));
        }
        assert_eq!(map.len(), keys.len());
    }
}
