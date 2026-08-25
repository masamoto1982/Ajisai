//! The Semantic Spine value model.

use std::sync::Arc;

use super::nil::NilReason;
use super::scalar::Scalar;

/// The six canonical value domains of Ajisai (SPEC `LANG.VALUES`), and the
/// *only* value shapes the language exposes.
///
/// What is deliberately absent here is as important as what is present. There
/// is no dense-tensor domain and no exact-scalar domain: both are storage
/// representations of an existing domain, held privately by [`Scalar`] (and, in
/// a later phase, by the vector representation), never surfaced as a separate
/// kind of value. There is no interpretation-role field on a value: how a value
/// is *displayed* is carried outside the spine as a
/// [`PresentationHint`](super::observation::PresentationHint), and it never
/// changes what a program computes.
#[derive(Clone, Debug, PartialEq)]
pub enum KernelValue {
    /// A single exact number. Its rational/exact-real backing is a private
    /// detail of [`Scalar`].
    Scalar(Scalar),
    /// A definite logical truth value. Distinct from any number: `TRUE` is not
    /// the scalar `1`.
    Boolean(bool),
    /// A first-class string of Unicode scalar values. Not a vector of
    /// codepoints wearing a role — a value domain in its own right.
    String(Arc<str>),
    /// An ordered sequence of values.
    Vector(Arc<[KernelValue]>),
    /// The absence value. `None` is a bare literal `NIL`; `Some(reason)` is a
    /// diagnostic absence carrying the minimal reason-centric model
    /// (migration plan §9). The *origin*, *recoverability*, and *diagnosis* of
    /// an absence live in diagnostic/trace state, not on the value.
    Nil(Option<NilReason>),
    /// A bare name — data until something executes it (SPEC
    /// `LANG.VALUES.DISJOINT`). Its own domain, reachable standalone as well
    /// as nested inside a Vector.
    ///
    /// This used to be a `CodeBlock` holding a token sequence, back when code
    /// was a domain of its own. The CodeBlock/Vector unification retired that:
    /// code is a Vector read as executable, not a separate shape, so the sixth
    /// domain is the Symbol a Vector element can be rather than the block a
    /// Vector could never be.
    Symbol(Arc<str>),
}
