use crate::error::{AjisaiError, Result};
use crate::interpreter::{audio, json, Interpreter};
use crate::types::{DisplayHint, Value, ValueData, WordDefinition};
use std::collections::HashSet;
use std::sync::Arc;

type ModuleExecutor = fn(&mut Interpreter) -> Result<()>;

struct ModuleWord {
    name: &'static str,
    description: &'static str,
    executor: ModuleExecutor,
    preserves_modes: bool,
}

struct ModuleSpec {
    name: &'static str,
    words: &'static [ModuleWord],
}

const MUSIC_WORDS: &[ModuleWord] = &[
    ModuleWord { name: "MUSIC::SEQ", description: "Set sequential playback mode", executor: audio::op_seq, preserves_modes: true },
    ModuleWord { name: "MUSIC::SIM", description: "Set simultaneous playback mode", executor: audio::op_sim, preserves_modes: true },
    ModuleWord { name: "MUSIC::SLOT", description: "Set slot duration in seconds", executor: audio::op_slot, preserves_modes: false },
    ModuleWord { name: "MUSIC::GAIN", description: "Set volume level (0.0-1.0)", executor: audio::op_gain, preserves_modes: false },
    ModuleWord { name: "MUSIC::GAIN-RESET", description: "Reset volume to default (1.0)", executor: audio::op_gain_reset, preserves_modes: false },
    ModuleWord { name: "MUSIC::PAN", description: "Set stereo position (-1.0 left to 1.0 right)", executor: audio::op_pan, preserves_modes: false },
    ModuleWord { name: "MUSIC::PAN-RESET", description: "Reset pan to center (0.0)", executor: audio::op_pan_reset, preserves_modes: false },
    ModuleWord { name: "MUSIC::FX-RESET", description: "Reset all audio effects to defaults", executor: audio::op_fx_reset, preserves_modes: false },
    ModuleWord { name: "MUSIC::PLAY", description: "Play audio", executor: audio::op_play, preserves_modes: false },
    ModuleWord { name: "MUSIC::CHORD", description: "Mark vector as chord (simultaneous)", executor: audio::op_chord, preserves_modes: false },
    ModuleWord { name: "MUSIC::ADSR", description: "Set ADSR envelope", executor: audio::op_adsr, preserves_modes: false },
    ModuleWord { name: "MUSIC::SINE", description: "Set sine waveform", executor: audio::op_sine, preserves_modes: false },
    ModuleWord { name: "MUSIC::SQUARE", description: "Set square waveform", executor: audio::op_square, preserves_modes: false },
    ModuleWord { name: "MUSIC::SAW", description: "Set sawtooth waveform", executor: audio::op_saw, preserves_modes: false },
    ModuleWord { name: "MUSIC::TRI", description: "Set triangle waveform", executor: audio::op_tri, preserves_modes: false },
];

const JSON_WORDS: &[ModuleWord] = &[
    ModuleWord { name: "JSON::PARSE", description: "Parse JSON string to Ajisai value", executor: json::op_parse, preserves_modes: false },
    ModuleWord { name: "JSON::STRINGIFY", description: "Convert Ajisai value to JSON string", executor: json::op_stringify, preserves_modes: false },
    ModuleWord { name: "JSON::GET", description: "Get value by key from JSON object", executor: json::op_json_get, preserves_modes: false },
    ModuleWord { name: "JSON::KEYS", description: "Get all keys from JSON object", executor: json::op_json_keys, preserves_modes: false },
    ModuleWord { name: "JSON::SET", description: "Set key-value in JSON object", executor: json::op_json_set, preserves_modes: false },
    ModuleWord { name: "JSON::EXPORT", description: "Export stack top as JSON file download", executor: json::op_json_export, preserves_modes: false },
];

const IO_WORDS: &[ModuleWord] = &[
    ModuleWord { name: "IO::INPUT", description: "Read text from input buffer", executor: json::op_input, preserves_modes: false },
    ModuleWord { name: "IO::OUTPUT", description: "Write value to output buffer", executor: json::op_output, preserves_modes: false },
];

const MODULE_SPECS: &[ModuleSpec] = &[
    ModuleSpec { name: "MUSIC", words: MUSIC_WORDS },
    ModuleSpec { name: "JSON", words: JSON_WORDS },
    ModuleSpec { name: "IO", words: IO_WORDS },
];

fn register_words(interp: &mut Interpreter, words: &[ModuleWord]) {
    for word in words {
        interp.dictionary.insert(
            word.name.to_string(),
            Arc::new(WordDefinition {
                lines: Arc::from([]),
                is_builtin: true,
                description: Some(word.description.to_string()),
                dependencies: HashSet::new(),
                original_source: None,
            }),
        );
    }
}

pub fn op_import(interp: &mut Interpreter) -> Result<()> {
    let value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let module_name = value_to_module_name(&value)
        .ok_or_else(|| AjisaiError::UnknownModule(value.to_string()))?
        .to_uppercase();

    if interp.imported_modules.contains(&module_name) {
        return Ok(());
    }

    let module = MODULE_SPECS
        .iter()
        .find(|module| module.name == module_name)
        .ok_or_else(|| AjisaiError::UnknownModule(module_name.clone()))?;

    register_words(interp, module.words);
    interp.imported_modules.insert(module_name);
    Ok(())
}

pub fn execute_module_word(interp: &mut Interpreter, name: &str) -> Option<Result<()>> {
    for module in MODULE_SPECS {
        for word in module.words {
            if word.name == name {
                return Some((word.executor)(interp));
            }
        }
    }
    None
}

/// Check if a module word should preserve operation modes (target/consumption).
/// Uses metadata from ModuleWord rather than hardcoding word names.
pub fn preserves_modes(name: &str) -> bool {
    MODULE_SPECS
        .iter()
        .flat_map(|m| m.words.iter())
        .any(|w| w.name == name && w.preserves_modes)
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
