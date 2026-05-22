use crate::builtins::WordShape;
use crate::coreword_registry::{
    self, CanonicalHome, CorewordMetadata, NilPolicy, Partiality, WordPurity,
};
use crate::interpreter::{audio, datetime, hash, interval_ops, json, random, serial, sort};
use crate::types::{Capabilities, Stability};

use super::module_word_types::{ModuleSpec, ModuleWord, SampleWord};

macro_rules! module_word {
    ($name:expr, $word_shape:expr, $description:expr, $executor:expr, $purity:expr, $effects:expr, $det:expr, $preview:expr, $preserves:expr, $stability:expr, $caps:expr) => {
        ModuleWord {
            short_name: $name,
            description: $description,
            word_shape: Some($word_shape),
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
    ($name:expr, $description:expr, $executor:expr, $purity:expr, $effects:expr, $det:expr, $preview:expr, $preserves:expr, $stability:expr, $caps:expr) => {
        ModuleWord {
            short_name: $name,
            description: $description,
            word_shape: None,
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
        "SEQ-GROUP",
        "Build an explicit sequential music group from a vector",
        audio::op_seq_group,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "SIM-GROUP",
        "Build an explicit simultaneous music group from a vector",
        audio::op_sim_group,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "CHORD",
        "Build an explicit chord group (simultaneous) from a vector",
        audio::op_chord,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "HZ",
        "Build a music.pitch from a frequency in Hz (exact rational)",
        audio::op_hz,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "DUR",
        "Build a music.duration from a number of seconds",
        audio::op_dur,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "NOTE",
        "Combine a music.pitch and a music.duration into a music.note",
        audio::op_note,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "REST",
        "Build a music.rest from a music.duration",
        audio::op_rest,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "EDO",
        "Build an equal-division-of-the-octave music.tuning",
        audio::op_edo,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "EDR",
        "Build an equal-division-of-a-ratio music.tuning (non-octave)",
        audio::op_edr,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "STEP",
        "Resolve a step within a music.tuning into a music.pitch",
        audio::op_step,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "VOICE",
        "Build a music group with the role of a single melodic voice",
        audio::op_voice,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "TRACK",
        "Build a music group with the role of an instrument track",
        audio::op_track,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "MEASURE",
        "Build a music group with the role of a measure (bar)",
        audio::op_measure,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "PHRASE",
        "Build a music group with the role of a phrase",
        audio::op_phrase,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "WITH-TUNING",
        "Bind a tuning over a body so bare integers become tuning steps",
        audio::op_with_tuning,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "EXPLAIN",
        "Explain how MUSIC@PLAY would interpret a value",
        audio::op_explain,
        WordPurity::Effectful,
        &["audio-output"],
        true,
        false,
        false,
        Stability::Experimental,
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
    WordShape::Form,
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
        WordShape::Map,
        "Square root. Exact rational roots stay exact; otherwise returns sound interval.",
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
        WordShape::Form,
        "Square root with explicit interval width bound eps.",
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
        WordShape::Form,
        "Create interval [lo, hi].",
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
        WordShape::Map,
        "Lower endpoint of number/interval.",
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
        WordShape::Map,
        "Upper endpoint of number/interval.",
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
        WordShape::Map,
        "Interval width hi-lo.",
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
        WordShape::Map,
        "True for exact number or degenerate interval.",
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

const SERIAL_WORDS: &[ModuleWord] = &[
    module_word!(
        "LIST-PORTS",
        "Ask the host to enumerate available serial ports",
        serial::op_list_ports,
        WordPurity::Effectful,
        &["serial-query"],
        false,
        false,
        false,
        Stability::Experimental,
        Capabilities::IO
    ),
    module_word!(
        "OPEN",
        "Open a serial port by id; leaves the port-id handle on the stack",
        serial::op_open,
        WordPurity::Effectful,
        &["serial-control"],
        false,
        false,
        false,
        Stability::Experimental,
        Capabilities::IO
    ),
    module_word!(
        "CONFIGURE",
        "Set the baud rate of an open serial port",
        serial::op_configure,
        WordPurity::Effectful,
        &["serial-control"],
        false,
        false,
        false,
        Stability::Experimental,
        Capabilities::IO
    ),
    module_word!(
        "WRITE",
        "Write a byte vector to an open serial port",
        serial::op_write,
        WordPurity::Effectful,
        &["serial-write"],
        false,
        false,
        false,
        Stability::Experimental,
        Capabilities::IO
    ),
    module_word!(
        "READ",
        "Drain received bytes from an open serial port; Bubble/NIL when none",
        serial::op_read,
        WordPurity::Effectful,
        &["serial-read"],
        false,
        false,
        false,
        Stability::Experimental,
        Capabilities::IO
    ),
    module_word!(
        "FLUSH",
        "Flush the outgoing buffer of an open serial port",
        serial::op_flush,
        WordPurity::Effectful,
        &["serial-control"],
        false,
        false,
        false,
        Stability::Experimental,
        Capabilities::IO
    ),
    module_word!(
        "CLOSE",
        "Close an open serial port",
        serial::op_close,
        WordPurity::Effectful,
        &["serial-control"],
        false,
        false,
        false,
        Stability::Experimental,
        Capabilities::IO
    ),
];

const MUSIC_SAMPLES: &[SampleWord] = &[];

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
    ModuleSpec {
        name: "SERIAL",
        words: SERIAL_WORDS,
        sample_words: &[],
    },
];

pub(crate) fn module_word_description(module_name: &str, short_name: &str) -> Option<&'static str> {
    let module = MODULE_SPECS.iter().find(|m| m.name == module_name)?;
    module
        .words
        .iter()
        .find(|w| w.short_name == short_name)
        .map(|w| w.description)
}

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
                metadata.canonical_home = CanonicalHome::Module(spec.name.to_string());
                metadata.listed_in_core = false;
                metadata.listed_in_modules = vec![spec.name.to_string()];
                metadata.listed_in_categories = Vec::new();
                // SERIAL@READ projects the no-data / disconnected condition onto
                // Bubble/NIL (Section 9.4), so it is Projecting/CreatesNil rather
                // than the effectful default of Partial/RejectsNil.
                if spec.name == "SERIAL" && word.short_name == "READ" {
                    metadata.partiality = Partiality::Projecting;
                    metadata.nil_policy = NilPolicy::CreatesNil;
                }
                metadata
            })
        })
        .collect()
}
