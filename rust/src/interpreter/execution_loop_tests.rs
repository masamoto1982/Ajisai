//! What the per-Word bookkeeping in `execution_loop` must answer, and must
//! answer without unpacking the data it is asked about.
//!
//! Two questions run after *every* core word: "is the top of the stack a
//! collection?" (the hint-override rule) and "did this Word produce a reasoned
//! absence?" (the error-flow trace). Both used to be answered through
//! `Value::as_vector_view`, whose `Cow` is `Owned` for a `Tensor`: it rebuilt
//! the entire buffer as boxed per-lane `Value`s, twice per Word, and threw both
//! copies away. On a 4096-lane tensor that was 67% of all instructions
//! executed, and it scaled with the data rather than with the failures.
//!
//! Both now read the representation instead — the discriminant for the shape
//! question, [`crate::types::DenseTensor`]'s absence map for the reason. These
//! are ratio-free, wall-clock-free gates on the part that could silently
//! change: the *answers* must not depend on which representation holds the
//! value, because the fast path is only safe while they agree.

#[cfg(test)]
mod execution_loop_tests {
    use crate::error::NilReason;
    use crate::interpreter::error_flow_trace::ErrorFlowEventKind;
    use crate::interpreter::Interpreter;
    use crate::types::{Interpretation, ValueData};

    /// Every reason the trace recorded for a `NilProduced` event raised by
    /// `word`, in the order the run recorded them.
    async fn traced_reasons(source: &str, word: &str) -> Vec<Option<NilReason>> {
        let mut interp = Interpreter::new();
        interp
            .execute(source)
            .await
            .unwrap_or_else(|e| panic!("`{source}` must compute, got: {e:?}"));
        interp
            .drain_error_flow_trace()
            .iter()
            .filter(|event| {
                event.kind == ErrorFlowEventKind::NilProduced && event.word.as_deref() == Some(word)
            })
            .map(|event| event.absence.as_ref().and_then(|a| a.reason))
            .collect()
    }

    async fn top_is_dense_tensor(source: &str) -> bool {
        let mut interp = Interpreter::new();
        interp.execute(source).await.expect("must compute");
        matches!(
            interp.get_stack().as_slice().last().map(|v| &v.data),
            Some(ValueData::Tensor { .. })
        )
    }

    /// `MAP`ping a failing block over a numeric vector lands a *dense tensor*
    /// whose lanes are absent for a reason. The gate is the representation as
    /// much as the reason: if this stops being a `Tensor`, the test below stops
    /// covering the dense absence-map read and silently passes on the `Vector`
    /// walk instead.
    #[tokio::test]
    async fn a_lifted_failure_lands_in_a_dense_tensor() {
        assert!(
            top_is_dense_tensor("[ 1 2 3 4 5 6 7 8 ] [ 9 0 DIV ] MAP").await,
            "MAP over a numeric vector must produce a dense Tensor for the \
             dense-absence gates below to mean anything"
        );
    }

    /// The reason survives the read. `MAP`'s own `NilProduced` event describes
    /// the tensor it just produced, and the only record of *why* those lanes
    /// are absent is the tensor's absence map.
    #[tokio::test]
    async fn a_dense_tensors_absence_reason_reaches_the_trace() {
        assert_eq!(
            traced_reasons("[ 1 2 3 4 5 6 7 8 ] [ 9 0 DIV ] MAP", "MAP").await,
            vec![Some(NilReason::DivisionByZero)],
            "the reason a dense lane is absent must reach the trace from the \
             absence map, as it did from the materialized lanes"
        );
    }

    /// Same failure, same reason, whichever representation carries it. This is
    /// the invariant the dense read has to hold: `DIV` over two vectors keeps
    /// an AoS `Vector` and takes the lane walk, `MAP` lands a `Tensor` and
    /// takes the absence map, and an observer cannot tell which ran.
    #[tokio::test]
    async fn the_traced_reason_does_not_depend_on_the_representation() {
        let dense = traced_reasons("[ 1 2 3 4 5 6 7 8 ] [ 9 0 DIV ] MAP", "MAP").await;
        let nested = traced_reasons("[ 6 6 6 6 6 6 6 6 ] [ 0 0 0 0 0 0 0 0 ] DIV", "DIV").await;
        assert_eq!(
            dense, nested,
            "a dense tensor and a nested vector holding the same failure must \
             trace the same reason"
        );
        assert_eq!(dense, vec![Some(NilReason::DivisionByZero)]);
    }

    /// A domain miss reads the same way, so the gate is not pinned to one
    /// reason's spelling.
    #[tokio::test]
    async fn a_dense_domain_miss_reaches_the_trace_too() {
        assert_eq!(
            traced_reasons("[ 1 2 3 4 5 6 7 8 ] [ 4 -1 MUL SQRT ] MAP", "MAP").await,
            vec![Some(NilReason::DomainMiss)],
            "SQRT's domain miss must survive the dense absence-map read"
        );
    }

    /// `Literal` is the absence a Word *received*, not one it produced, so it
    /// is not an event — and a `NIL` written in source densifies into a tensor
    /// lane carrying exactly that reason. The dense read has to skip it for the
    /// same reason the lane walk did.
    #[tokio::test]
    async fn a_dense_literal_absence_is_not_traced_as_produced() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 1 NIL 3 4 5 6 7 8 ] 1 ADD")
            .await
            .expect("must compute");
        let produced: Vec<_> = interp
            .drain_error_flow_trace()
            .iter()
            .filter(|event| event.kind == ErrorFlowEventKind::NilProduced)
            .map(|event| event.word.clone())
            .collect();
        assert!(
            produced.is_empty(),
            "a literal NIL is propagated, not produced: {produced:?}"
        );
    }

    /// The hint-override rule the shape question exists for. `Interval` is a
    /// *number*'s presentation, so `SQRT` stamps it on a scalar result and
    /// leaves a collection's own role alone — the behavior that made the
    /// question necessary, now answered from the discriminant.
    #[tokio::test]
    async fn sqrt_stamps_interval_on_a_scalar_and_not_on_a_collection() {
        let mut scalar = Interpreter::new();
        scalar.execute("4 SQRT").await.expect("must compute");
        assert_eq!(
            scalar.get_stack().last_role(),
            Interpretation::Interval,
            "a scalar SQRT result carries the Interval role"
        );

        let mut collection = Interpreter::new();
        collection
            .execute("[ 4 9 16 25 36 49 64 81 ] SQRT")
            .await
            .expect("must compute");
        assert_ne!(
            collection.get_stack().last_role(),
            Interpretation::Interval,
            "a collection keeps the role it was built with; only its lanes are \
             numbers"
        );
    }
}
