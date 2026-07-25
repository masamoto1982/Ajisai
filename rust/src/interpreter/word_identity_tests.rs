//! Tests for the §8.6 content digest.
//!
//! The digest is a published cryptographic hash, so its correctness is checked
//! against BLAKE3's own test vectors rather than against whatever this build
//! happens to produce. The vectors below are the official ones from the BLAKE3
//! reference repository's `test_vectors.json`: the empty input, `"abc"`, and
//! two lengths that straddle the 1024-byte chunk boundary — the point where the
//! implementation switches from hashing one chunk to combining a chunk tree, so
//! a backend that got the tree wrong would still pass short-input tests.
//!
//! Official inputs of length `n` are the byte sequence `i % 251` for
//! `i in 0..n`, which `pattern` reproduces.

use super::word_identity::content_digest;

/// The official test-vector input of a given length.
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// `content_digest` prefixes the raw hex with `#`; the vectors do not.
fn digest_hex(bytes: &[u8]) -> String {
    let digest = content_digest(bytes);
    digest
        .strip_prefix('#')
        .expect("content_digest is `#`-prefixed")
        .to_string()
}

#[test]
fn matches_blake3_vector_for_the_empty_input() {
    assert_eq!(
        digest_hex(b""),
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    );
}

#[test]
fn matches_blake3_vector_for_abc() {
    assert_eq!(
        digest_hex(b"abc"),
        "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
    );
}

#[test]
fn matches_blake3_vector_at_the_chunk_boundary() {
    assert_eq!(
        digest_hex(&pattern(1024)),
        "42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7"
    );
}

#[test]
fn matches_blake3_vector_past_the_chunk_boundary() {
    assert_eq!(
        digest_hex(&pattern(1025)),
        "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444"
    );
}

#[test]
fn digest_shape_is_a_hash_marker_and_64_lowercase_hex() {
    for input in [b"".as_slice(), b"abc".as_slice(), &pattern(4096)] {
        let digest = content_digest(input);
        assert_eq!(digest.len(), 65, "digest {digest} is not `#` + 64 hex");
        assert!(digest.starts_with('#'));
        assert!(
            digest[1..]
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "digest {digest} is not lowercase hex"
        );
    }
}

#[test]
fn distinct_inputs_get_distinct_digests() {
    // Length extension and single-bit changes both have to move the digest;
    // the retired polynomial hash is what this guards against returning.
    assert_ne!(content_digest(b"AB"), content_digest(b"AC"));
    assert_ne!(content_digest(b"AB"), content_digest(b"ABA"));
    assert_ne!(content_digest(b""), content_digest(b"\0"));
}

#[test]
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
fn identity_algorithm_names_the_hash_actually_used() {
    // The name recorded in lockfiles and receipts has to track the function
    // that actually produced the digests above, not drift from it.
    assert_eq!(super::word_identity::IDENTITY_ALGORITHM, "blake3");
}
