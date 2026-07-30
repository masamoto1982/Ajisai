//! The Semantic Spine absence model.
//!
//! The spine's NIL is reason-centric and minimal (migration plan §9): a NIL
//! carries at most a [`NilReason`]. The *origin* of an absence, its
//! *recoverability*, and any structured *diagnosis* are not part of the value —
//! they belong to diagnostic/trace state observed separately. Keeping them off
//! the value means "what a NIL is" and "how this NIL came to be" cannot drift
//! into each other.
//!
//! The reason enum itself is shared with the rest of the crate rather than
//! duplicated: there is one authoritative list of absence reasons.

pub use crate::error::NilReason;
