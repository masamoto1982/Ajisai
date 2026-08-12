//! What an error report keeps when the failing stack is too large to send.
//!
//! An error report carries two different things, and they are not equally
//! important. The **diagnosis** is the answer: why the program stopped, which
//! ceiling it met, what to do next. The **stack** is residual state: whatever
//! the program happened to be holding when it stopped. Serializing the residue
//! in full and letting the whole envelope exceed a host's response ceiling
//! trades the answer for the residue, which is backwards.
//!
//! It was not hypothetical. `[ 1 21000 ] RANGE 1 { * } FOLD` is refused by the
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
//!    `displayHint` and `semantics`, and gains an `elided` record naming what
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
    let slots: Vec<(ProtocolNode, String)> = interp
        .get_stack()
        .iter_slots()
        .map(|(value, role)| {
            (
                value_to_protocol(value, Some(role)),
                crate::types::display::format_with_hint(value, role),
            )
        })
        .collect();

    let rendered: Vec<Json> = slots
        .iter()
        .map(|(node, _)| protocol_node_json(node))
        .collect();
    let costs: Vec<usize> = slots
        .iter()
        .zip(rendered.iter())
        .map(|((_, display), node_json)| json_wire_bytes(node_json) + display.len())
        .collect();

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

    if keep.iter().all(|kept| *kept) {
        return ElidedStack {
            stack: Json::Array(rendered),
            stack_display: slots.into_iter().map(|(_, display)| display).collect(),
            elided: None,
        };
    }

    let mut stack = Vec::with_capacity(slots.len());
    let mut stack_display = Vec::with_capacity(slots.len());
    let mut elided_slots = Vec::new();
    for (index, ((node, display), node_json)) in slots.iter().zip(rendered).enumerate() {
        if keep[index] {
            stack.push(node_json);
            stack_display.push(display.clone());
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
        elided: Some(json!({
            "reason": "errorStackBudget",
            "budgetBytes": MAX_ERROR_STACK_BYTES,
            "slots": elided_slots,
        })),
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
    obj.insert(
        "displayHint".into(),
        json!(crate::types::value_protocol::interpretation_protocol_str(
            node.display_hint
        )),
    );
    if let Some(source) = &node.semantics {
        obj.insert(
            "semantics".into(),
            semantics_json(source, node.display_hint),
        );
    }
    obj.insert("type".into(), json!(node.type_str));
    obj.insert("value".into(), Json::Null);
    let mut record = Map::new();
    record.insert("reason".into(), json!("errorStackBudget"));
    record.insert("approxBytes".into(), json!(approx_bytes));
    if let Some(elements) = elements {
        record.insert("elements".into(), json!(elements));
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

/// Serialized length of `value` in the compact form the wire uses.
///
/// Computed by walking the tree rather than by serializing it, so measuring a
/// slot the report is about to discard does not first build the megabytes of
/// text that discarding it exists to avoid.
fn json_wire_bytes(value: &Json) -> usize {
    match value {
        Json::Null => 4,
        Json::Bool(true) => 4,
        Json::Bool(false) => 5,
        Json::Number(number) => number.to_string().len(),
        Json::String(text) => text.len() + 2,
        Json::Array(items) => {
            // brackets, plus one comma between each pair
            2 + items.len().saturating_sub(1) + items.iter().map(json_wire_bytes).sum::<usize>()
        }
        Json::Object(entries) => {
            // braces, plus one comma between each pair; each entry is a quoted
            // key, a colon, and its value
            2 + entries.len().saturating_sub(1)
                + entries
                    .iter()
                    .map(|(key, value)| key.len() + 3 + json_wire_bytes(value))
                    .sum::<usize>()
        }
    }
}
