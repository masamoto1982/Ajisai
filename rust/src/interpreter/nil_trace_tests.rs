//! What the per-Word bookkeeping in `nil_trace` must answer, and must answer
//! without unpacking the data it is asked about.
//!
//! One question runs after *every* Word: "did this Word produce a reasoned
//! absence?" (the error-flow trace). Two things decide it. *Whether* the Word
//! produced one is the mint count (`crate::semantic::minted_absence_count`):
//! a NIL that arrived in an operand and left in the result was carried, not
//! produced (LANG.FAILURE.PASSTHROUGH). *Which* reason it produced is read off
//! the result — and for a dense tensor, off its absence map rather than by
//! rebuilding every lane as a boxed `Value`, which on a 4096-lane tensor was
//! 67% of all instructions executed and scaled with the data rather than the
//! failures. These are ratio-free, wall-clock-free gates on the parts that
//! could silently change.

#[cfg(test)]
mod nil_trace_tests {
    use crate::error::NilReason;
    use crate::interpreter::error_flow_trace::ErrorFlowEventKind;
    use crate::interpreter::nil_trace::projected_nil_reason;
    use crate::interpreter::Interpreter;
    use crate::semantic::Recoverability;
    use crate::types::{Value, ValueData};

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

    fn number(n: i64) -> Value {
        Value::from_number(crate::types::fraction::Fraction::new(n.into(), 1.into()))
    }

    /// `MAP`ping a failing block over a numeric vector lands a *dense tensor*
    /// whose lanes are absent for a reason. The gate is the representation as
    /// much as the reason: if this stops being a `Tensor`, the reads below stop
    /// covering the dense absence map and silently pass on the `Vector` walk
    /// instead.
    #[tokio::test]
    async fn a_lifted_failure_lands_in_a_dense_tensor() {
        assert!(
            top_is_dense_tensor("[ 1 2 3 4 5 6 7 8 ] [ 9 0 DIV ] MAP").await,
            "MAP over a numeric vector must produce a dense Tensor for the \
             dense-absence gates below to mean anything"
        );
    }

    /// The reason survives the read: a reasoned absent lane densified into a
    /// tensor is read back from the absence map, the same answer the
    /// materialized lane walk gave.
    #[test]
    fn a_dense_tensors_absence_reason_is_read_from_its_map() {
        let lanes = vec![
            number(1),
            Value::nil_with_reason(NilReason::DivisionByZero, Recoverability::Recoverable),
            number(3),
        ];
        let dense = Value::from_vector_promoted(lanes.clone());
        assert!(
            matches!(dense.data, ValueData::Tensor { .. }),
            "numeric lanes with an absent one must densify"
        );
        assert_eq!(
            projected_nil_reason(&dense),
            Some(NilReason::DivisionByZero)
        );
        // Same failure, same reason, whichever representation carries it.
        let boxed = Value::from_vector(lanes);
        assert_eq!(projected_nil_reason(&boxed), projected_nil_reason(&dense));
    }

    /// The Word that produced a lane's absence is the one that ran the
    /// projection, not the Word whose result carries it: `MAP` lands a dense
    /// tensor whose lanes `DIV` made absent, so `DIV` is the producer — once per
    /// lane it answered — and `MAP` is the frame it happened in. `MAP` used to
    /// record the tensor as its own production, and a reader was sent to the
    /// wrong Word.
    #[tokio::test]
    async fn the_word_that_projected_the_lane_is_the_producer() {
        let source = "[ 1 2 3 4 5 6 7 8 ] [ 4 -1 MUL SQRT ] MAP";
        assert!(top_is_dense_tensor(source).await);
        assert_eq!(traced_reasons(source, "MAP").await, Vec::new());
        assert_eq!(
            traced_reasons(source, "SQRT").await,
            vec![Some(NilReason::DomainMiss); 8]
        );
    }

    /// A Word lifted over its operand projects per lane and is the producer
    /// itself: the reason is read off the result it built.
    #[tokio::test]
    async fn a_lifted_word_that_projects_is_the_producer() {
        assert_eq!(
            traced_reasons("[ 1 4 -1 9 ] SQRT", "SQRT").await,
            vec![Some(NilReason::DomainMiss)]
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

    /// A reasoned absent lane carried through a lifted Word is not produced
    /// again by it: `ADD` over the tensor `MAP` left passes every absent lane
    /// through (LANG.FAILURE.PASSTHROUGH per lane) and mints nothing.
    #[tokio::test]
    async fn a_dense_reasoned_absence_passed_through_is_not_traced_again() {
        assert_eq!(
            traced_reasons("[ 1 2 3 4 5 6 7 8 ] [ 9 0 DIV ] MAP 1 ADD", "ADD").await,
            Vec::new()
        );
    }
}
