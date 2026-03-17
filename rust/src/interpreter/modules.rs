use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{is_string_value, value_as_string};
use crate::interpreter::{audio, json, ConsumptionMode, Interpreter};
use crate::types::{Value, WordDefinition};
use std::collections::HashSet;
use std::sync::Arc;

type ModuleExecutor = fn(&mut Interpreter) -> Result<()>;

struct ModuleWord {
    name: &'static str,
    description: &'static str,
    executor: ModuleExecutor,
    preserves_modes: bool,
}

struct SampleWord {
    name: &'static str,
    definition: &'static str,
    description: &'static str,
}

struct ModuleSpec {
    name: &'static str,
    words: &'static [ModuleWord],
    sample_words: &'static [SampleWord],
}

const MUSIC_WORDS: &[ModuleWord] = &[
    ModuleWord {
        name: "MUSIC::SEQ",
        description: "Set sequential playback mode",
        executor: audio::op_seq,
        preserves_modes: true,
    },
    ModuleWord {
        name: "MUSIC::SIM",
        description: "Set simultaneous playback mode",
        executor: audio::op_sim,
        preserves_modes: true,
    },
    ModuleWord {
        name: "MUSIC::SLOT",
        description: "Set slot duration in seconds",
        executor: audio::op_slot,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::GAIN",
        description: "Set volume level (0.0-1.0)",
        executor: audio::op_gain,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::GAIN-RESET",
        description: "Reset volume to default (1.0)",
        executor: audio::op_gain_reset,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::PAN",
        description: "Set stereo position (-1.0 left to 1.0 right)",
        executor: audio::op_pan,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::PAN-RESET",
        description: "Reset pan to center (0.0)",
        executor: audio::op_pan_reset,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::FX-RESET",
        description: "Reset all audio effects to defaults",
        executor: audio::op_fx_reset,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::PLAY",
        description: "Play audio",
        executor: audio::op_play,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::CHORD",
        description: "Mark vector as chord (simultaneous)",
        executor: audio::op_chord,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::ADSR",
        description: "Set ADSR envelope",
        executor: audio::op_adsr,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::SINE",
        description: "Set sine waveform",
        executor: audio::op_sine,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::SQUARE",
        description: "Set square waveform",
        executor: audio::op_square,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::SAW",
        description: "Set sawtooth waveform",
        executor: audio::op_saw,
        preserves_modes: false,
    },
    ModuleWord {
        name: "MUSIC::TRI",
        description: "Set triangle waveform",
        executor: audio::op_tri,
        preserves_modes: false,
    },
];

const JSON_WORDS: &[ModuleWord] = &[
    ModuleWord {
        name: "JSON::PARSE",
        description: "Parse JSON string to Ajisai value",
        executor: json::op_parse,
        preserves_modes: false,
    },
    ModuleWord {
        name: "JSON::STRINGIFY",
        description: "Convert Ajisai value to JSON string",
        executor: json::op_stringify,
        preserves_modes: false,
    },
    ModuleWord {
        name: "JSON::GET",
        description: "Get value by key from JSON object",
        executor: json::op_json_get,
        preserves_modes: false,
    },
    ModuleWord {
        name: "JSON::KEYS",
        description: "Get all keys from JSON object",
        executor: json::op_json_keys,
        preserves_modes: false,
    },
    ModuleWord {
        name: "JSON::SET",
        description: "Set key-value in JSON object",
        executor: json::op_json_set,
        preserves_modes: false,
    },
    ModuleWord {
        name: "JSON::EXPORT",
        description: "Export stack top as JSON file download",
        executor: json::op_json_export,
        preserves_modes: false,
    },
];

const IO_WORDS: &[ModuleWord] = &[
    ModuleWord {
        name: "IO::INPUT",
        description: "Read text from input buffer",
        executor: json::op_input,
        preserves_modes: false,
    },
    ModuleWord {
        name: "IO::OUTPUT",
        description: "Write value to output buffer",
        executor: json::op_output,
        preserves_modes: false,
    },
];

const MUSIC_SAMPLES: &[SampleWord] = &[
    SampleWord { name: "C4", definition: "264", description: "純正律 C4 / ド (264Hz)" },
    SampleWord { name: "D4", definition: "C4 9 * 8 /", description: "純正律 D4 / レ (297Hz)" },
    SampleWord { name: "E4", definition: "C4 5 * 4 /", description: "純正律 E4 / ミ (330Hz)" },
    SampleWord { name: "F4", definition: "C4 4 * 3 /", description: "純正律 F4 / ファ (352Hz)" },
    SampleWord { name: "G4", definition: "C4 3 * 2 /", description: "純正律 G4 / ソ (396Hz)" },
    SampleWord { name: "A4", definition: "C4 5 * 3 /", description: "純正律 A4 / ラ (440Hz)" },
    SampleWord { name: "B4", definition: "C4 15 * 8 /", description: "純正律 B4 / シ (495Hz)" },
    SampleWord { name: "C5", definition: "C4 2 *", description: "純正律 C5 / 高いド (528Hz)" },
];

const MODULE_SPECS: &[ModuleSpec] = &[
    ModuleSpec {
        name: "MUSIC",
        words: MUSIC_WORDS,
        sample_words: MUSIC_SAMPLES,
    },
    ModuleSpec {
        name: "JSON",
        words: JSON_WORDS,
        sample_words: &[],
    },
    ModuleSpec {
        name: "IO",
        words: IO_WORDS,
        sample_words: &[],
    },
];

fn register_words(interp: &mut Interpreter, words: &[ModuleWord]) {
    for word in words {
        interp.builtin_dictionary.insert(
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
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let module_name = parse_module_name(&value)
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
    register_sample_words(interp, &module_name, module.sample_words)?;
    interp.imported_modules.insert(module_name);
    Ok(())
}

fn register_sample_words(
    interp: &mut Interpreter,
    module_name: &str,
    sample_words: &[SampleWord],
) -> Result<()> {
    if sample_words.is_empty() {
        return Ok(());
    }

    let module_dict = interp.module_samples
        .entry(module_name.to_string())
        .or_default();

    for sample in sample_words {
        let tokens = crate::tokenizer::tokenize(sample.definition)
            .map_err(|e| AjisaiError::from(format!(
                "Failed to tokenize sample word '{}': {}", sample.name, e
            )))?;
        let lines = parse_sample_definition_body(&tokens)?;
        let def = WordDefinition {
            lines: lines.into(),
            is_builtin: false,
            description: Some(sample.description.to_string()),
            dependencies: HashSet::new(),
            original_source: None,
        };
        module_dict.insert(sample.name.to_uppercase(), Arc::new(def));
    }

    // Rebuild dependencies for module sample words
    rebuild_module_sample_dependencies(interp, module_name);

    Ok(())
}

fn parse_sample_definition_body(
    tokens: &[crate::types::Token],
) -> Result<Vec<crate::types::ExecutionLine>> {
    let mut lines = Vec::new();
    let mut current_tokens = Vec::new();

    for token in tokens {
        match token {
            crate::types::Token::LineBreak => {
                if !current_tokens.is_empty() {
                    lines.push(crate::types::ExecutionLine {
                        body_tokens: current_tokens.clone().into(),
                    });
                    current_tokens.clear();
                }
            }
            _ => {
                current_tokens.push(token.clone());
            }
        }
    }

    if !current_tokens.is_empty() {
        lines.push(crate::types::ExecutionLine {
            body_tokens: current_tokens.into(),
        });
    }

    if lines.is_empty() {
        return Err(AjisaiError::from("Sample word definition cannot be empty"));
    }

    Ok(lines)
}

fn rebuild_module_sample_dependencies(interp: &mut Interpreter, module_name: &str) {
    if let Some(module_dict) = interp.module_samples.get(module_name) {
        let word_names: Vec<String> = module_dict.keys().cloned().collect();
        let word_name_set: HashSet<String> = word_names.iter().cloned().collect();

        let mut updates: Vec<(String, HashSet<String>)> = Vec::new();

        for (word_name, def) in module_dict {
            let mut dependencies = HashSet::new();
            for line in def.lines.iter() {
                for token in line.body_tokens.iter() {
                    if let crate::types::Token::Symbol(s) = token {
                        let upper_s = s.to_uppercase();
                        if word_name_set.contains(&upper_s) {
                            dependencies.insert(upper_s.clone());
                        }
                    }
                }
            }
            updates.push((word_name.clone(), dependencies));
        }

        // Apply dependency updates
        if let Some(module_dict) = interp.module_samples.get_mut(module_name) {
            for (word_name, dependencies) in &updates {
                if let Some(def) = module_dict.get_mut(word_name) {
                    Arc::make_mut(def).dependencies = dependencies.clone();
                }
            }
        }

        // Update dependents map
        for (word_name, dependencies) in updates {
            for dep in dependencies {
                interp.dependents
                    .entry(dep)
                    .or_default()
                    .insert(word_name.clone());
            }
        }
    }
}

/// Re-import a module by name without requiring a stack value.
/// Used for restoring module state from JS side.
pub fn restore_module(interp: &mut Interpreter, module_name: &str) -> bool {
    let upper = module_name.to_uppercase();
    if interp.imported_modules.contains(&upper) {
        return true;
    }
    if let Some(module) = MODULE_SPECS.iter().find(|m| m.name == upper) {
        register_words(interp, module.words);
        if register_sample_words(interp, &upper, module.sample_words).is_err() {
            return false;
        }
        interp.imported_modules.insert(upper);
        true
    } else {
        false
    }
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

fn parse_module_name(value: &Value) -> Option<String> {
    if is_string_value(value) {
        return value_as_string(value);
    }

    let children = value.as_vector()?;
    if children.len() != 1 {
        return None;
    }
    if !is_string_value(&children[0]) {
        return None;
    }
    value_as_string(&children[0])
}
