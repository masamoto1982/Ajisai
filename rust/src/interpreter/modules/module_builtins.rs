use crate::coreword_registry::{self, CorewordMetadata, WordPurity};
use crate::interpreter::{audio, datetime, hash, interval_ops, json, random, sort};
use crate::types::{Capabilities, Stability};

use super::module_word_types::{ModuleSpec, ModuleWord, SampleWord};

macro_rules! module_word {
    ($name:expr, $description:expr, $executor:expr, $purity:expr, $effects:expr, $det:expr, $preview:expr, $preserves:expr, $stability:expr, $caps:expr) => {
        ModuleWord {
            short_name: $name,
            description: $description,
            executor: $executor,
            purity: $purity,
            effects: $effects,
            deterministic: $det,
            safe_preview: $preview,
            preserves_modes: $preserves,
            stability: $stability,
            capabilities: $caps,
        }
    };
}

const MUSIC_WORDS: &[ModuleWord] = &[
    module_word!(
        "SEQ",
        "Set sequential playback mode",
        audio::op_seq,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        true,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "SIM",
        "Set simultaneous playback mode",
        audio::op_sim,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        true,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "SLOT",
        "Set slot duration in seconds",
        audio::op_slot,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "GAIN",
        "Set volume level (0.0-1.0)",
        audio::op_gain,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "GAIN-RESET",
        "Reset volume to default (1.0)",
        audio::op_gain_reset,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "PAN",
        "Set stereo position (-1.0 left to 1.0 right)",
        audio::op_pan,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "PAN-RESET",
        "Reset pan to center (0.0)",
        audio::op_pan_reset,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "FX-RESET",
        "Reset all audio effects to defaults",
        audio::op_fx_reset,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "PLAY",
        "Play audio",
        audio::op_play,
        WordPurity::Effectful,
        &["audio-output"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "CHORD",
        "Mark vector as chord (simultaneous)",
        audio::op_chord,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "ADSR",
        "Set ADSR envelope",
        audio::op_adsr,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "SINE",
        "Set sine waveform",
        audio::op_sine,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "SQUARE",
        "Set square waveform",
        audio::op_square,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "SAW",
        "Set sawtooth waveform",
        audio::op_saw,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "TRI",
        "Set triangle waveform",
        audio::op_tri,
        WordPurity::Effectful,
        &["audio-control"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
];

const JSON_WORDS: &[ModuleWord] = &[
    module_word!(
        "PARSE",
        "Parse JSON string to Ajisai value",
        json::op_parse,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "STRINGIFY",
        "Convert Ajisai value to JSON string",
        json::op_stringify,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "GET",
        "Get value by key from JSON object",
        json::op_json_get,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "KEYS",
        "Get all keys from JSON object",
        json::op_json_keys,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "SET",
        "Set key-value in JSON object",
        json::op_json_set,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "EXPORT",
        "Export stack top as JSON file download",
        json::op_json_export,
        WordPurity::Effectful,
        &["file-write"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
];

const IO_WORDS: &[ModuleWord] = &[
    module_word!(
        "INPUT",
        "Read text from input buffer",
        json::op_input,
        WordPurity::Observable,
        &["io-read"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
    module_word!(
        "OUTPUT",
        "Write value to output buffer",
        json::op_output,
        WordPurity::Effectful,
        &["io-write"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::IO
    ),
];

const TIME_WORDS: &[ModuleWord] = &[
    module_word!(
        "NOW",
        "Get current Unix timestamp",
        datetime::op_now,
        WordPurity::Observable,
        &["time-read"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "DATETIME",
        "Convert timestamp to datetime vector",
        datetime::op_datetime,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "TIMESTAMP",
        "Convert datetime vector to timestamp",
        datetime::op_timestamp,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
];

const CRYPTO_WORDS: &[ModuleWord] = &[
    // RANDOM handling: CSPRNG reads an external entropy source, so it is modeled as observable.
    module_word!(
        "CSPRNG",
        "Generate cryptographically secure random numbers",
        random::op_csprng,
        WordPurity::Observable,
        &["random-read"],
        false,
        false,
        false,
        Stability::Stable,
        Capabilities::RANDOM.union(Capabilities::CRYPTO)
    ),
    module_word!(
        "HASH",
        "Compute hash value",
        hash::op_hash,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE.union(Capabilities::CRYPTO)
    ),
];

const ALGO_WORDS: &[ModuleWord] = &[module_word!(
    "SORT",
    "Sort vector elements in ascending order",
    sort::op_sort,
    WordPurity::Pure,
    &[],
    true,
    true,
    false,
    Stability::Stable,
    Capabilities::PURE
)];

const MATH_WORDS: &[ModuleWord] = &[
    module_word!(
        "SQRT",
        "Map: Square root. Exact rational roots stay exact; otherwise returns sound interval.",
        interval_ops::op_sqrt,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "SQRT-EPS",
        "Form: Square root with explicit interval width bound eps.",
        interval_ops::op_sqrt_eps,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "INTERVAL",
        "Form: Create interval [lo, hi].",
        interval_ops::op_interval,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "LOWER",
        "Map: Lower endpoint of number/interval.",
        interval_ops::op_lower,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "UPPER",
        "Map: Upper endpoint of number/interval.",
        interval_ops::op_upper,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "WIDTH",
        "Map: Interval width hi-lo.",
        interval_ops::op_width,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "IS-EXACT",
        "Map: True for exact number or degenerate interval.",
        interval_ops::op_is_exact,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
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

pub(super) const MODULE_SPECS: &[ModuleSpec] = &[
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
    ModuleSpec {
        name: "TIME",
        words: TIME_WORDS,
        sample_words: &[],
    },
    ModuleSpec {
        name: "CRYPTO",
        words: CRYPTO_WORDS,
        sample_words: &[],
    },
    ModuleSpec {
        name: "ALGO",
        words: ALGO_WORDS,
        sample_words: &[],
    },
    ModuleSpec {
        name: "MATH",
        words: MATH_WORDS,
        sample_words: &[],
    },
];

pub(crate) fn module_word_metadata_entries() -> Vec<CorewordMetadata> {
    MODULE_SPECS
        .iter()
        .flat_map(|spec| {
            spec.words.iter().map(move |word| {
                let mut metadata = match word.purity {
                    WordPurity::Pure => {
                        coreword_registry::pure(word.short_name, &spec.name.to_lowercase())
                    }
                    WordPurity::Observable => coreword_registry::observable(
                        word.short_name,
                        &spec.name.to_lowercase(),
                        word.effects,
                        Some(word.deterministic),
                    ),
                    WordPurity::Effectful => coreword_registry::effectful(
                        word.short_name,
                        &spec.name.to_lowercase(),
                        word.effects,
                    ),
                };
                metadata.deterministic = word.deterministic;
                metadata.safe_preview = word.safe_preview;
                metadata.formerly_module = Some(spec.name.to_string());
                metadata
            })
        })
        .collect()
}
