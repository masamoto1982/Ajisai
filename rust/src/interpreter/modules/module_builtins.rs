use crate::interpreter::{audio, datetime, hash, json, random, sort};
use crate::types::{Capabilities, Stability};

use super::module_word_types::{ModuleSpec, ModuleWord, SampleWord};

const MUSIC_WORDS: &[ModuleWord] = &[
    ModuleWord { short_name: "SEQ", description: "Set sequential playback mode", executor: audio::op_seq, preserves_modes: true, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "SIM", description: "Set simultaneous playback mode", executor: audio::op_sim, preserves_modes: true, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "SLOT", description: "Set slot duration in seconds", executor: audio::op_slot, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "GAIN", description: "Set volume level (0.0-1.0)", executor: audio::op_gain, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "GAIN-RESET", description: "Reset volume to default (1.0)", executor: audio::op_gain_reset, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "PAN", description: "Set stereo position (-1.0 left to 1.0 right)", executor: audio::op_pan, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "PAN-RESET", description: "Reset pan to center (0.0)", executor: audio::op_pan_reset, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "FX-RESET", description: "Reset all audio effects to defaults", executor: audio::op_fx_reset, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "PLAY", description: "Play audio", executor: audio::op_play, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "CHORD", description: "Mark vector as chord (simultaneous)", executor: audio::op_chord, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "ADSR", description: "Set ADSR envelope", executor: audio::op_adsr, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "SINE", description: "Set sine waveform", executor: audio::op_sine, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "SQUARE", description: "Set square waveform", executor: audio::op_square, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "SAW", description: "Set sawtooth waveform", executor: audio::op_saw, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "TRI", description: "Set triangle waveform", executor: audio::op_tri, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
];

const JSON_WORDS: &[ModuleWord] = &[
    ModuleWord { short_name: "PARSE", description: "Parse JSON string to Ajisai value", executor: json::op_parse, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::PURE },
    ModuleWord { short_name: "STRINGIFY", description: "Convert Ajisai value to JSON string", executor: json::op_stringify, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::PURE },
    ModuleWord { short_name: "GET", description: "Get value by key from JSON object", executor: json::op_json_get, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::PURE },
    ModuleWord { short_name: "KEYS", description: "Get all keys from JSON object", executor: json::op_json_keys, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::PURE },
    ModuleWord { short_name: "SET", description: "Set key-value in JSON object", executor: json::op_json_set, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::PURE },
    ModuleWord { short_name: "EXPORT", description: "Export stack top as JSON file download", executor: json::op_json_export, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
];

const IO_WORDS: &[ModuleWord] = &[
    ModuleWord { short_name: "INPUT", description: "Read text from input buffer", executor: json::op_input, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
    ModuleWord { short_name: "OUTPUT", description: "Write value to output buffer", executor: json::op_output, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::IO },
];

const TIME_WORDS: &[ModuleWord] = &[
    ModuleWord { short_name: "NOW", description: "Get current Unix timestamp", executor: datetime::op_now, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::TIME },
    ModuleWord { short_name: "DATETIME", description: "Convert timestamp to datetime vector", executor: datetime::op_datetime, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::TIME },
    ModuleWord { short_name: "TIMESTAMP", description: "Convert datetime vector to timestamp", executor: datetime::op_timestamp, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::TIME },
];

const CRYPTO_WORDS: &[ModuleWord] = &[
    ModuleWord { short_name: "CSPRNG", description: "Generate cryptographically secure random numbers", executor: random::op_csprng, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::RANDOM.union(Capabilities::CRYPTO) },
    ModuleWord { short_name: "HASH", description: "Compute hash value", executor: hash::op_hash, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::PURE.union(Capabilities::CRYPTO) },
];

const ALGO_WORDS: &[ModuleWord] = &[
    ModuleWord { short_name: "SORT", description: "Sort vector elements in ascending order", executor: sort::op_sort, preserves_modes: false, stability: Stability::Stable, capabilities: Capabilities::PURE },
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

pub(super) const MODULE_SPECS: &[ModuleSpec] = &[
    ModuleSpec { name: "MUSIC", words: MUSIC_WORDS, sample_words: MUSIC_SAMPLES },
    ModuleSpec { name: "JSON", words: JSON_WORDS, sample_words: &[] },
    ModuleSpec { name: "IO", words: IO_WORDS, sample_words: &[] },
    ModuleSpec { name: "TIME", words: TIME_WORDS, sample_words: &[] },
    ModuleSpec { name: "CRYPTO", words: CRYPTO_WORDS, sample_words: &[] },
    ModuleSpec { name: "ALGO", words: ALGO_WORDS, sample_words: &[] },
];
