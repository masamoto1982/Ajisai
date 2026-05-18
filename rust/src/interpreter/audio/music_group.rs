//! Semantic music group values for the MUSIC module.
//!
//! A music group is a `ValueData::Record` carrying explicit provenance so that
//! AI tooling can explain *why* a nested structure plays a certain way, rather
//! than guessing musical meaning from a raw nested vector.

use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::types::{Interpretation, Value, ValueData};
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) const GROUP_KIND: &str = "music.group";

const FIELD_KEYS: [&str; 5] = ["kind", "mode", "role", "provenance", "children"];

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
    pub provenance: String,
    pub children: Vec<Value>,
}

fn plain_vector(children: Vec<Value>) -> Value {
    Value {
        data: ValueData::Vector(Rc::new(children)),
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

fn record_pair(key: &str, value: Value) -> Value {
    plain_vector(vec![Value::from_string(key), value])
}

/// Build a `music.group` record value with explicit provenance.
pub(crate) fn make_group(
    mode: GroupMode,
    role: &str,
    provenance: &str,
    children: Vec<Value>,
) -> Value {
    let pairs = vec![
        record_pair("kind", Value::from_string(GROUP_KIND)),
        record_pair("mode", Value::from_string(mode.as_str())),
        record_pair("role", Value::from_string(role)),
        record_pair("provenance", Value::from_string(provenance)),
        record_pair("children", plain_vector(children)),
    ];
    let mut index = HashMap::new();
    for (i, key) in FIELD_KEYS.iter().enumerate() {
        index.insert((*key).to_string(), i);
    }
    Value {
        data: ValueData::Record {
            pairs: Rc::new(pairs),
            index,
        },
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

fn record_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    if let ValueData::Record { pairs, index } = &value.data {
        let pos = *index.get(key)?;
        if let Some(pair) = pairs.get(pos) {
            if let ValueData::Vector(kv) = &pair.data {
                if kv.len() == 2 {
                    return Some(&kv[1]);
                }
            }
        }
    }
    None
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
    let provenance = record_field(value, "provenance")
        .and_then(value_as_string)
        .unwrap_or_else(|| "unknown".to_string());
    let children = match record_field(value, "children").map(|c| &c.data) {
        Some(ValueData::Vector(v)) => v.as_ref().clone(),
        _ => Vec::new(),
    };
    Some(MusicGroup {
        mode,
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
        let (label, boundary, simultaneous) = match group.mode {
            GroupMode::Sequential => ("Sequential group", "AudioStructure::Seq", false),
            GroupMode::Simultaneous => ("Simultaneous group", "AudioStructure::Sim", true),
            GroupMode::Chord => ("Chord group", "AudioStructure::Sim", true),
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
