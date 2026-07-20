#[cfg(test)]
use super::module_word_types::ModuleSpec;

/// Authored four-section docs for a module word. The table below is the
/// canonical source of `Category` / `Summary` / `Role` / `Stack Effect`
/// for module-imported words. `Category` is derived from the module name
/// at render time, so only the three text fields are stored.
#[derive(Clone, Copy)]
pub(super) struct ModuleWordDoc {
    pub module: &'static str,
    pub word: &'static str,
    pub summary: &'static str,
    pub role: &'static str,
    pub stack_effect: &'static str,
}

pub(super) fn lookup_module_word_doc(module: &str, word: &str) -> Option<&'static ModuleWordDoc> {
    MODULE_WORD_DOCS
        .iter()
        .find(|d| d.module == module && d.word == word)
}

#[cfg(test)]
pub(super) fn assert_every_word_has_doc(specs: &[ModuleSpec]) -> Result<(), String> {
    for module in specs {
        for word in module.words {
            let doc = MODULE_WORD_DOCS
                .iter()
                .find(|d| d.module == module.name && d.word == word.short_name);
            match doc {
                None => {
                    return Err(format!(
                        "missing module word doc entry for {}@{}",
                        module.name, word.short_name
                    ))
                }
                Some(d) => {
                    if d.summary.is_empty() {
                        return Err(format!("{}@{} has empty summary", d.module, d.word));
                    }
                    if d.role.is_empty() {
                        return Err(format!("{}@{} has empty role", d.module, d.word));
                    }
                    if d.stack_effect.is_empty() {
                        return Err(format!("{}@{} has empty stack_effect", d.module, d.word));
                    }
                }
            }
        }
    }
    Ok(())
}

const MODULE_WORD_DOCS: &[ModuleWordDoc] = &[
    // ==================================================================
    // DATA
    // ==================================================================
    ModuleWordDoc {
        module: "DATA",
        word: "CSV-PARSE",
        summary: "Parse CSV text into a vector of Records (the first row is the header).",
        role: "Pure table reader: text in, one Record per data row. Malformed or ragged CSV projects to a reasoned NIL.",
        stack_effect: "[ text ] -> [ records ]",
    },
    ModuleWordDoc {
        module: "DATA",
        word: "CSV-STRINGIFY",
        summary: "Render a vector of Records as CSV text sharing one column shape.",
        role: "Pure table writer: the inverse of CSV-PARSE. A non-table or shape-mismatched input projects to a reasoned NIL.",
        stack_effect: "[ records ] -> [ text ]",
    },
    ModuleWordDoc {
        module: "DATA",
        word: "SELECT",
        summary: "Project a table onto the named columns, in order.",
        role: "Pure column selection: an absent column yields a NIL (MissingField) cell so the result stays rectangular.",
        stack_effect: "[ table ] [ columns ] -> [ table ]",
    },
    ModuleWordDoc {
        module: "DATA",
        word: "WHERE",
        summary: "Keep the rows whose predicate on a named column is true.",
        role: "Pure row selection: a false, UNKNOWN, or NIL (missing column) predicate result drops the row; the result is always a table.",
        stack_effect: "[ table ] [ column ] [ predicate ] -> [ table ]",
    },
    // ==================================================================
    // MUSIC
    // ==================================================================
    ModuleWordDoc {
        module: "MUSIC",
        word: "SEQ",
        summary: "Set the active playback mode to sequential.",
        role: "Playback-mode modifier: subsequent grouping words emit notes in order.",
        stack_effect: "no values popped or pushed",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "SIM",
        summary: "Set the active playback mode to simultaneous.",
        role: "Playback-mode modifier: subsequent grouping words emit notes in parallel.",
        stack_effect: "no values popped or pushed",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "SLOT",
        summary: "Set the slot duration (in seconds) used by bare notes.",
        role: "Timing control for the default slot length.",
        stack_effect: "[ secs ] -> no values popped or pushed",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "GAIN",
        summary: "Set the master output gain (0.0-1.0).",
        role: "Output-level control for the audio engine.",
        stack_effect: "[ level ] -> no values popped or pushed",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "GAIN-RESET",
        summary: "Reset the master gain to the default 1.0.",
        role: "Audio control that restores the default output level.",
        stack_effect: "no values popped or pushed",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "PAN",
        summary: "Set the stereo pan position (-1.0 left .. 1.0 right).",
        role: "Stereo-placement control for the audio engine.",
        stack_effect: "[ pan ] -> no values popped or pushed",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "PAN-RESET",
        summary: "Reset pan to center (0.0).",
        role: "Audio control that restores the default stereo position.",
        stack_effect: "no values popped or pushed",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "FX-RESET",
        summary: "Reset all audio effects (gain, pan, envelope, waveform) to defaults.",
        role: "Bulk audio-control reset.",
        stack_effect: "no values popped or pushed",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "PLAY",
        summary: "Play a music value (note, group, voice, ...).",
        role: "Primary audio-output trigger for the music module.",
        stack_effect: "[ music ] -> no values popped or pushed",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "SEQ-GROUP",
        summary: "Build an explicit sequential music group from a vector of notes.",
        role: "Structural grouping word that fixes sequential semantics independent of the ambient mode.",
        stack_effect: "[ notes ] -> [ group ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "SIM-GROUP",
        summary: "Build an explicit simultaneous music group from a vector of notes.",
        role: "Structural grouping word that fixes simultaneous semantics independent of the ambient mode.",
        stack_effect: "[ notes ] -> [ group ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "CHORD",
        summary: "Build a chord (simultaneous group) from a vector of pitches or notes.",
        role: "Convenience constructor for harmonic groupings.",
        stack_effect: "[ pitches ] -> [ chord ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "HZ",
        summary: "Build a music.pitch from a frequency in hertz.",
        role: "Pitch constructor that names a frequency exactly as a rational.",
        stack_effect: "[ hz ] -> [ pitch ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "DUR",
        summary: "Build a music.duration from a number of seconds.",
        role: "Duration constructor for use in NOTE / REST.",
        stack_effect: "[ secs ] -> [ duration ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "NOTE",
        summary: "Combine a music.pitch and a music.duration into a music.note.",
        role: "Primary note constructor.",
        stack_effect: "[ pitch ] [ duration ] -> [ note ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "REST",
        summary: "Build a music.rest from a music.duration.",
        role: "Constructor for silence within a sequence.",
        stack_effect: "[ duration ] -> [ rest ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "EDO",
        summary: "Build an equal-division-of-the-octave tuning.",
        role: "Tuning constructor for N-equal divisions of 2/1.",
        stack_effect: "[ ref-hz ] [ divisions ] -> [ tuning ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "EDR",
        summary: "Build an equal-division-of-a-ratio tuning (non-octave).",
        role: "Tuning constructor for N-equal divisions of an arbitrary equave.",
        stack_effect: "[ ref-hz ] [ equave ] [ divisions ] -> [ tuning ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "STEP",
        summary: "Resolve a tuning step into an exact music.pitch.",
        role: "Index-into-tuning operator.",
        stack_effect: "[ tuning ] [ step ] -> [ pitch ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "VOICE",
        summary: "Build a music group with the role of a single melodic voice.",
        role: "Structural role tag for a single-line voice.",
        stack_effect: "[ notes ] -> [ voice ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "TRACK",
        summary: "Build a music group with the role of an instrument track.",
        role: "Structural role tag for an instrument track.",
        stack_effect: "[ notes ] -> [ track ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "MEASURE",
        summary: "Build a music group with the role of a measure (bar).",
        role: "Structural role tag marking a measure boundary.",
        stack_effect: "[ notes ] -> [ measure ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "PHRASE",
        summary: "Build a music group with the role of a phrase.",
        role: "Structural role tag for a phrase grouping.",
        stack_effect: "[ notes ] -> [ phrase ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "WITH-TUNING",
        summary: "Bind a tuning over a body so bare integers are read as tuning steps.",
        role: "Scoped binding that re-interprets numeric literals as steps within a tuning.",
        stack_effect: "[ tuning ] [ body ] -> [ scope ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "EXPLAIN",
        summary: "Describe how MUSIC@PLAY would interpret a value, without playing it.",
        role: "Diagnostic / inspection word for music values.",
        stack_effect: "[ music ] -> [ music ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "ADSR",
        summary: "Set the ADSR envelope used by subsequent notes.",
        role: "Envelope control for the audio engine.",
        stack_effect: "[ target ] [ params ] -> [ target ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "SINE",
        summary: "Select the sine waveform on a target.",
        role: "Waveform control for synthesised notes.",
        stack_effect: "[ target ] -> [ target ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "SQUARE",
        summary: "Select the square waveform on a target.",
        role: "Waveform control for synthesised notes.",
        stack_effect: "[ target ] -> [ target ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "SAW",
        summary: "Select the sawtooth waveform on a target.",
        role: "Waveform control for synthesised notes.",
        stack_effect: "[ target ] -> [ target ]",
    },
    ModuleWordDoc {
        module: "MUSIC",
        word: "TRI",
        summary: "Select the triangle waveform on a target.",
        role: "Waveform control for synthesised notes.",
        stack_effect: "[ target ] -> [ target ]",
    },
    // ==================================================================
    // JSON
    // ==================================================================
    ModuleWordDoc {
        module: "JSON",
        word: "PARSE",
        summary: "Parse a JSON string into an Ajisai value.",
        role: "JSON ingress operator.",
        stack_effect: "[ text ] -> [ value ]",
    },
    ModuleWordDoc {
        module: "JSON",
        word: "STRINGIFY",
        summary: "Serialise an Ajisai value to a JSON string.",
        role: "JSON egress operator.",
        stack_effect: "[ value ] -> [ text ]",
    },
    ModuleWordDoc {
        module: "JSON",
        word: "GET",
        summary: "Look up a key in a JSON object.",
        role: "Field access on JSON objects.",
        stack_effect: "[ obj ] [ key ] -> [ value ]",
    },
    ModuleWordDoc {
        module: "JSON",
        word: "KEYS",
        summary: "Return all keys of a JSON object as a vector.",
        role: "Introspection of a JSON object's shape.",
        stack_effect: "[ obj ] -> [ keys ]",
    },
    ModuleWordDoc {
        module: "JSON",
        word: "SET",
        summary: "Return a JSON object with the given key bound to the given value.",
        role: "Functional update on a JSON object.",
        stack_effect: "[ obj ] [ key ] [ value ] -> [ obj' ]",
    },
    ModuleWordDoc {
        module: "JSON",
        word: "HAS",
        summary: "True if a JSON object contains the given key.",
        role: "Membership check on a JSON object.",
        stack_effect: "[ obj ] [ key ] -> [ bool ]",
    },
    ModuleWordDoc {
        module: "JSON",
        word: "VALUES",
        summary: "Return all values of a JSON object as a vector.",
        role: "Introspection of a JSON object's content.",
        stack_effect: "[ obj ] -> [ values ]",
    },
    ModuleWordDoc {
        module: "JSON",
        word: "MERGE",
        summary: "Merge two JSON objects; right-hand keys win on conflict.",
        role: "Right-biased shallow merge operator.",
        stack_effect: "[ base ] [ overlay ] -> [ merged ]",
    },
    ModuleWordDoc {
        module: "JSON",
        word: "DELETE",
        summary: "Return a JSON object with the given key removed.",
        role: "Functional key removal on a JSON object.",
        stack_effect: "[ obj ] [ key ] -> [ obj' ]",
    },
    ModuleWordDoc {
        module: "JSON",
        word: "EXPORT",
        summary: "Export the top of the stack as a downloadable JSON file.",
        role: "Effectful egress to the host browser.",
        stack_effect: "[ value ] -> no values popped or pushed",
    },
    // ==================================================================
    // IO
    // ==================================================================
    ModuleWordDoc {
        module: "IO",
        word: "INPUT",
        summary: "Read text from the host input buffer.",
        role: "Observable host I/O ingress for textual input.",
        stack_effect: "no values popped or pushed -> [ text ]",
    },
    ModuleWordDoc {
        module: "IO",
        word: "OUTPUT",
        summary: "Write a value to the host output buffer.",
        role: "Effectful host I/O egress for textual output.",
        stack_effect: "[ value ] -> no values popped or pushed",
    },
    // ==================================================================
    // TIME
    // ==================================================================
    ModuleWordDoc {
        module: "TIME",
        word: "NOW",
        summary: "Return the current Unix timestamp.",
        role: "Observable clock read.",
        stack_effect: "no values popped or pushed -> [ timestamp ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "DATETIME",
        summary: "Render an instant as civil [ Y M D h m s ] at a UTC offset (hours).",
        role: "Convert an instant + offset to a civil datetime vector.",
        stack_effect: "[ timestamp ] [ offset ] -> [ datetime ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "TIMESTAMP",
        summary: "Resolve a civil datetime to an instant at a UTC offset (hours).",
        role: "Convert a civil datetime + offset into a Unix timestamp.",
        stack_effect: "[ datetime ] [ offset ] -> [ timestamp ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "DATE",
        summary: "Extract the [ Y M D ] date portion of a datetime.",
        role: "Projection of a datetime onto its date fields.",
        stack_effect: "[ datetime ] -> [ date ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "TIME",
        summary: "Extract the [ h m s ] time-of-day from a datetime.",
        role: "Projection of a datetime onto its clock fields.",
        stack_effect: "[ datetime ] -> [ time ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "YEAR",
        summary: "Return the year field of a date or datetime.",
        role: "Scalar field accessor.",
        stack_effect: "[ date-or-datetime ] -> [ year ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "MONTH",
        summary: "Return the month field of a date or datetime.",
        role: "Scalar field accessor.",
        stack_effect: "[ date-or-datetime ] -> [ month ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "DAY",
        summary: "Return the day field of a date or datetime.",
        role: "Scalar field accessor.",
        stack_effect: "[ date-or-datetime ] -> [ day ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "HOUR",
        summary: "Return the hour field of a time or datetime.",
        role: "Scalar field accessor.",
        stack_effect: "[ time-or-datetime ] -> [ hour ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "MINUTE",
        summary: "Return the minute field of a time or datetime.",
        role: "Scalar field accessor.",
        stack_effect: "[ time-or-datetime ] -> [ minute ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "SECOND",
        summary: "Return the second field of a time or datetime.",
        role: "Scalar field accessor.",
        stack_effect: "[ time-or-datetime ] -> [ second ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "WEEKDAY",
        summary: "Return the ISO weekday of a date or datetime (Monday=1 .. Sunday=7).",
        role: "Calendar projection onto ISO weekday numbering.",
        stack_effect: "[ date-or-datetime ] -> [ weekday ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "ADD-DAYS",
        summary: "Shift a date or datetime by N whole days.",
        role: "Calendar arithmetic in whole-day units.",
        stack_effect: "[ date-or-datetime ] [ n ] -> [ date-or-datetime' ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "DIFF-DAYS",
        summary: "Whole-day difference (a - b) between two dates or datetimes.",
        role: "Calendar arithmetic returning a signed day count.",
        stack_effect: "[ a ] [ b ] -> [ days ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "FORMAT",
        summary: "Render a date as YYYY-MM-DD or a datetime as YYYY-MM-DDThh:mm:ss.",
        role: "ISO-8601 text egress.",
        stack_effect: "[ date-or-datetime ] -> [ text ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "PARSE-ISO",
        summary: "Parse an ISO-8601 civil string into a datetime; Bubble/NIL if invalid.",
        role: "ISO-8601 text ingress (projecting).",
        stack_effect: "[ text ] -> [ datetime | NIL ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "ADD-MONTHS",
        summary: "Add N months to a date/datetime, clamping to the month end.",
        role: "Calendar arithmetic in whole-month units with end-of-month clamping.",
        stack_effect: "[ date-or-datetime ] [ n ] -> [ date-or-datetime' ]",
    },
    ModuleWordDoc {
        module: "TIME",
        word: "ADD-YEARS",
        summary: "Add N years to a date/datetime, clamping Feb 29 in non-leap years.",
        role: "Calendar arithmetic in whole-year units with leap-year clamping.",
        stack_effect: "[ date-or-datetime ] [ n ] -> [ date-or-datetime' ]",
    },
    // ==================================================================
    // CRYPTO
    // ==================================================================
    ModuleWordDoc {
        module: "CRYPTO",
        word: "CSPRNG",
        summary: "Generate cryptographically secure random rationals with the given denominator.",
        role: "Observable host source of cryptographic randomness.",
        stack_effect: "[ denom ] [ count ] -> [ randoms ]",
    },
    ModuleWordDoc {
        module: "CRYPTO",
        word: "HASH",
        summary: "Compute a cryptographic hash of a value at a chosen bit width.",
        role: "Pure deterministic digest operator.",
        stack_effect: "[ value ] [ bits ] -> [ digest ]",
    },
    // ==================================================================
    // ALGO
    // ==================================================================
    ModuleWordDoc {
        module: "ALGO",
        word: "SORT",
        summary: "Return a copy of a vector sorted in ascending order.",
        role: "General sorting primitive for the algo module.",
        stack_effect: "[ vec ] -> [ sorted ]",
    },
    ModuleWordDoc {
        module: "ALGO",
        word: "UNIQUE",
        summary: "Return a copy of a vector with duplicates removed, preserving first-occurrence order.",
        role: "Deduplication primitive.",
        stack_effect: "[ vec ] -> [ unique ]",
    },
    ModuleWordDoc {
        module: "ALGO",
        word: "CONTAINS",
        summary: "True if a vector contains an element equal to the given value.",
        role: "Membership test for vectors.",
        stack_effect: "[ vec ] [ value ] -> [ bool ]",
    },
    ModuleWordDoc {
        module: "ALGO",
        word: "INDEX-OF",
        summary: "Index of the first element equal to the value; Bubble/NIL if absent.",
        role: "Linear-search primitive that projects misses onto NIL.",
        stack_effect: "[ vec ] [ value ] -> [ index | NIL ]",
    },
    // ==================================================================
    // MATH
    // ==================================================================
    ModuleWordDoc {
        module: "MATH",
        word: "SQRT",
        summary: "Square root. Exact rational roots stay exact; otherwise returns a sound interval.",
        role: "Numeric primitive with exact/interval dispatch.",
        stack_effect: "[ x ] -> [ root ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "SQRT-EPS",
        summary: "Square root with an explicit interval width bound eps.",
        role: "Width-controlled variant of SQRT for interval arithmetic.",
        stack_effect: "[ x ] [ eps ] -> [ root ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "INTERVAL",
        summary: "Create a sound interval [ lo, hi ].",
        role: "Interval constructor.",
        stack_effect: "[ lo ] [ hi ] -> [ interval ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "LOWER",
        summary: "Lower endpoint of a number or interval.",
        role: "Endpoint projection for interval values.",
        stack_effect: "[ x ] -> [ lo ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "UPPER",
        summary: "Upper endpoint of a number or interval.",
        role: "Endpoint projection for interval values.",
        stack_effect: "[ x ] -> [ hi ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "WIDTH",
        summary: "Width of an interval (hi - lo).",
        role: "Interval-width projection.",
        stack_effect: "[ x ] -> [ width ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "IS-EXACT",
        summary: "True for an exact number or a degenerate (zero-width) interval.",
        role: "Predicate distinguishing exact values from sound intervals.",
        stack_effect: "[ x ] -> [ bool ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "ABS",
        summary: "Absolute value of a number.",
        role: "Sign-stripping numeric primitive.",
        stack_effect: "[ x ] -> [ abs ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "NEG",
        summary: "Numeric negation.",
        role: "Sign-flipping numeric primitive.",
        stack_effect: "[ x ] -> [ -x ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "SIGN",
        summary: "Sign of a number: -1, 0, or 1.",
        role: "Sign extraction primitive.",
        stack_effect: "[ x ] -> [ sign ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "MIN",
        summary: "Smaller of two numbers.",
        role: "Ordering primitive returning the lesser operand.",
        stack_effect: "[ a ] [ b ] -> [ min ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "MAX",
        summary: "Larger of two numbers.",
        role: "Ordering primitive returning the greater operand.",
        stack_effect: "[ a ] [ b ] -> [ max ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "POW",
        summary: "Integer-exponent exact power: base^exp.",
        role: "Exact-power primitive; projects 0^negative onto Bubble/NIL.",
        stack_effect: "[ base ] [ exp ] -> [ result ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "GCD",
        summary: "Greatest common divisor of two integers.",
        role: "Integer number-theory primitive.",
        stack_effect: "[ a ] [ b ] -> [ gcd ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "LCM",
        summary: "Least common multiple of two integers.",
        role: "Integer number-theory primitive.",
        stack_effect: "[ a ] [ b ] -> [ lcm ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "PI",
        summary: "Push the exact real pi as a refinable rational enclosure.",
        role: "Tier 2 numeric constant; its order can be observed within a water budget.",
        stack_effect: "[ ] -> [ pi ]",
    },
    ModuleWordDoc {
        module: "MATH",
        word: "ENCLOSE",
        summary: "Observe a value's rational enclosure within an explicit water budget.",
        role: "Water-explicit observation returning a sound [ lo, hi ] interval.",
        stack_effect: "[ x ] [ budget ] -> [ interval ]",
    },
    // ==================================================================
    // SERIAL
    // ==================================================================
    ModuleWordDoc {
        module: "SERIAL",
        word: "LIST-PORTS",
        summary: "Ask the host to enumerate available serial ports.",
        role: "Effectful host-discovery operator.",
        stack_effect: "no values popped or pushed",
    },
    ModuleWordDoc {
        module: "SERIAL",
        word: "OPEN",
        summary: "Open a serial port by id; leaves the port handle on the stack.",
        role: "Effectful host-resource acquisition.",
        stack_effect: "[ id ] -> [ handle ]",
    },
    ModuleWordDoc {
        module: "SERIAL",
        word: "CONFIGURE",
        summary: "Set the baud rate of an open serial port.",
        role: "Effectful host-resource configuration.",
        stack_effect: "[ handle ] [ baud ] -> [ handle ]",
    },
    ModuleWordDoc {
        module: "SERIAL",
        word: "WRITE",
        summary: "Write a byte vector to an open serial port.",
        role: "Effectful host-resource egress.",
        stack_effect: "[ handle ] [ bytes ] -> [ handle ]",
    },
    ModuleWordDoc {
        module: "SERIAL",
        word: "READ",
        summary: "Drain received bytes from an open serial port; Bubble/NIL when none.",
        role: "Observable host-resource ingress that projects empty reads onto NIL.",
        stack_effect: "[ handle ] -> [ bytes | NIL ]",
    },
    ModuleWordDoc {
        module: "SERIAL",
        word: "FLUSH",
        summary: "Flush the outgoing buffer of an open serial port.",
        role: "Effectful host-resource synchronisation.",
        stack_effect: "[ handle ] -> [ handle ]",
    },
    ModuleWordDoc {
        module: "SERIAL",
        word: "CLOSE",
        summary: "Close an open serial port.",
        role: "Effectful host-resource release.",
        stack_effect: "[ handle ] -> no values popped or pushed",
    },
];
