use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::{DisplayHint, Value, ValueData, WordDefinition};
use std::collections::HashSet;
use std::sync::Arc;

const MUSIC_WORDS: &[(&str, &str)] = &[
    ("MUSIC::SEQ", "Set sequential playback mode"),
    ("MUSIC::SIM", "Set simultaneous playback mode"),
    ("MUSIC::SLOT", "Set slot duration in seconds"),
    ("MUSIC::GAIN", "Set volume level (0.0-1.0)"),
    ("MUSIC::GAIN-RESET", "Reset volume to default (1.0)"),
    ("MUSIC::PAN", "Set stereo position (-1.0 left to 1.0 right)"),
    ("MUSIC::PAN-RESET", "Reset pan to center (0.0)"),
    ("MUSIC::FX-RESET", "Reset all audio effects to defaults"),
    ("MUSIC::PLAY", "Play audio"),
    ("MUSIC::CHORD", "Mark vector as chord (simultaneous)"),
    ("MUSIC::ADSR", "Set ADSR envelope"),
    ("MUSIC::SINE", "Set sine waveform"),
    ("MUSIC::SQUARE", "Set square waveform"),
    ("MUSIC::SAW", "Set sawtooth waveform"),
    ("MUSIC::TRI", "Set triangle waveform"),
];

const JSON_WORDS: &[(&str, &str)] = &[
    ("JSON::PARSE", "Parse JSON string to Ajisai value"),
    ("JSON::STRINGIFY", "Convert Ajisai value to JSON string"),
    ("JSON::GET", "Get value by key from JSON object"),
    ("JSON::KEYS", "Get all keys from JSON object"),
    ("JSON::SET", "Set key-value in JSON object"),
    ("JSON::EXPORT", "Export stack top as JSON file download"),
];

const IO_WORDS: &[(&str, &str)] = &[
    ("IO::INPUT", "Read text from input buffer"),
    ("IO::OUTPUT", "Write value to output buffer"),
];

fn register_words(interp: &mut Interpreter, words: &[(&str, &str)]) {
    for (name, description) in words {
        interp.dictionary.insert(
            (*name).to_string(),
            Arc::new(WordDefinition {
                lines: Arc::from([]),
                is_builtin: true,
                description: Some((*description).to_string()),
                dependencies: HashSet::new(),
                original_source: None,
            }),
        );
    }
}

pub fn op_import(interp: &mut Interpreter) -> Result<()> {
    let value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let module_name = value_to_module_name(&value)
        .ok_or_else(|| AjisaiError::UnknownModule(value.to_string()))?;

    let normalized = module_name.to_uppercase();
    if interp.imported_modules.contains(&normalized) {
        return Ok(());
    }

    match normalized.as_str() {
        "MUSIC" => register_words(interp, MUSIC_WORDS),
        "JSON" => register_words(interp, JSON_WORDS),
        "IO" => register_words(interp, IO_WORDS),
        _ => return Err(AjisaiError::UnknownModule(module_name)),
    }

    interp.imported_modules.insert(normalized);
    Ok(())
}

fn value_to_module_name(value: &Value) -> Option<String> {
    if value.display_hint == DisplayHint::String {
        return vector_to_string(value);
    }

    match &value.data {
        ValueData::Vector(children) if children.len() == 1 => vector_to_string(&children[0]),
        _ => None,
    }
}

fn vector_to_string(value: &Value) -> Option<String> {
    match &value.data {
        ValueData::Vector(children) => {
            let mut out = String::with_capacity(children.len());
            for child in children.iter() {
                let ValueData::Scalar(f) = &child.data else {
                    return None;
                };
                let code = f.to_i64()?;
                let ch = char::from_u32(code as u32)?;
                out.push(ch);
            }
            Some(out)
        }
        _ => None,
    }
}
