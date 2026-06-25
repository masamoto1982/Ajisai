//! Pure HOF kernel memoization (direction B).
//!
//! Higher-order words such as `MAP` apply a kernel to each element against an
//! **isolated, element-only stack**: `map.rs` swaps the working stack out,
//! `clear()`s it, and pushes a single element before running the kernel. A
//! *pure* kernel therefore produces a result that depends only on the element
//! it is handed, so the per-element application `(kernel, element) -> result`
//! is a pure function and is safe to memoize. When a vector carries repeated
//! elements (`[ 3 3 3 5 ]`), this collapses N kernel runs into one run per
//! distinct element.
//!
//! Soundness rests on three independent guards:
//!
//! 1. **Pure kernel only.** The kernel must be `QuantizedPurity::Pure`, so it
//!    has no host effects and its result is a deterministic function of the
//!    element. Impure/unknown kernels are never memoized.
//! 2. **Canonical element identity.** The key is built from a *reduced rational*
//!    `numerator/denominator` plus the element's interpretation tag. A value
//!    without a cheap, decidable canonical identity (irrationals/`ExactScalar`,
//!    collections, NIL, ...) returns `None` and is executed normally — never a
//!    false cache hit.
//! 3. **Definition-change invalidation.** The backing store is the existing
//!    `elastic_cache`, which `invalidate_execution_artifacts` flushes on every
//!    dictionary or module epoch bump (`DEF`/`DEL`/import). The cache key also
//!    embeds both epochs, so a stale entry is doubly unreachable after any
//!    redefinition that could change the kernel's meaning.
//!
//! This activates the previously dormant `CacheManager` on a real execution
//! path while keeping the default observable result byte-for-byte identical.

use crate::interpreter::Interpreter;
use crate::types::{Interpretation, Token, Value};

/// Stable single-byte tag for an interpretation role, folded into the element
/// key so two scalars that are numerically equal but carry different roles
/// (e.g. a `RawNumber` vs a `TruthValue`) never share a cache slot. This is an
/// internal cache discriminator, not an external protocol surface.
fn hint_tag(hint: Interpretation) -> u8 {
    match hint {
        Interpretation::Unassigned => 0,
        Interpretation::RawNumber => 1,
        Interpretation::Interval => 2,
        Interpretation::Text => 3,
        Interpretation::TruthValue => 4,
        Interpretation::Timestamp => 5,
        Interpretation::Nil => 6,
        Interpretation::ContinuedFraction => 7,
    }
}

/// Canonical memo key for a single element, or `None` when the element has no
/// cheap decidable canonical identity and must not be memoized.
///
/// Only bare rational scalars qualify: `Value::as_scalar` returns `Some` solely
/// for `ValueData::Scalar(Fraction)` (irrational `ExactScalar`, Booleans,
/// tensors, vectors, records and NIL all return `None`). A `Fraction` is stored
/// reduced, so `numerator/denominator` is its canonical form and equal values
/// produce equal keys.
pub(super) fn element_value_key(elem: &Value) -> Option<String> {
    let f = elem.as_scalar()?;
    Some(format!(
        "{}:{}/{}",
        hint_tag(elem.hint),
        f.numerator(),
        f.denominator()
    ))
}

/// Stable, collision-free key for a kernel's token stream. The full serialized
/// form is used directly (not a hash) so distinct kernels can never collide
/// into a false cache hit. Returns `None` when the kernel has no plain token
/// body to key on.
pub(super) fn kernel_token_key(tokens: Option<&[Token]>) -> Option<String> {
    let tokens = tokens?;
    let mut key = String::with_capacity(tokens.len() * 4);
    for token in tokens {
        match token {
            Token::Number(n) => {
                key.push('n');
                key.push_str(n);
            }
            Token::String(s) => {
                key.push('s');
                key.push_str(s);
            }
            Token::Symbol(s) => {
                key.push('w');
                key.push_str(s);
            }
            Token::VectorStart => key.push('['),
            Token::VectorEnd => key.push(']'),
            Token::BlockStart => key.push('{'),
            Token::BlockEnd => key.push('}'),
            Token::Pipeline => key.push('~'),
            Token::NilCoalesce => key.push('^'),
            Token::CondClauseSep => key.push('|'),
            Token::LineBreak => key.push('\n'),
        }
        key.push('\u{1}');
    }
    Some(key)
}

impl Interpreter {
    fn hof_memo_cache_key(&self, kernel_key: &str, elem_key: &str) -> String {
        crate::elastic::CacheManager::build_key_with_context(
            kernel_key,
            elem_key,
            "hof-map",
            Some("memo"),
            self.dictionary_epoch,
            self.module_epoch,
        )
    }

    /// Look up a memoized per-element kernel result. Bumps the HOF-memo hit or
    /// miss counter and returns the cached `Value` on a hit.
    pub(crate) fn hof_memo_fetch(&mut self, kernel_key: &str, elem_key: &str) -> Option<Value> {
        let key = self.hof_memo_cache_key(kernel_key, elem_key);
        let (value, hit) = self.elastic_cache.fetch(&key);
        if hit {
            self.runtime_metrics.hof_memo_hit_count += 1;
        } else {
            self.runtime_metrics.hof_memo_miss_count += 1;
        }
        value
    }

    /// Store a per-element kernel result. The caller guarantees the kernel was
    /// pure, so the value is unconditionally cacheable.
    pub(crate) fn hof_memo_store(&mut self, kernel_key: &str, elem_key: &str, value: &Value) {
        let key = self.hof_memo_cache_key(kernel_key, elem_key);
        self.elastic_cache.store(key, value.clone(), true);
        self.runtime_metrics.hof_memo_store_count += 1;
    }
}
