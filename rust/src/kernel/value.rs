//! The Semantic Spine value model.

use std::sync::Arc;

use crate::types::Token;

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
    /// A quoted, unevaluated block of code.
    CodeBlock(CodeBlock),
}

/// A quoted, unevaluated token sequence (SPEC `LANG.VALUES.CODEBLOCK`).
#[derive(Clone, Debug, PartialEq)]
pub struct CodeBlock {
    tokens: Arc<[Token]>,
}

impl CodeBlock {
    pub fn new(tokens: Arc<[Token]>) -> Self {
        Self { tokens }
    }

    /// The block's tokens, in source order.
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
}
