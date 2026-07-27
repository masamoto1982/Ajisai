//! # ajisai-audit
//!
//! Content addressing and execution receipts for Ajisai, as an external
//! package.
//!
//! ## What this is not part of
//!
//! None of this is in the language. A digest is not part of a word's identity;
//! two words with the same name and body are the same word whether or not
//! anybody has hashed them. A receipt is not part of an execution's result;
//! `1 2 ADD` leaves `3` and nothing else. An attestation is not a conformance
//! condition; an implementation that has never heard of this crate is a
//! completely conforming Ajisai.
//!
//! Ajisai Core does not depend on this crate, contains no hook for it, and has
//! no idea it exists. Everything below is built out of Core's ordinary public
//! API: [`ajisai_core::syntax::parse`], [`ajisai_core::syntax::render_program`],
//! and the interpreter's rendered flow.
//!
//! ## What it is
//!
//! Two things, and both of them are ordinary data:
//!
//! * a **digest** of a program, taken over its canonical form rather than its
//!   source text, so that whitespace, comments, and the choice between `+` and
//!   `ADD` do not change it;
//! * a **receipt** recording what was run and what came out, which a caller
//!   may keep, compare, or throw away.
//!
//! ## Stability
//!
//! This package sets its own policy. The digest construction below is
//! versioned by [`DIGEST_DOMAIN`]; changing how a digest is computed is a
//! breaking change *for this package*, and has no bearing on the language.

pub mod sha256;

use ajisai_core::{syntax, Interpreter};

/// The domain separator mixed into every digest.
///
/// It names what is being hashed and pins the construction. If the canonical
/// form or the framing below ever changes, this string changes with it, so an
/// old digest can never silently appear to match a new one.
pub const DIGEST_DOMAIN: &str = "ajisai-audit/program-digest/1";

/// A program's content address: the digest of its canonical form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Digest {
    /// Lowercase hexadecimal SHA-256.
    pub hex: String,
    /// The canonical form the digest was taken over. Kept so that a mismatch
    /// can be explained rather than merely reported.
    pub canonical: String,
}

/// A record of one execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub digest: Digest,
    /// The rendered flow, bottom first, if the program completed.
    pub flow: Option<Vec<String>>,
    /// The error, rendered, if it did not.
    pub error: Option<String>,
}

impl Receipt {
    pub fn succeeded(&self) -> bool {
        self.error.is_none()
    }
}

/// The content address of a program.
///
/// The digest is taken over the *canonical form* — the program tree rendered
/// back to source with canonical word names — so equivalent spellings address
/// the same content:
///
/// ```
/// let a = ajisai_audit::digest("1 2 +").unwrap();
/// let b = ajisai_audit::digest("  1  2  ADD  # a comment\n").unwrap();
/// assert_eq!(a.hex, b.hex);
/// ```
pub fn digest(source: &str) -> ajisai_core::Result<Digest> {
    let program = syntax::parse(source)?;
    let canonical = syntax::render_program(&program);
    // Length-prefixed framing so that no combination of domain and canonical
    // text can be confused with another.
    let framed = format!("{DIGEST_DOMAIN}\n{}\n{canonical}", canonical.len());
    Ok(Digest {
        hex: sha256::hex(framed.as_bytes()),
        canonical,
    })
}

/// Run a program on a fresh interpreter and record what happened.
///
/// The interpreter is an ordinary [`Interpreter`]; nothing about it is in
/// audit mode, and running the same program without this function produces the
/// same flow.
pub fn run_with_receipt(source: &str) -> ajisai_core::Result<Receipt> {
    let digest = digest(source)?;
    let mut interpreter = Interpreter::new();
    match interpreter.execute(source) {
        Ok(()) => Ok(Receipt {
            digest,
            flow: Some(ajisai_core::render_stack(&interpreter)),
            error: None,
        }),
        Err(error) => Ok(Receipt {
            digest,
            flow: None,
            error: Some(error.to_string()),
        }),
    }
}

/// Whether a program still addresses a digest recorded earlier.
pub fn verify(source: &str, expected_hex: &str) -> ajisai_core::Result<bool> {
    Ok(digest(source)?.hex == expected_hex)
}
