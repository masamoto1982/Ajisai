//! Observation digest: folds a program's whole observation into one 32-byte
//! BLAKE3 digest (Phase 1, `docs/dev/competitive-advantage-work-order-2026-08.md`).
//!
//! Ajisai is a pure function from source to observation — 64 of 65 Words are
//! deterministic and the one host-relative Word (`PRINT`) still produces a
//! deterministic *sequence* of output lines — and value equality is semantic
//! rather than representational (`rust/src/types/value_hash_tests.rs`). Both
//! facts together are what make one digest of the whole observation
//! meaningful: two runs that are the same *program result*, however they got
//! there, must fold to the same bytes. Four representation traps make that
//! easy to get wrong without a test noticing:
//!
//!  * **Algebraic normal forms are not canonical.** `8 SQRT` keeps the basis
//!    `{8}`; `2 SQRT 2 SQRT +` keeps `{2}` — equal values, disagreeing
//!    `normal_form_terms()`. Hashing the normal form would make this Phase's
//!    own founding example fail. `impl Hash for Algebraic`
//!    (`types/exact/algebraic.rs`) already solves this by hashing
//!    `floor(value * 2^bits)`, doubling `bits` until the bounding interval's
//!    floor is decided; this module reruns that exact loop at that impl's own
//!    `HASH_KEY_BITS` precision (`DIGEST_ALGEBRAIC_BITS`), not a larger one —
//!    see `encode_algebraic`'s doc comment for why a larger precision is not
//!    just unnecessary but actively wrong for a value that can carry
//!    hundreds of terms. This gives "equal ⇒ same digest" but not the
//!    converse: two distinct algebraic numbers closer together than
//!    `2^-DIGEST_ALGEBRAIC_BITS` fold to the same key. Documented as a
//!    residual, not silently assumed away.
//!  * **`Vector` and `Tensor` can be the same value.** A rectangular numeric
//!    `Vector` equals the `Tensor` holding the same lanes
//!    (`tensor_eq_vector`), so encoding by `ValueData` variant would answer
//!    differently for two equal values. Both are encoded through
//!    `Value::len()` / `Value::child(i)` instead, which is representation
//!    agnostic — but `Scalar`/`ExactScalar` also report `len() == 1` and
//!    `child(0) == self`, so those two are matched as their own leaves before
//!    a collection encoder ever runs, or encoding them would recurse forever.
//!  * **`hint` is presentation, not meaning; `absence`'s reason is meaning.**
//!    `PartialEq for Value` never reads `hint` and always reads the NIL
//!    reason, so the digest follows exactly that split.
//!  * **`stackDisplay` is not the value.** It is SPEC §4.2.3's continued
//!    fraction truncated at a display budget (`√2` runs to ~194 characters
//!    and ends in `...)`), so hashing it would fold two different numbers to
//!    one digest the moment either runs past the budget. The digest always
//!    encodes the value itself.
//!
//! `ExactReal::Computable` (Tier 2) has no canonical finite representation and
//! no current Word constructs it; meeting one aborts the whole digest to
//! `None` rather than fabricate a value for it.

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::Zero;

use crate::interpreter::word_identity::{content_digest, encode_token};
use crate::types::exact::{Algebraic, ExactReal};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

/// Version tag for this byte grammar. Bump it (and the schema version noun,
/// not `report::SCHEMA_VERSION`) if the grammar itself ever changes — the
/// digest is not a compatible value across a tag change.
pub(crate) const DIGEST_SCHEMA_TAG: &[u8] = b"AJISAI-OBS-1";

/// Bits of fractional precision an algebraic scalar's digest key is taken at
/// — deliberately `types::exact::algebraic::HASH_KEY_BITS`, the same
/// precision `impl Hash for Algebraic` already uses, not a larger one; see
/// `encode_algebraic`. Gives "equal ⇒ same digest" (soundness); does not
/// separate two distinct algebraic numbers closer together than
/// `2^-DIGEST_ALGEBRAIC_BITS` — see the module comment.
pub(crate) const DIGEST_ALGEBRAIC_BITS: u32 = 64;

/// Everything one observation digest is taken over.
pub(crate) struct ObservationDigestInput<'a> {
    /// The report's `status` (`"ok"` / `"error"`).
    pub status: &'a str,
    /// The stack, bottom to top — `Interpreter::get_stack()`'s own order.
    pub stack: &'a [Value],
    /// `PRINT` output lines, in order.
    pub output: &'a [String],
    /// `(normalized word name, content identity)` pairs, sorted by name by
    /// the caller.
    pub user_words: &'a [(String, String)],
    /// The error's stable category id. `None` on success.
    pub error_category: Option<&'a str>,
}

/// The canonical digest of one observation, or `None` when it cannot be
/// encoded (a Tier 2 `ExactReal::Computable` scalar is present anywhere in
/// the stack).
pub(crate) fn observation_digest(input: ObservationDigestInput<'_>) -> Option<String> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(DIGEST_SCHEMA_TAG);

    write_section(&mut bytes, 0x01);
    write_str(&mut bytes, input.status);

    write_section(&mut bytes, 0x02);
    write_u64(&mut bytes, input.stack.len() as u64);
    for value in input.stack {
        encode_value(&mut bytes, value)?;
    }

    write_section(&mut bytes, 0x03);
    write_u64(&mut bytes, input.output.len() as u64);
    for line in input.output {
        write_str(&mut bytes, line);
    }

    write_section(&mut bytes, 0x04);
    write_u64(&mut bytes, input.user_words.len() as u64);
    for (name, identity) in input.user_words {
        write_str(&mut bytes, name);
        write_str(&mut bytes, identity);
    }

    write_section(&mut bytes, 0x05);
    write_opt_str(&mut bytes, input.error_category);

    Some(content_digest(&bytes))
}

fn write_section(bytes: &mut Vec<u8>, id: u8) {
    bytes.push(0x1D);
    bytes.push(id);
}

fn write_u64(bytes: &mut Vec<u8>, n: u64) {
    bytes.extend_from_slice(&n.to_be_bytes());
}

fn write_str(bytes: &mut Vec<u8>, s: &str) {
    write_u64(bytes, s.len() as u64);
    bytes.extend_from_slice(s.as_bytes());
}

fn write_opt_str(bytes: &mut Vec<u8>, s: Option<&str>) {
    match s {
        None => bytes.push(0x00),
        Some(s) => {
            bytes.push(0x01);
            write_str(bytes, s);
        }
    }
}

/// A `BigInt`, written as its signed radix-16 spelling. Never downcast to a
/// machine integer — the whole point is that arbitrary-precision values stay
/// exact through the encoding.
fn write_sint(bytes: &mut Vec<u8>, i: &BigInt) {
    write_str(bytes, &i.to_str_radix(16));
}

/// Encode one value. Leaves are matched directly on `ValueData` first — this
/// is what keeps `Scalar`/`ExactScalar` (whose `len()`/`child()` describe a
/// one-element self-loop) from ever reaching the `Vector`/`Tensor` recursion.
/// `Vector` and `Tensor` share one arm precisely because they must encode
/// identically when they are equal: it walks `Value::len()` / `Value::child`,
/// never the raw variant payload.
fn encode_value(bytes: &mut Vec<u8>, value: &Value) -> Option<()> {
    match &value.data {
        ValueData::Nil => {
            bytes.push(b'N');
            let reason = value
                .nil_reason()
                .map(|r| r.as_protocol_str())
                .unwrap_or("");
            write_str(bytes, reason);
        }
        ValueData::Boolean(b) => {
            bytes.push(b'B');
            bytes.push(u8::from(*b));
        }
        ValueData::Text(s) => {
            bytes.push(b'S');
            write_str(bytes, s);
        }
        ValueData::CodeBlock(tokens) => {
            bytes.push(b'K');
            write_u64(bytes, tokens.len() as u64);
            for token in tokens {
                // Same inter-token separator `body_content_key` uses ahead of
                // `encode_token`: without it, a single `Symbol("AYB")` and the
                // two tokens `Symbol("A"), Symbol("YB")`-shaped pairs are not
                // guaranteed to stay distinguishable once concatenated.
                bytes.push(0x1f);
                encode_token(bytes, token);
            }
        }
        ValueData::Scalar(f) => encode_rational(bytes, f),
        ValueData::ExactScalar(exact) => match exact {
            ExactReal::Rational(f) => encode_rational(bytes, f),
            ExactReal::Algebraic(alg) => encode_algebraic(bytes, alg),
            ExactReal::Computable(_) => return None,
        },
        ValueData::Vector(_) | ValueData::Tensor { .. } => {
            bytes.push(b'V');
            let len = value.len();
            write_u64(bytes, len as u64);
            for i in 0..len {
                let child = value
                    .child(i)
                    .expect("i < value.len() always has a child for Vector/Tensor");
                encode_value(bytes, &child)?;
            }
        }
    }
    Some(())
}

/// A rational scalar, reduced and sign-normalized the same way
/// `impl Hash for Fraction` (`types/fraction.rs`) is: divide out the gcd, then
/// flip both signs if the denominator came out negative. Uses
/// `num_integer::Integer::gcd` rather than `Fraction`'s own
/// `balanced_bigint_gcd` fast path, which lives in `types/bigint_gcd` — Phase
/// 1 reads `rust/src/types/` but does not edit it — but both compute the same
/// canonical reduced pair, so an unreduced and a reduced fraction still land
/// on identical bytes (Step 1.4's `unreduced_fraction_matches_reduced`).
fn encode_rational(bytes: &mut Vec<u8>, f: &Fraction) {
    let (mut n, mut d) = f.to_bigint_pair();
    let g = n.gcd(&d);
    if !g.is_zero() {
        n /= &g;
        d /= &g;
    }
    if d.sign() == Sign::Minus {
        n = -n;
        d = -d;
    }
    bytes.push(b'Q');
    write_sint(bytes, &n);
    write_sint(bytes, &d);
}

/// An algebraic scalar, keyed by `floor(value * 2^DIGEST_ALGEBRAIC_BITS)` —
/// the same representation-independent enclosure key `impl Hash for
/// Algebraic` uses (`HASH_KEY_BITS`), at that same precision rather than a
/// larger one of this module's own. `Algebraic` never holds a rational
/// (rationals demote eagerly), so the value is never exactly on a `2^-bits`
/// grid line and the loop below always terminates.
///
/// Reusing `HASH_KEY_BITS` rather than choosing a larger precision is load
/// bearing, not just tidy: this loop's cost is `O(terms)` big-integer square
/// roots at `~2*bits` bits each, and a legitimately reachable value can carry
/// up to `max_algebraic_terms` (512) terms. Keying at 512 bits instead of 64
/// made one such 256-term stack value's digest cost tens of milliseconds in
/// a release build and several hundred in a debug one — multiplied across a
/// stack of them, enough to make an unrelated resource-limit regression test
/// time out in CI. `HASH_KEY_BITS` is not a lesser precision chosen for
/// speed at correctness's expense: it is the precision `UNIQUE` / `GROUP` /
/// `TALLY` already trust for exactly this soundness property everywhere in
/// the interpreter, so reusing it does not introduce a new residual — it
/// reuses the one already load-bearing elsewhere. The residual this leaves
/// (two distinct algebraic numbers within `2^-64` of each other digest
/// alike) is documented, not hidden.
fn encode_algebraic(bytes: &mut Vec<u8>, alg: &Algebraic) {
    let target_bits = u64::from(DIGEST_ALGEBRAIC_BITS);
    let mut bits = target_bits + 8;
    let key = loop {
        let (lo, hi) = alg.bounds(bits);
        let lo_key = floor_scaled(&lo, target_bits);
        let hi_key = floor_scaled(&hi, target_bits);
        if lo_key == hi_key {
            break lo_key;
        }
        debug_assert!(
            bits < (1 << 24),
            "algebraic digest key failed to converge — an irrational value \
             should never sit exactly on a precision grid line"
        );
        bits *= 2;
    };
    bytes.push(b'A');
    write_sint(bytes, &key);
}

/// `floor(f * 2^target_bits)`, exactly (`BigInt` division, never `f64`).
fn floor_scaled(f: &Fraction, target_bits: u64) -> BigInt {
    let (num, den) = f.to_bigint_pair();
    (num << target_bits).div_floor(&den)
}
