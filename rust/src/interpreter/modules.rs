use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{is_string_value, value_as_string};
use crate::interpreter::{
    audio, json, ConsumptionMode, ImportedModule, Interpreter, ModuleDictionary,
};
use crate::types::{Value, ValueData, WordDefinition};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

type ModuleExecutor = fn(&mut Interpreter) -> Result<()>;

#[derive(Clone)]
struct ModuleWord {
    short_name: &'static str,
    description: &'static str,
    executor: ModuleExecutor,
    preserves_modes: bool,
}

#[derive(Clone)]
struct SampleWord {
    name: &'static str,
    definition: &'static str,
    description: &'static str,
}

#[derive(Clone)]
struct ModuleSpec {
    name: &'static str,
    words: &'static [ModuleWord],
    sample_words: &'static [SampleWord],
}

const MUSIC_WORDS: &[ModuleWord] = &[
    ModuleWord {
        short_name: "SEQ",
        description: "Set sequential playback mode",
        executor: audio::op_seq,
        preserves_modes: true,
    },
    ModuleWord {
        short_name: "SIM",
        description: "Set simultaneous playback mode",
        executor: audio::op_sim,
        preserves_modes: true,
    },
    ModuleWord {
        short_name: "SLOT",
        description: "Set slot duration in seconds",
        executor: audio::op_slot,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "GAIN",
        description: "Set volume level (0.0-1.0)",
        executor: audio::op_gain,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "GAIN-RESET",
        description: "Reset volume to default (1.0)",
        executor: audio::op_gain_reset,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "PAN",
        description: "Set stereo position (-1.0 left to 1.0 right)",
        executor: audio::op_pan,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "PAN-RESET",
        description: "Reset pan to center (0.0)",
        executor: audio::op_pan_reset,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "FX-RESET",
        description: "Reset all audio effects to defaults",
        executor: audio::op_fx_reset,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "PLAY",
        description: "Play audio",
        executor: audio::op_play,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "CHORD",
        description: "Mark vector as chord (simultaneous)",
        executor: audio::op_chord,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "ADSR",
        description: "Set ADSR envelope",
        executor: audio::op_adsr,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "SINE",
        description: "Set sine waveform",
        executor: audio::op_sine,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "SQUARE",
        description: "Set square waveform",
        executor: audio::op_square,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "SAW",
        description: "Set sawtooth waveform",
        executor: audio::op_saw,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "TRI",
        description: "Set triangle waveform",
        executor: audio::op_tri,
        preserves_modes: false,
    },
];

const JSON_WORDS: &[ModuleWord] = &[
    ModuleWord {
        short_name: "PARSE",
        description: "Parse JSON string to Ajisai value",
        executor: json::op_parse,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "STRINGIFY",
        description: "Convert Ajisai value to JSON string",
        executor: json::op_stringify,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "GET",
        description: "Get value by key from JSON object",
        executor: json::op_json_get,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "KEYS",
        description: "Get all keys from JSON object",
        executor: json::op_json_keys,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "SET",
        description: "Set key-value in JSON object",
        executor: json::op_json_set,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "EXPORT",
        description: "Export stack top as JSON file download",
        executor: json::op_json_export,
        preserves_modes: false,
    },
];

const IO_WORDS: &[ModuleWord] = &[
    ModuleWord {
        short_name: "INPUT",
        description: "Read text from input buffer",
        executor: json::op_input,
        preserves_modes: false,
    },
    ModuleWord {
        short_name: "OUTPUT",
        description: "Write value to output buffer",
        executor: json::op_output,
        preserves_modes: false,
    },
];

const MUSIC_SAMPLES: &[SampleWord] = &[
    SampleWord {
        name: "C4",
        definition: "264",
        description: "純正律 C4 / ド (264Hz)",
    },
    SampleWord {
        name: "D4",
        definition: "C4 9 * 8 /",
        description: "純正律 D4 / レ (297Hz)",
    },
    SampleWord {
        name: "E4",
        definition: "C4 5 * 4 /",
        description: "純正律 E4 / ミ (330Hz)",
    },
    SampleWord {
        name: "F4",
        definition: "C4 4 * 3 /",
        description: "純正律 F4 / ファ (352Hz)",
    },
    SampleWord {
        name: "G4",
        definition: "C4 3 * 2 /",
        description: "純正律 G4 / ソ (396Hz)",
    },
    SampleWord {
        name: "A4",
        definition: "C4 5 * 3 /",
        description: "純正律 A4 / ラ (440Hz)",
    },
    SampleWord {
        name: "B4",
        definition: "C4 15 * 8 /",
        description: "純正律 B4 / シ (495Hz)",
    },
    SampleWord {
        name: "C5",
        definition: "C4 2 *",
        description: "純正律 C5 / 高いド (528Hz)",
    },
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

fn ensure_module_dictionary(interp: &mut Interpreter, module_name: &str) -> Result<()> {
    if interp.module_vocabulary.contains_key(module_name) {
        return Ok(());
    }
    let module = MODULE_SPECS
        .iter()
        .find(|module| module.name == module_name)
        .ok_or_else(|| AjisaiError::UnknownModule(module_name.to_string()))?;

    let mut words = HashMap::new();
    for word in module.words {
        let qualified = format!("{}@{}", module.name, word.short_name);
        words.insert(
            qualified.clone(),
            Arc::new(WordDefinition {
                lines: Arc::from([]),
                is_builtin: true,
                description: Some(word.description.to_string()),
                dependencies: HashSet::new(),
                original_source: None,
                namespace: Some(module.name.to_string()),
                registration_order: 0,
            }),
        );
    }

    let sample_words = build_sample_words(module.name, module.sample_words)?;
    interp.module_vocabulary.insert(
        module_name.to_string(),
        ModuleDictionary {
            words,
            sample_words,
        },
    );
    Ok(())
}

fn build_sample_words(
    module_name: &str,
    sample_words: &[SampleWord],
) -> Result<HashMap<String, Arc<WordDefinition>>> {
    let mut result = HashMap::new();
    for sample in sample_words {
        let tokens = crate::tokenizer::tokenize(sample.definition).map_err(|e| {
            AjisaiError::from(format!(
                "Failed to tokenize sample word '{}': {}",
                sample.name, e
            ))
        })?;
        let lines = parse_sample_definition_body(&tokens)?;
        result.insert(
            sample.name.to_uppercase(),
            Arc::new(WordDefinition {
                lines: lines.into(),
                is_builtin: false,
                description: Some(sample.description.to_string()),
                dependencies: HashSet::new(),
                original_source: None,
                namespace: Some(module_name.to_string()),
                registration_order: 0,
            }),
        );
    }
    Ok(result)
}

fn parse_only_items(value: &Value) -> Result<Vec<String>> {
    if is_string_value(value) {
        let s = value_as_string(value)
            .ok_or_else(|| AjisaiError::from("IMPORT-ONLY expects string selectors"))?;
        return Ok(vec![s]);
    }

    let Some(items) = value.as_vector() else {
        return Err(AjisaiError::from(
            "IMPORT-ONLY expects a vector of word names",
        ));
    };

    let mut result = Vec::new();
    for item in items {
        if !is_string_value(item) {
            return Err(AjisaiError::from("IMPORT-ONLY selectors must be strings"));
        }
        let Some(name) = value_as_string(item) else {
            continue;
        };
        result.push(name);
    }
    Ok(result)
}

fn import_all_public(interp: &mut Interpreter, module_name: &str) -> Result<()> {
    ensure_module_dictionary(interp, module_name)?;
    let module_dict = interp
        .module_vocabulary
        .get(module_name)
        .ok_or_else(|| AjisaiError::UnknownModule(module_name.to_string()))?;

    let mut imported_words = HashSet::new();
    let mut imported_samples = HashSet::new();

    for qualified in module_dict.words.keys() {
        if let Some((_, short)) = qualified.split_once('@') {
            imported_words.insert(short.to_string());
        }
    }
    for short in module_dict.sample_words.keys() {
        imported_samples.insert(short.clone());
    }

    let entry = interp
        .import_table
        .modules
        .entry(module_name.to_string())
        .or_insert_with(|| ImportedModule {
            import_all_public: false,
            imported_words: HashSet::new(),
            imported_samples: HashSet::new(),
        });

    entry.import_all_public = true;
    entry.imported_words = imported_words;
    entry.imported_samples = imported_samples;
    emit_import_conflict_warnings(interp, module_name);
    Ok(())
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

    let module_name = extract_module_name_from_value(&value)
        .ok_or_else(|| AjisaiError::UnknownModule(value.to_string()))?
        .to_uppercase();

    import_all_public(interp, &module_name)?;
    interp.rebuild_dependencies()?;
    Ok(())
}

pub fn op_import_only(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let selectors = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let module_value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let module_name = extract_module_name_from_value(&module_value)
        .ok_or_else(|| AjisaiError::UnknownModule(module_value.to_string()))?
        .to_uppercase();

    ensure_module_dictionary(interp, &module_name)?;
    let selected = parse_only_items(&selectors)?;

    let module_dict = interp
        .module_vocabulary
        .get(&module_name)
        .ok_or_else(|| AjisaiError::UnknownModule(module_name.clone()))?;
    let mut validated: Vec<(String, bool)> = Vec::new();
    for item in selected {
        let short = item.to_uppercase();
        let qualified = format!("{}@{}", module_name, short);
        if module_dict.words.contains_key(&qualified) {
            validated.push((short, true));
        } else if module_dict.sample_words.contains_key(&short) {
            validated.push((short, false));
        } else {
            return Err(AjisaiError::from(format!(
                "Unknown module word '{}' in {}",
                short, module_name
            )));
        }
    }

    let entry = interp
        .import_table
        .modules
        .entry(module_name.clone())
        .or_insert_with(|| ImportedModule {
            import_all_public: false,
            imported_words: HashSet::new(),
            imported_samples: HashSet::new(),
        });

    for (short, is_word) in validated {
        if is_word {
            entry.imported_words.insert(short.clone());
        } else {
            entry.imported_samples.insert(short.clone());
        }
    }

    emit_import_conflict_warnings(interp, &module_name);
    interp.rebuild_dependencies()?;
    Ok(())
}



pub fn restore_module(interp: &mut Interpreter, module_name: &str) -> bool {
    let upper = module_name.to_uppercase();
    import_all_public(interp, &upper).is_ok()
}

pub fn execute_module_word(interp: &mut Interpreter, name: &str) -> Option<Result<()>> {
    let upper = name.to_uppercase();
    let (module_name, word_name) = upper.split_once('@')?;
    let module = MODULE_SPECS.iter().find(|m| m.name == module_name)?;
    let word = module.words.iter().find(|w| w.short_name == word_name)?;
    Some((word.executor)(interp))
}

pub fn is_mode_preserving_word(name: &str) -> bool {
    let upper = name.to_uppercase();
    let Some((module_name, word_name)) = upper.split_once('@') else {
        return false;
    };

    MODULE_SPECS
        .iter()
        .find(|m| m.name == module_name)
        .and_then(|m| m.words.iter().find(|w| w.short_name == word_name))
        .map(|w| w.preserves_modes)
        .unwrap_or(false)
}

fn extract_module_name_from_value(value: &Value) -> Option<String> {
    if is_string_value(value) {
        return value_as_string(value);
    }

    match &value.data {
        ValueData::Vector(children)
        | ValueData::Record {
            pairs: children, ..
        } => {
            if children.len() != 1 {
                return None;
            }
            if !is_string_value(&children[0]) {
                return None;
            }
            value_as_string(&children[0])
        }
        _ => None,
    }
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

fn emit_import_conflict_warnings(interp: &mut Interpreter, module_name: &str) {
    let Some(module_dict) = interp.module_vocabulary.get(module_name) else {
        return;
    };

    for short_name in module_dict.sample_words.keys() {
        let mut collisions: Vec<String> = Vec::new();
        for (dict_name, dict) in &interp.user_dictionaries {
            if dict.words.contains_key(short_name) {
                collisions.push(format!("{}@{}", dict_name, short_name));
            }
        }
        if collisions.is_empty() {
            continue;
        }

        let mut all_paths = vec![format!("{}@{}", module_name, short_name)];
        all_paths.extend(collisions);
        interp.output_buffer.push_str(&format!(
            "Warning: '{}' now exists in both {}. Use a qualified path when calling this word.\n",
            short_name,
            all_paths.join(" and ")
        ));
    }
}
