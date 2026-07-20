//! Regression tests for the module-word semantic-role leak (SPEC §12.1) and
//! the JSON@PARSE failure contract (SPEC §11.2 / §15.2).
//!
//! The leak: a module word that consumed a Text operand and pushed its result
//! at the same stack position inherited the operand's `Text` role
//! position-wise, so a parsed Record rendered as `''`. These tests observe the
//! fix through the value protocol (protocol strings, per §15.2), never
//! through Rust `Debug` output.

use crate::error::NilReason;
use crate::interpreter::Interpreter;
use crate::types::value_protocol::{
    interpretation_protocol_str, value_to_protocol, ProtocolNode, ProtocolValue,
};
use crate::types::{display::format_with_hint, Interpretation};

/// Protocol node for the top stack entry, rendered under the semantic-plane
/// role for its position — exactly what the WASM boundary and `--json` CLI
/// serialize for consumers.
fn top_protocol_node(interp: &Interpreter) -> ProtocolNode {
    let value = interp.get_stack().last().expect("stack must not be empty");
    let hint = interp
        .collect_stack_hints()
        .last()
        .copied()
        .unwrap_or(Interpretation::Unassigned);
    value_to_protocol(value, Some(hint))
}

/// Phase 4 role-ownership regression guard: until the stack abstraction owns
/// `(value, role)` as one unit, every observable execution boundary must keep
/// the two legacy vectors position-aligned.
fn assert_stack_hints_aligned(interp: &Interpreter) {
    assert_eq!(
        interp.get_stack().len(),
        interp.collect_stack_hints().len(),
        "stack values and semantic-plane roles must remain position-aligned"
    );
}

#[tokio::test]
async fn parse_record_role_is_record_not_text() {
    let mut interp = Interpreter::new();
    interp
        .execute(r#"'JSON' IMPORT '{"a":1}' JSON@PARSE"#)
        .await
        .unwrap();
    let node = top_protocol_node(&interp);
    // The consumed input string's Text role must not leak onto the Record.
    assert_ne!(
        interpretation_protocol_str(node.display_hint),
        "text",
        "parsed record must not carry the consumed input's text role"
    );
    let value = interp.get_stack().last().unwrap();
    assert_eq!(value.semantic_kind().as_protocol_str(), "record");
    // Under the leaked Text role the record serialized as an empty string;
    // structurally it must serialize as a vector of pairs.
    assert_eq!(node.type_str, "vector");
    assert!(matches!(node.value, ProtocolValue::Children(_)));
}

#[tokio::test]
async fn parse_keys_role_is_vector_of_text() {
    let mut interp = Interpreter::new();
    interp
        .execute(r#"'JSON' IMPORT '{"a":1,"b":2}' JSON@PARSE JSON@KEYS"#)
        .await
        .unwrap();
    let node = top_protocol_node(&interp);
    assert_ne!(interpretation_protocol_str(node.display_hint), "text");
    assert_eq!(node.type_str, "vector");
    let ProtocolValue::Children(keys) = &node.value else {
        panic!("keys must serialize as a vector, got {}", node.type_str);
    };
    let rendered: Vec<&str> = keys
        .iter()
        .map(|k| match &k.value {
            ProtocolValue::Text(s) => s.as_str(),
            other => panic!("key must serialize as protocol string, got {other:?}"),
        })
        .collect();
    assert_eq!(rendered, ["a", "b"]);
}

#[tokio::test]
async fn parse_result_stack_display_is_structural() {
    let mut interp = Interpreter::new();
    interp
        .execute(r#"'JSON' IMPORT '{"a":1}' JSON@PARSE"#)
        .await
        .unwrap();
    let value = interp.get_stack().last().unwrap();
    let hint = *interp.collect_stack_hints().last().unwrap();
    assert_eq!(format_with_hint(value, hint), "[ [ 'a' 1/1 ] ]");
}

#[tokio::test]
async fn keep_mode_preserves_untouched_slot_roles() {
    let mut interp = Interpreter::new();
    interp
        .execute(r#"'JSON' IMPORT '{"a":1}' ,, JSON@PARSE"#)
        .await
        .unwrap();
    let hints = interp.collect_stack_hints();
    assert_eq!(hints.len(), 2);
    // The retained input keeps its Text role; only the new result slot is
    // (re)derived from the constructed value.
    assert_eq!(hints[0], Interpretation::Text);
    assert_ne!(hints[1], Interpretation::Text);
}

#[tokio::test]
async fn cf_retag_updates_only_target_stack_slot_role() {
    let mut interp = Interpreter::new();
    interp.execute("'anchor' 5/2 >CF").await.unwrap();

    assert_stack_hints_aligned(&interp);
    let stack = interp.get_stack();
    let hints = interp.collect_stack_hints();
    assert_eq!(stack.len(), 2);
    assert_eq!(hints[0], Interpretation::Text);
    assert_eq!(hints[1], Interpretation::ContinuedFraction);
    assert_eq!(format_with_hint(&stack[0], hints[0]), "'anchor'");
    assert_eq!(format_with_hint(&stack[1], hints[1]), "( 2 ( 2 ) )");
}

/// A module word must not re-derive roles for lower slots it did not consume.
/// This is the regression that the pre-migration fingerprint path protects and
/// that the Phase 4 stack-owned-role implementation must preserve without
/// relying on value or `Arc` identity.
#[tokio::test]
async fn module_word_preserves_lower_cf_retag() {
    let mut interp = Interpreter::new();
    interp
        .execute(r#"'JSON' IMPORT 5/2 >CF '{"a":1}' JSON@PARSE"#)
        .await
        .unwrap();

    assert_stack_hints_aligned(&interp);
    let stack = interp.get_stack();
    let hints = interp.collect_stack_hints();
    assert_eq!(stack.len(), 2);
    assert_eq!(hints[0], Interpretation::ContinuedFraction);
    assert_eq!(format_with_hint(&stack[0], hints[0]), "( 2 ( 2 ) )");
    assert_ne!(hints[1], Interpretation::ContinuedFraction);
    assert_eq!(stack[1].semantic_kind().as_protocol_str(), "record");
}

#[tokio::test]
async fn cond_keep_preserves_outer_slot_role_alignment() {
    let mut interp = Interpreter::new();
    interp
        .execute("[ -5 ] ,, { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND")
        .await
        .unwrap();

    assert_stack_hints_aligned(&interp);
    let hints = interp.collect_stack_hints();
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0], Interpretation::RawNumber);
    assert_eq!(hints[1], Interpretation::Text);
}

#[tokio::test]
async fn nil_passthrough_keeps_reason_and_effective_role_alignment() {
    let mut interp = Interpreter::new();
    interp.execute("1 0 /").await.unwrap();

    assert_stack_hints_aligned(&interp);
    let value = interp.get_stack().last().unwrap();
    assert!(value.is_nil());
    assert!(
        value.nil_reason().is_some(),
        "division by zero must keep a structured NIL reason"
    );
    let node = top_protocol_node(&interp);
    // Arithmetic NIL passthrough keeps the effective operand/result role while
    // carrying absence metadata in the value payload. Phase 4 must preserve
    // that existing stack-position role behavior unless SPEC changes it.
    assert_eq!(node.display_hint, Interpretation::RawNumber);
}

#[tokio::test]
async fn time_parse_iso_does_not_inherit_text_role() {
    let mut interp = Interpreter::new();
    interp
        .execute("'TIME' IMPORT '2024-01-02T03:04:05' TIME@PARSE-ISO")
        .await
        .unwrap();
    let node = top_protocol_node(&interp);
    // Same path-level leak as JSON@PARSE: the consumed ISO text's role must
    // not garble the datetime vector into a bogus string.
    assert_ne!(interpretation_protocol_str(node.display_hint), "text");
    assert_eq!(node.type_str, "vector");
}

#[tokio::test]
async fn parse_failure_is_reasoned_invalid_encoding_bubble() {
    let mut interp = Interpreter::new();
    interp
        .execute("'JSON' IMPORT 'not json' JSON@PARSE")
        .await
        .unwrap();
    let value = interp.get_stack().last().unwrap();
    assert!(value.is_nil());
    let reason = value
        .nil_reason()
        .expect("PARSE failure must attach a structured reason (§15.2)");
    assert_eq!(reason.as_protocol_str(), "invalidEncoding");
    assert_eq!(reason, &NilReason::InvalidEncoding);
}

#[tokio::test]
async fn parse_failure_reason_observable_via_nil_reason_word() {
    let mut interp = Interpreter::new();
    interp
        .execute("'JSON' IMPORT 'not json' JSON@PARSE NIL-REASON")
        .await
        .unwrap();
    let value = interp.get_stack().last().unwrap();
    let hint = *interp.collect_stack_hints().last().unwrap();
    assert_eq!(format_with_hint(value, hint), "'invalidEncoding'");
}

#[tokio::test]
async fn parse_failure_has_no_output_effect() {
    let mut interp = Interpreter::new();
    interp
        .execute("'JSON' IMPORT 'not json' JSON@PARSE")
        .await
        .unwrap();
    // PARSE is registered Pure (JSON_WORDS): a failed parse must be reported
    // only through the reasoned Bubble/NIL, never via the output buffer.
    assert!(
        interp.output_buffer.is_empty(),
        "Pure JSON@PARSE must not write to output_buffer, got {:?}",
        interp.output_buffer
    );
}
