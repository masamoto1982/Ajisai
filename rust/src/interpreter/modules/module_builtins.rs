use crate::builtins::WordShape;
use crate::coreword_registry::{
    self, CanonicalHome, CorewordMetadata, NilPolicy, Partiality, SafetyLevel, WordProfile,
    WordPurity,
};
use crate::interpreter::{
    algo_ops, audio, data_ops, datetime, hash, interval_ops, json, math_ops, random, serial, sort,
    tier2_ops, time_ops, HostCapability,
};
use crate::types::{Capabilities, Stability};

use super::module_word_types::{ModuleSpec, ModuleWord};

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
        "HAS",
        "True if a JSON object contains the given key",
        json::op_json_has,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "VALUES",
        "Get all values from a JSON object",
        json::op_json_values,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "MERGE",
        "Merge two JSON objects; right-hand keys win on conflict",
        json::op_json_merge,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "DELETE",
        "Remove a key from a JSON object",
        json::op_json_delete,
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

const DATA_WORDS: &[ModuleWord] = &[
    module_word!(
        "CSV-PARSE",
        "Parse CSV text into a vector of Records",
        data_ops::op_csv_parse,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "CSV-STRINGIFY",
        "Convert a vector of Records into CSV text",
        data_ops::op_csv_stringify,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "SELECT",
        "Project a table onto the named columns",
        data_ops::op_select,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "WHERE",
        "Keep table rows whose column predicate is true",
        data_ops::op_where,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
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
        "Render an instant as civil [Y M D h m s] at a UTC offset (hours)",
        time_ops::op_datetime,
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
        "Resolve a civil datetime to an instant at a UTC offset (hours)",
        time_ops::op_timestamp,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "DATE",
        "Extract the [Y M D] date from a datetime",
        time_ops::op_date,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "TIME",
        "Extract the [h m s] time-of-day from a datetime",
        time_ops::op_time,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "YEAR",
        "Year field of a date or datetime",
        time_ops::op_year,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "MONTH",
        "Month field of a date or datetime",
        time_ops::op_month,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "DAY",
        "Day field of a date or datetime",
        time_ops::op_day,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "HOUR",
        "Hour field of a time or datetime",
        time_ops::op_hour,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "MINUTE",
        "Minute field of a time or datetime",
        time_ops::op_minute,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "SECOND",
        "Second field of a time or datetime",
        time_ops::op_second,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "WEEKDAY",
        "ISO weekday of a date or datetime (Monday=1 .. Sunday=7)",
        time_ops::op_weekday,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "ADD-DAYS",
        "Shift a date or datetime by N whole days",
        time_ops::op_add_days,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "DIFF-DAYS",
        "Whole-day difference a-b between two dates/datetimes",
        time_ops::op_diff_days,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "FORMAT",
        "ISO-8601 text for a date (YYYY-MM-DD) or datetime (YYYY-MM-DDThh:mm:ss)",
        time_ops::op_format,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "PARSE-ISO",
        "Parse an ISO-8601 civil string into a datetime; Bubble/NIL if invalid",
        time_ops::op_parse,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "ADD-MONTHS",
        "Add N months to a date/datetime, clamping to the month end",
        time_ops::op_add_months,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::TIME
    ),
    module_word!(
        "ADD-YEARS",
        "Add N years to a date/datetime, clamping Feb 29 in non-leap years",
        time_ops::op_add_years,
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

const ALGO_WORDS: &[ModuleWord] = &[
    module_word!(
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
    ),
    module_word!(
        "UNIQUE",
        WordShape::Form,
        "Remove duplicate elements, preserving first-occurrence order",
        algo_ops::op_unique,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "CONTAINS",
        WordShape::Form,
        "True if a vector contains an element equal to the given value",
        algo_ops::op_contains,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "INDEX-OF",
        WordShape::Form,
        "Index of the first element equal to the value; Bubble/NIL if absent",
        algo_ops::op_index_of,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
];

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
    module_word!(
        "ABS",
        WordShape::Map,
        "Absolute value of a number.",
        math_ops::op_abs,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "NEG",
        WordShape::Map,
        "Negate a number.",
        math_ops::op_neg,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "SIGN",
        WordShape::Map,
        "Sign of a number: -1, 0, or 1.",
        math_ops::op_sign,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "MIN",
        WordShape::Form,
        "Smaller of two numbers.",
        math_ops::op_min,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "MAX",
        WordShape::Form,
        "Larger of two numbers.",
        math_ops::op_max,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "POW",
        WordShape::Form,
        "Integer-exponent exact power: base exp -- base^exp.",
        math_ops::op_pow,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "GCD",
        WordShape::Form,
        "Greatest common divisor of two integers.",
        math_ops::op_gcd,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "LCM",
        WordShape::Form,
        "Least common multiple of two integers.",
        math_ops::op_lcm,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Stable,
        Capabilities::PURE
    ),
    module_word!(
        "PI",
        WordShape::Form,
        "Push the exact real pi as a refinable rational enclosure.",
        tier2_ops::op_pi,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
        Capabilities::PURE
    ),
    module_word!(
        "ENCLOSE",
        WordShape::Form,
        "Observe a value's rational enclosure within an explicit water budget.",
        tier2_ops::op_enclose,
        WordPurity::Pure,
        &[],
        true,
        true,
        false,
        Stability::Experimental,
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

pub(super) const MODULE_SPECS: &[ModuleSpec] = &[
    ModuleSpec {
        name: "MUSIC",
        words: MUSIC_WORDS,
    },
    ModuleSpec {
        name: "JSON",
        words: JSON_WORDS,
    },
    ModuleSpec {
        name: "DATA",
        words: DATA_WORDS,
    },
    ModuleSpec {
        name: "IO",
        words: IO_WORDS,
    },
    ModuleSpec {
        name: "TIME",
        words: TIME_WORDS,
    },
    ModuleSpec {
        name: "CRYPTO",
        words: CRYPTO_WORDS,
    },
    ModuleSpec {
        name: "ALGO",
        words: ALGO_WORDS,
    },
    ModuleSpec {
        name: "MATH",
        words: MATH_WORDS,
    },
    ModuleSpec {
        name: "SERIAL",
        words: SERIAL_WORDS,
    },
];

/// One catalog entry for a module word as declared in
/// `MODULE_SPECS`, independent of whether it is currently imported. The GUI
/// uses this to render the full module dictionary (active + inactive words)
/// so an inactive word can be surfaced greyed-out and toggled with IMPORT-ONLY.
pub(crate) struct CatalogWord {
    pub short_name: &'static str,
    pub description: &'static str,
}

/// All importable module names, in specification order.
pub(crate) fn available_module_names() -> Vec<&'static str> {
    MODULE_SPECS.iter().map(|m| m.name).collect()
}

/// Full word catalog for a module, read directly from `MODULE_SPECS`
/// so it is available even when the module has never been imported. Returns
/// `None` when the module name is unknown.
pub(crate) fn module_catalog_words(module_name: &str) -> Option<Vec<CatalogWord>> {
    let upper = module_name.to_uppercase();
    let module = MODULE_SPECS.iter().find(|m| m.name == upper)?;
    let out: Vec<CatalogWord> = module
        .words
        .iter()
        .map(|w| CatalogWord {
            short_name: w.short_name,
            description: w.description,
        })
        .collect();
    Some(out)
}

pub(crate) fn module_word_description(module_name: &str, short_name: &str) -> Option<&'static str> {
    let module = MODULE_SPECS.iter().find(|m| m.name == module_name)?;
    module
        .words
        .iter()
        .find(|w| w.short_name == short_name)
        .map(|w| w.description)
}

/// Render the four-section LOOKUP body for a module word, given a
/// qualified `MODULE@WORD` name (e.g. `"JSON@PARSE"`). Returns `None` if
/// the word does not exist.
///
/// `Category / Summary / Role / Stack Effect` come from the authored
/// table in `super::module_word_docs`. Category is derived from the
/// module name; the other three are looked up by `(module, short_name)`.
pub(crate) fn lookup_module_word_detail(name: &str) -> Option<String> {
    let upper = name.to_uppercase();
    let (module_name, short_name) = upper.split_once('@')?;
    let module = MODULE_SPECS.iter().find(|m| m.name == module_name)?;
    let word = module.words.iter().find(|w| w.short_name == short_name)?;
    let stability_str = match word.stability {
        Stability::Stable => "stable",
        Stability::Experimental => "experimental",
    };
    let category = module.name.to_lowercase();
    let doc = super::module_word_docs::lookup_module_word_doc(module.name, word.short_name)?;
    Some(crate::builtins::render_four_section(
        "",
        &format!("{}@{}", module.name, word.short_name),
        stability_str,
        &category,
        doc.summary,
        doc.role,
        doc.stack_effect,
    ))
}

/// Per-word contract overrides for module words whose `partiality` /
/// `nil_policy` differ from the purity-class default produced by
/// `coreword_registry::{pure,observable,effectful}`.
fn contract_override(module: &str, word: &str) -> Option<(Partiality, NilPolicy)> {
    match (module, word) {
        // SERIAL@READ projects the no-data / disconnected condition onto
        // Bubble/NIL (Section 9.4), so it is Projecting/CreatesNil rather
        // than the effectful default of Partial/RejectsNil.
        ("SERIAL", "READ") => Some((Partiality::Projecting, NilPolicy::CreatesNil)),
        // MATH@POW projects 0 raised to a negative exponent onto Bubble/NIL
        // (reason = divisionByZero) while erroring on malformed use.
        ("MATH", "POW") => Some((Partiality::Projecting, NilPolicy::CreatesNil)),
        // MATH@GCD / MATH@LCM raise an error on non-integer numeric inputs
        // (malformed use, cf. CHR) and pass NIL operands through.
        ("MATH", "GCD") | ("MATH", "LCM") => Some((Partiality::Partial, NilPolicy::Passthrough)),
        // ALGO@INDEX-OF projects a well-formed miss (value absent from a
        // valid vector) onto Bubble/NIL with reason = missingField.
        ("ALGO", "INDEX-OF") => Some((Partiality::Projecting, NilPolicy::CreatesNil)),
        // MIN / MAX / SORT are total-by-projection (SPEC §7.4.3, §7.14): an
        // undecidable governing comparison is projected onto the logical
        // Unknown (U), so they are `Projecting`, not the pure-class default
        // `Total`. NIL operands pass through (with NIL taking priority over a
        // U-producing comparison, §4.5.2), so `nil_policy` stays `Passthrough`.
        ("MATH", "MIN") | ("MATH", "MAX") | ("ALGO", "SORT") => {
            Some((Partiality::Projecting, NilPolicy::Passthrough))
        }
        // TIME@PARSE-ISO projects an unparseable-but-well-formed text value
        // onto Bubble/NIL with reason = invalidEncoding (cf. NUM).
        ("TIME", "PARSE-ISO") => Some((Partiality::Projecting, NilPolicy::CreatesNil)),
        _ => None,
    }
}

fn host_capability_for_module_word(module: &str, word: &str) -> Option<HostCapability> {
    match (module, word) {
        ("TIME", "NOW") => Some(HostCapability::Clock),
        ("CRYPTO", "CSPRNG") => Some(HostCapability::SecureRandom),
        ("SERIAL", _) => Some(HostCapability::Serial),
        ("MUSIC", _) => Some(HostCapability::Audio),
        ("JSON", "EXPORT") => Some(HostCapability::JsonExport),
        ("IO", "INPUT") | ("IO", "OUTPUT") => Some(HostCapability::Effect),
        _ => None,
    }
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
                if let Some(capability) =
                    host_capability_for_module_word(spec.name, word.short_name)
                {
                    metadata.profile = WordProfile::Hosted;
                    metadata.required_capability = Some(capability);
                }
                metadata.canonical_home = CanonicalHome::Module(spec.name.to_string());
                metadata.listed_in_core = false;
                metadata.listed_in_modules = vec![spec.name.to_string()];
                metadata.listed_in_categories = Vec::new();
                if let Some((partiality, nil_policy)) =
                    contract_override(spec.name, word.short_name)
                {
                    metadata.partiality = partiality;
                    metadata.nil_policy = nil_policy;
                }
                // SPEC §7.14: safety A is reserved for *total* words. A pure
                // word that the override makes `Partial` (it raises on some
                // well-shaped input, e.g. MATH@GCD / MATH@LCM on non-integers)
                // must be safety B ("partial but with explicit error
                // categories"). `Projecting` is total-by-projection and stays A.
                if metadata.safety_level == SafetyLevel::A
                    && metadata.partiality == Partiality::Partial
                {
                    metadata.safety_level = SafetyLevel::B;
                }
                metadata
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::MODULE_SPECS;

    #[test]
    fn every_module_word_has_authored_four_section_doc() {
        if let Err(msg) = super::super::module_word_docs::assert_every_word_has_doc(MODULE_SPECS) {
            panic!("{}", msg);
        }
    }

    #[test]
    fn every_module_word_lookup_contains_all_four_sections() {
        for module in MODULE_SPECS {
            for word in module.words {
                let qualified = format!("{}@{}", module.name, word.short_name);
                let body = super::lookup_module_word_detail(&qualified)
                    .unwrap_or_else(|| panic!("{} produced no body", qualified));
                for section in ["Category:", "Summary:", "Role:", "Stack Effect:"] {
                    assert!(
                        body.contains(section),
                        "{} LOOKUP body missing section {}:\n{}",
                        qualified,
                        section,
                        body
                    );
                }
            }
        }
    }

    #[test]
    fn every_module_word_stack_effect_grammar() {
        for module in MODULE_SPECS {
            for word in module.words {
                let doc = super::super::module_word_docs::lookup_module_word_doc(
                    module.name,
                    word.short_name,
                )
                .expect("doc lookup");
                let s = doc.stack_effect;
                let is_literal_no_op = s == "no values popped or pushed";
                if is_literal_no_op {
                    continue;
                }
                assert!(
                    s.contains("->"),
                    "{}@{} stack_effect missing '->' arrow: {:?}",
                    module.name,
                    word.short_name,
                    s
                );
            }
        }
    }

    #[test]
    fn every_module_word_doc_is_utf8_plain_text() {
        for module in MODULE_SPECS {
            for word in module.words {
                let doc = super::super::module_word_docs::lookup_module_word_doc(
                    module.name,
                    word.short_name,
                )
                .expect("doc lookup");
                let q = format!("{}@{}", module.name, word.short_name);
                assert!(
                    !doc.summary.chars().any(|c| c.is_control() && c != '\n'),
                    "{} summary must be UTF-8 plain text without control characters",
                    q
                );
                assert!(
                    !doc.role.chars().any(|c| c.is_control() && c != '\n'),
                    "{} role must be UTF-8 plain text without control characters",
                    q
                );
                assert!(
                    !doc.stack_effect.chars().any(|c| c.is_control() && c != '\n'),
                    "{} stack_effect must be UTF-8 plain text without control characters",
                    q
                );
            }
        }
    }

    // The word-info area renders a module word's `description` on a single
    // line (CSS nowrap + ellipsis). A multi-line description overflows and
    // gets clipped, so descriptions must stay single-line.
    #[test]
    fn module_descriptions_are_single_line() {
        for module in MODULE_SPECS {
            for word in module.words {
                assert!(
                    !word.description.contains('\n'),
                    "module {} word {} has a multi-line description",
                    module.name,
                    word.short_name
                );
            }
        }
    }
}
