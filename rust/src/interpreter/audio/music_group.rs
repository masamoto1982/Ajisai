//! Semantic music group values for the MUSIC module.
//!
//! A music group is a `ValueData::Record` carrying explicit provenance so that
//! AI tooling can explain *why* a nested structure plays a certain way, rather
//! than guessing musical meaning from a raw nested vector.

use super::music_values::{make_record, record_field};
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::types::{Interpretation, Value, ValueData};
use std::sync::Arc;

pub(crate) const GROUP_KIND: &str = "music.group";

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GroupMode {
    Sequential,
    Simultaneous,
    Chord,
}

impl GroupMode {
    fn as_str(self) -> &'static str {
        match self {
            GroupMode::Sequential => "seq",
            GroupMode::Simultaneous => "sim",
            GroupMode::Chord => "chord",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "seq" => Some(GroupMode::Sequential),
            "sim" => Some(GroupMode::Simultaneous),
            "chord" => Some(GroupMode::Chord),
            _ => None,
        }
    }
}

pub(crate) struct MusicGroup {
    pub mode: GroupMode,
    pub role: String,
    pub provenance: String,
    pub children: Vec<Value>,
}

/// Build a `music.group` record value with explicit provenance.
pub(crate) fn make_group(
    mode: GroupMode,
    role: &str,
    provenance: &str,
    children: Vec<Value>,
) -> Value {
    let children_value = Value {
        data: ValueData::Vector(Arc::new(children)),
        hint: Interpretation::Unassigned,
        absence: None,
    };
    make_record(vec![
        ("kind", Value::from_string(GROUP_KIND)),
        ("mode", Value::from_string(mode.as_str())),
        ("role", Value::from_string(role)),
        ("provenance", Value::from_string(provenance)),
        ("children", children_value),
    ])
}

/// Report whether `value` is a `music.group` record.
pub(crate) fn is_music_group(value: &Value) -> bool {
    record_field(value, "kind")
        .and_then(value_as_string)
        .map(|s| s == GROUP_KIND)
        .unwrap_or(false)
}

/// Parse a `music.group` record into its semantic parts.
pub(crate) fn parse_group(value: &Value) -> Option<MusicGroup> {
    if !is_music_group(value) {
        return None;
    }
    let mode = GroupMode::from_str(&record_field(value, "mode").and_then(value_as_string)?)?;
    let role = record_field(value, "role")
        .and_then(value_as_string)
        .unwrap_or_else(|| "generic".to_string());
    let provenance = record_field(value, "provenance")
        .and_then(value_as_string)
        .unwrap_or_else(|| "unknown".to_string());
    let children = match record_field(value, "children").map(|c| &c.data) {
        Some(ValueData::Vector(v)) => v.as_ref().clone(),
        _ => Vec::new(),
    };
    Some(MusicGroup {
        mode,
        role,
        provenance,
        children,
    })
}

/// Extract the playable children from a group-constructor operand.
///
/// Accepts a plain (non-empty) vector or an existing music group. The runtime
/// deliberately does not infer further musical meaning from the operand shape.
pub(crate) fn operand_children(value: &Value, word: &str) -> Result<Vec<Value>> {
    if let Some(group) = parse_group(value) {
        return Ok(group.children);
    }
    let empty_err = || AjisaiError::from(format!("MUSIC@{} requires a non-empty vector", word));
    if value.is_nil() {
        return Err(empty_err());
    }
    match &value.data {
        ValueData::Vector(v) => {
            if v.is_empty() {
                Err(empty_err())
            } else {
                Ok(v.as_ref().clone())
            }
        }
        ValueData::Tensor { .. } => {
            let len = value.len();
            if len == 0 {
                return Err(empty_err());
            }
            Ok((0..len).filter_map(|i| value.child(i)).collect())
        }
        _ => Err(AjisaiError::from(format!(
            "MUSIC@{} requires a vector or music group",
            word
        ))),
    }
}

/// Produce a human/AI readable explanation of a value's musical interpretation.
pub(crate) fn explain_value(value: &Value) -> String {
    if let Some(group) = parse_group(value) {
        let boundary = match group.mode {
            GroupMode::Sequential => "AudioStructure::Seq",
            GroupMode::Simultaneous | GroupMode::Chord => "AudioStructure::Sim",
        };
        let simultaneous = !matches!(group.mode, GroupMode::Sequential);
        let label = match group.role.as_str() {
            "voice" => "Voice group",
            "track" => "Track group",
            "measure" => "Measure group",
            "phrase" => "Phrase group",
            "chord" => "Chord group",
            _ => match group.mode {
                GroupMode::Sequential => "Sequential group",
                GroupMode::Simultaneous => "Simultaneous group",
                GroupMode::Chord => "Chord group",
            },
        };
        let provenance = group
            .provenance
            .strip_prefix("explicit:")
            .map(|w| format!("explicit {}", w))
            .unwrap_or(group.provenance);
        let mut s = format!("{}: {} children", label, group.children.len());
        if simultaneous {
            s.push_str(", simultaneous playback");
        }
        s.push_str(".\n");
        s.push_str(&format!("Provenance: {}.\n", provenance));
        s.push_str(&format!(
            "Playback boundary: converted to {} at MUSIC@PLAY.",
            boundary
        ));
        return s;
    }

    if let Some(leaf) = super::music_values::describe_leaf(value) {
        return leaf;
    }

    if matches!(value.data, ValueData::Vector(_) | ValueData::Tensor { .. }) {
        return "Raw Vector: legacy MUSIC playback would interpret this as a \
                sequential group.\nUse MUSIC@SEQ-GROUP or MUSIC@SIM-GROUP for \
                explicit AI-readable semantics."
            .to_string();
    }
    if value.is_nil() {
        return "NIL: MUSIC@PLAY would interpret this as a rest.".to_string();
    }
    if value.is_scalar() {
        return "Scalar: legacy MUSIC tone (numerator = Hz, denominator = \
                duration). Wrap it with MUSIC@SEQ-GROUP for explicit semantics."
            .to_string();
    }
    "Untyped value: MUSIC@PLAY has no explicit musical interpretation for this value.".to_string()
}
