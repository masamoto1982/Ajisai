//! What an error report keeps when the failing stack is too large to send.
//!
//! An error report carries two different things, and they are not equally
//! important. The **diagnosis** is the answer: why the program stopped, which
//! ceiling it met, what to do next. The **stack** is residual state: whatever
//! the program happened to be holding when it stopped. Serializing the residue
//! in full and letting the whole envelope exceed a host's response ceiling
//! trades the answer for the residue, which is backwards.
//!
//! It was not hypothetical. `1 21000 RANGE 1 [ MUL ] FOLD` is refused by the
//! work meter with `numericWork of 10000573 exceeds the limit of 10000000` —
//! precisely the diagnosis an agent needs — but the failing stack holds a
//! 21,000-element vector and an 81,649-digit partial product, so the envelope
//! came to 5,773,682 bytes of which 5,571,973 were the stack. Against the MCP
//! adapter's 1 MiB `responseBytes` ceiling that became
//! `hostError: responseTooLarge`, and the agent was told its *answer* was too
//! big rather than that it had exceeded the *work budget* — which points it at
//! shrinking output when the fix is to compute less. `numericWork` could not be
//! reclassified from `injectedLimit` to `boundary` for the same reason: a
//! control whose diagnosis does not survive the wire is not observable.
//!
//! So an error report elides the value payload of the slots it cannot afford,
//! and says that it did. Three rules make that honest:
//!
//! 1. **Only errors.** A successful result *is* its stack; truncating it would
//!    change the answer, and an oversized success is correctly reported as
//!    `responseTooLarge` so the caller can ask for less.
//! 2. **Values are dropped, never reasons.** `diagnosis`, `aiDiagnostic`,
//!    `errorFlowTrace`, `message` and `runtimeMetrics` are never touched.
//! 3. **Every slot stays in place.** An elided slot keeps its index, `type`,
//!    `semantics` (less an algebraic value's `exactTerms`), and gains an `elided` record naming what
//!    was dropped. Positions stay meaningful, so a diagnosis that points at
//!    stack depth still points at the same thing.
//!
//! This bounds what is *sent*, not what is built; the generative ceiling
//! (`maxMaterializedElements`) is what bounds the latter. And it is a
//! best-effort preservation, not a guarantee of delivery: a host whose response
//! ceiling is below [`MAX_ERROR_STACK_BYTES`] still refuses the result, and
//! `responseBytes` remains the hard gate.

use serde_json::{json, Map, Value as Json};

use crate::interpreter::Interpreter;
use crate::types::value_protocol::{value_to_protocol, ProtocolNode, ProtocolValue};
use crate::types::ValueData;

use super::report::{protocol_node_json, semantics_json};

/// Byte budget for an error report's `stack` and `stackDisplay` payload,
/// together.
///
/// One sixteenth of the 1 MiB `responseBytes` ceiling the MCP host profile
/// declares, which leaves the diagnosis, the flow trace and the adapter's own
/// envelope fields room to be pathological and still arrive. Ordinary errors
/// are nowhere near it — an unknown-Word diagnosis is about 15 KiB in total —
/// so nothing an agent normally sees changes by a byte.
pub(super) const MAX_ERROR_STACK_BYTES: usize = 64 * 1024;

/// The stack of a failed run: slots the budget could afford, rendered in full;
/// the rest kept in place with their values dropped.
pub(super) struct ElidedStack {
    pub stack: Json,
    pub stack_display: Vec<String>,
    /// The envelope's `stackElided` record, or `None` when everything fit and
    /// the report is byte-for-byte what it always was.
    pub elided: Option<Json>,
}

pub(super) fn elided_error_stack(interp: &Interpreter) -> ElidedStack {
    // The protocol node is built for every slot, because it is what says which
    // domain the value belonged to and an elided slot still reports that. What
    // is *not* built for a slot the budget cannot afford is its JSON and its
    // display string — which is where the bytes are. Deciding from the node
    // instead of from the serialized text is the difference between throwing
    // away 27 MB and never building it: `0 99999 RANGE LENGHT` spent 1.5 s
    // rendering a stack it was about to discard, close enough to `wallTimeMs`
    // that a slow host would have seen a timeout instead of its typo.
    let slots: Vec<ProtocolNode> = interp.get_stack().iter().map(value_to_protocol).collect();
    let costs: Vec<usize> = slots.iter().map(node_wire_bytes).collect();

    // Fill from the top down. The operands a failure names are the ones nearest
    // the top, so when the budget cannot hold everything it is the top that is
    // worth holding. Lower slots that still fit in what remains are kept, so a
    // single enormous slot does not cost the small ones below it.
    let mut remaining = MAX_ERROR_STACK_BYTES;
    let mut keep = vec![false; slots.len()];
    for index in (0..slots.len()).rev() {
        if costs[index] <= remaining {
            remaining -= costs[index];
            keep[index] = true;
        }
    }
    let all_kept = keep.iter().all(|kept| *kept);

    let mut stack = Vec::with_capacity(slots.len());
    let mut stack_display = Vec::with_capacity(slots.len());
    let mut elided_slots = Vec::new();
    for (index, node) in slots.iter().enumerate() {
        if keep[index] {
            stack.push(protocol_node_json(node));
            stack_display.push(render_slot(interp, index));
            continue;
        }
        let elements = element_count(node);
        stack.push(elided_node_json(node, costs[index], elements));
        // A text-only client reads `stackDisplay` and nothing else, so the
        // marker has to carry the same facts the structured record does.
        stack_display.push(match elements {
            Some(elements) => format!(
                "<elided {} of {} elements, ~{} bytes>",
                node.type_str, elements, costs[index]
            ),
            None => format!("<elided {}, ~{} bytes>", node.type_str, costs[index]),
        });
        let mut record = Map::new();
        record.insert("index".into(), json!(index));
        record.insert("approxBytes".into(), json!(costs[index]));
        if let Some(elements) = elements {
            record.insert("elements".into(), json!(elements));
        }
        elided_slots.push(Json::Object(record));
    }

    ElidedStack {
        stack: Json::Array(stack),
        stack_display,
        elided: (!all_kept).then(|| {
            json!({
                "reason": "errorStackBudget",
                "budgetBytes": MAX_ERROR_STACK_BYTES,
                "slots": elided_slots,
            })
        }),
    }
}

/// The display string for one slot, rendered only when the slot is kept.
fn render_slot(interp: &Interpreter, index: usize) -> String {
    interp
        .get_stack()
        .get(index)
        .map(crate::types::Value::to_string)
        .unwrap_or_default()
}

/// Bytes one protocol node adds around its own value: the `semantics` block,
/// the `type` string, and the punctuation between them.
///
/// Measured against `protocol_node_json`, and deliberately applied to interior
/// nodes too even though those carry no `semantics` — over-estimating elides a
/// little sooner, which is the safe direction for a budget whose whole job is
/// to keep a diagnosis deliverable.
const NODE_ENVELOPE_BYTES: usize = 256;

/// Serialized size of a slot, estimated from the node rather than from the
/// text — so a slot about to be discarded is never serialized to find out how
/// big it was.
///
/// The payload is counted twice because a slot is sent twice: once as a value
/// in `stack` and once as text in `stackDisplay`, and the budget covers both.
/// Counting it once let an 81,649-digit integer through on the strength of its
/// `stack` node alone, and the display of the same number then doubled the
/// report.
fn node_wire_bytes(node: &ProtocolNode) -> usize {
    NODE_ENVELOPE_BYTES + 2 * node_payload_bytes(node)
}

/// Bytes one algebraic term costs in `semantics.exactTerms`: a numerator, a
/// denominator and a radicand, quoted and keyed. Measured at 62 on a
/// 512-term value.
const EXACT_TERM_BYTES: usize = 64;

fn node_payload_bytes(node: &ProtocolNode) -> usize {
    let value_bytes = match &node.value {
        ProtocolValue::Null => 4,
        ProtocolValue::Bool(_) => 5,
        ProtocolValue::Text(text) => text.len() + 2,
        ProtocolValue::Number {
            numerator,
            denominator,
        } => numerator.len() + denominator.len() + 32,
        ProtocolValue::Children(children) => children
            .iter()
            .map(|child| NODE_ENVELOPE_BYTES / 2 + node_payload_bytes(child))
            .sum(),
        ProtocolValue::Record { keys, values } => {
            keys.iter()
                .chain(values.iter())
                .map(|child| NODE_ENVELOPE_BYTES / 2 + node_payload_bytes(child))
                .sum::<usize>()
                + 24
        }
    };
    // An algebraic value's `value` is its *approximate* rational — small — while
    // the number itself lives in `semantics.exactTerms`, which is the opposite
    // of small. Estimating the semantics block at a flat constant let eighteen
    // 512-term values through a 64 KiB budget as if they were 300 bytes each,
    // and the report came to 387 KB.
    value_bytes + exact_terms_bytes(node)
}

/// Terms an algebraic value carries, for the record of what was dropped: with
/// `exactTerms` gone from an elided slot, this is what says how much there was.
fn algebraic_term_count(node: &ProtocolNode) -> Option<usize> {
    match &node.semantics.data {
        ValueData::ExactScalar(exact) => Some(exact.algebraic_term_count()),
        _ => None,
    }
}

fn exact_terms_bytes(node: &ProtocolNode) -> usize {
    match &node.semantics.data {
        ValueData::ExactScalar(exact) => exact.algebraic_term_count() * EXACT_TERM_BYTES,
        _ => 0,
    }
}

/// An elided slot: everything the full node said about *what kind of value*
/// this was, and nothing of the value itself.
///
/// `value` is `null` rather than the node being dropped, because dropping it
/// would renumber every slot above it and silently move what a diagnosis points
/// at. A reader tells this apart from a genuine `NIL` by `type` (which still
/// names the real domain) and by the presence of `elided`.
fn elided_node_json(node: &ProtocolNode, approx_bytes: usize, elements: Option<usize>) -> Json {
    let mut obj = Map::new();
    // For an algebraic value the number itself lives in
    // `semantics.exactTerms`, not in `value` — `value` is only the marked
    // approximation. Eliding `value` and keeping `semantics` therefore
    // dropped the cheap half and kept the expensive one: eighteen 512-term
    // values still came to 388 KB with seventeen of them "elided". What a
    // reader needs from a dropped slot is what kind of value it was, which
    // is everything in the block except the exact form; the term count goes
    // into the `elided` record instead, so nothing is silently missing.
    let mut semantics = semantics_json(&node.semantics);
    if let Some(object) = semantics.as_object_mut() {
        object.remove("exactTerms");
    }
    obj.insert("semantics".into(), semantics);
    obj.insert("type".into(), json!(node.type_str));
    obj.insert("value".into(), Json::Null);
    let mut record = Map::new();
    record.insert("reason".into(), json!("errorStackBudget"));
    record.insert("approxBytes".into(), json!(approx_bytes));
    if let Some(elements) = elements {
        record.insert("elements".into(), json!(elements));
    }
    if let Some(terms) = algebraic_term_count(node) {
        record.insert("algebraicTerms".into(), json!(terms));
    }
    obj.insert("elided".into(), Json::Object(record));
    Json::Object(obj)
}

/// Direct children of a composite node, or `None` for a leaf — the one fact
/// about a dropped collection that its `semantics` block does not carry.
fn element_count(node: &ProtocolNode) -> Option<usize> {
    match &node.value {
        ProtocolValue::Children(children) => Some(children.len()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    //! An error report must deliver its diagnosis, whatever the failing stack was
    //! holding.
    //!
    //! The case these exist for: the work meter refuses
    //! `1 21000 RANGE 1 [ MUL ] FOLD` and names `numericWork`, but the stack at
    //! that moment holds a 21,000-element vector and an 81,649-digit partial
    //! product. Serialized in full that is 5.7 MB, which a host response ceiling
    //! turns into "your answer was too big" — the opposite of what the program
    //! actually did wrong.

    use crate::agent::api::ComputeOptions;
    use crate::test_support::agent_json;
    use serde_json::Value as Json;

    fn compact_len(report: &Json) -> usize {
        serde_json::to_string(report)
            .expect("a report serializes")
            .len()
    }

    /// The source whose refusal used to be unreportable.
    const RUNAWAY_FOLD: &str = "1 21000 RANGE 1 [ MUL ] FOLD";

    #[tokio::test]
    async fn a_refusal_reports_its_resource_rather_than_its_residue() {
        let report = agent_json(RUNAWAY_FOLD, ComputeOptions::agent(None)).await;
        assert_eq!(report["status"], "error");
        assert_eq!(
            report["diagnosis"]["resourceLimit"]["resource"], "numericWork",
            "the ceiling that fired must survive to the wire"
        );
        assert!(
            compact_len(&report) < 64 * 1024,
            "a refusal must be deliverable; got {} bytes",
            compact_len(&report)
        );
    }

    #[tokio::test]
    async fn an_elided_slot_says_what_it_dropped() {
        let report = agent_json(RUNAWAY_FOLD, ComputeOptions::agent(None)).await;
        let elided = &report["stackElided"];
        assert_eq!(elided["reason"], "errorStackBudget");
        assert_eq!(
            elided["slots"][0]["index"], 0,
            "the record names which slots were dropped"
        );
        assert_eq!(
            elided["slots"][0]["elements"], 21000,
            "and how much was in them"
        );
        assert!(
            elided["slots"][0]["approxBytes"].as_u64().unwrap() > 1_000_000,
            "and roughly what they would have cost"
        );
    }

    #[tokio::test]
    async fn an_elided_slot_keeps_its_position_and_its_kind() {
        let report = agent_json(RUNAWAY_FOLD, ComputeOptions::agent(None)).await;
        let stack = report["stack"].as_array().expect("an array");
        assert_eq!(
            stack.len(),
            report["stackDisplay"].as_array().unwrap().len(),
            "the two views of the stack stay aligned"
        );
        // Dropping the slot outright would renumber everything above it and
        // silently move whatever a diagnosis points at.
        assert_eq!(stack[0]["type"], "vector", "the domain is still named");
        assert!(stack[0]["value"].is_null(), "the value is what goes");
        assert_eq!(stack[0]["elided"]["reason"], "errorStackBudget");
        // The block that was applied is small, so it is still there in full:
        // the budget drops what it must, not everything.
        assert_eq!(
            report["stackDisplay"][2], "[ MUL ]",
            "an affordable slot is untouched"
        );
    }

    #[tokio::test]
    async fn a_text_only_client_still_learns_what_was_there() {
        let report = agent_json(RUNAWAY_FOLD, ComputeOptions::agent(None)).await;
        let marker = report["stackDisplay"][0].as_str().expect("a string");
        assert!(
            marker.contains("elided") && marker.contains("21000"),
            "`stackDisplay` is the whole result for a text-only client, got: {marker}"
        );
    }

    #[tokio::test]
    async fn an_ordinary_error_is_not_elided_at_all() {
        // The budget must be invisible to the errors an agent actually meets.
        for source in [
            "[ 1 2 3 ] LENGHT",
            "1 0 DIV 2 UNKNOWNWORD",
            "[ 1 2 ] [ 1 2 3 ] ADD",
        ] {
            let report = agent_json(source, ComputeOptions::agent(None)).await;
            assert_eq!(report["status"], "error", "`{source}` must fail");
            assert!(
                report["stackElided"].is_null(),
                "`{source}` must report no elision"
            );
            for node in report["stack"].as_array().expect("an array") {
                assert!(
                    node.get("elided").is_none(),
                    "`{source}` must carry its stack in full"
                );
            }
        }
    }

    #[tokio::test]
    async fn a_successful_result_is_never_elided() {
        // A success *is* its stack. Truncating it would change the answer, so
        // an oversized success stays oversized and the host says so.
        let report = agent_json("0 20000 RANGE", ComputeOptions::agent(None)).await;
        assert_eq!(report["status"], "ok");
        assert!(report["stackElided"].is_null());
        assert!(
            compact_len(&report) > 1_000_000,
            "the success path is deliberately untouched"
        );
    }

    /// Eighteen 256-term algebraic values, refused by the work meter.
    const REPEATED_CASCADE: &str =
        "[ 2 SQRT 3 SQRT ADD 5 SQRT 7 SQRT ADD MUL 11 SQRT 13 SQRT ADD MUL \
    17 SQRT 19 SQRT ADD MUL 23 SQRT 29 SQRT ADD MUL 31 SQRT 37 SQRT ADD MUL 41 SQRT 43 SQRT ADD MUL \
    47 SQRT 53 SQRT ADD MUL ] 'C' DEF C C C C C C C C C C C C C C C C C C C";

    #[tokio::test]
    async fn eliding_an_algebraic_value_drops_the_part_that_is_large() {
        // For an algebraic value the number lives in `semantics.exactTerms`,
        // while `value` is only the marked approximation — so dropping `value`
        // and keeping `semantics` dropped the cheap half and kept the expensive
        // one. Eighteen 256-term values came to 388 KB with seventeen of them
        // reported as elided.
        let report = agent_json(REPEATED_CASCADE, ComputeOptions::agent(None)).await;
        assert_eq!(report["status"], "error");
        assert!(
            compact_len(&report) < 128 * 1024,
            "an elided algebraic stack must actually shrink; got {} bytes",
            compact_len(&report)
        );
        let elided = report["stack"]
            .as_array()
            .expect("an array")
            .iter()
            .find(|node| node.get("elided").is_some())
            .expect("something was elided");
        assert!(
            elided["semantics"].get("exactTerms").is_none(),
            "the exact form is the value, and an elided slot carries no value"
        );
        assert_eq!(
            elided["elided"]["algebraicTerms"], 256,
            "how much was there is said in the record instead, so nothing goes \
             missing silently"
        );
        assert_eq!(
            elided["type"], "number",
            "what kind of value it was still survives"
        );
    }
}
