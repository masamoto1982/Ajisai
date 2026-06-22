//! Layer 0 semantic music values for the MUSIC module.
//!
//! These records carry exact, explainable musical meaning so that AI tooling
//! can reason about pitch and rhythm instead of guessing from raw numbers:
//!
//!   * `music.pitch`    - a frequency. Either a direct/just-intonation Hz value
//!                        (held as an exact rational) or an equal-division step
//!                        kept symbolically until the `MUSIC@PLAY` boundary.
//!   * `music.duration` - a duration in seconds (exact rational).
//!   * `music.note`     - a pitch paired with a duration.
//!   * `music.rest`     - a silent duration.
//!   * `music.tuning`   - an equal-division tuning (EDO / EDR).
//!
//! Equal-division tunings stay symbolic (`reference_hz`, `equave`, `divisions`,
//! `step`); the irrational `f64` approximation happens only when a structure is
//! lowered to `AudioStructure` at playback time.

use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) const PITCH_KIND: &str = "music.pitch";
pub(crate) const DURATION_KIND: &str = "music.duration";
pub(crate) const NOTE_KIND: &str = "music.note";
pub(crate) const REST_KIND: &str = "music.rest";
pub(crate) const TUNING_KIND: &str = "music.tuning";
pub(crate) const SCOPE_KIND: &str = "music.scope";

fn plain_vector(children: Vec<Value>) -> Value {
    Value {
        data: ValueData::Vector(Arc::new(children)),
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

/// Build a record value from ordered key/value fields.
pub(crate) fn make_record(fields: Vec<(&str, Value)>) -> Value {
    let mut pairs = Vec::with_capacity(fields.len());
    let mut index = HashMap::with_capacity(fields.len());
    for (i, (key, value)) in fields.into_iter().enumerate() {
        index.insert(key.to_string(), i);
        pairs.push(plain_vector(vec![Value::from_string(key), value]));
    }
    Value {
        data: ValueData::Record {
            pairs: Arc::new(pairs),
            index,
        },
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

/// Borrow a record field's value by key.
pub(crate) fn record_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    if let ValueData::Record { pairs, index } = &value.data {
        let pos = *index.get(key)?;
        if let ValueData::Vector(kv) = &pairs.get(pos)?.data {
            if kv.len() == 2 {
                return Some(&kv[1]);
            }
        }
    }
    None
}

fn record_string(value: &Value, key: &str) -> Option<String> {
    value_as_string(record_field(value, key)?)
}

fn record_scalar(value: &Value, key: &str) -> Option<Fraction> {
    record_field(value, key)?.as_scalar().cloned()
}

/// Return the `kind` discriminant of a music record, if any.
pub(crate) fn record_kind(value: &Value) -> Option<String> {
    record_string(value, "kind")
}

fn ratio_pair(num: Fraction, den: Fraction) -> Value {
    plain_vector(vec![Value::from_fraction(num), Value::from_fraction(den)])
}

fn ratio_components(value: &Value) -> Option<(Fraction, Fraction)> {
    let num = value.child(0)?;
    let den = value.child(1)?;
    Some((num.as_scalar()?.clone(), den.as_scalar()?.clone()))
}

/// Parse a frequency ratio operand: a scalar `n` (meaning `n/1`) or a
/// two-element vector `[ num den ]`.
pub(crate) fn parse_ratio(value: &Value) -> Option<(Fraction, Fraction)> {
    if let Some(f) = value.as_scalar() {
        return Some((f.clone(), Fraction::from(1)));
    }
    if matches!(value.data, ValueData::Vector(_) | ValueData::Tensor { .. }) && value.len() == 2 {
        return ratio_components(value);
    }
    None
}

/// Build a `music.pitch` whose frequency is a direct Hz value.
pub(crate) fn make_hz_pitch(hz: Fraction, provenance: &str) -> Value {
    make_record(vec![
        ("kind", Value::from_string(PITCH_KIND)),
        ("pitch_kind", Value::from_string("hz")),
        ("hz", Value::from_fraction(hz)),
        ("provenance", Value::from_string(provenance)),
    ])
}

/// Build a `music.pitch` defined symbolically as a step within an
/// equal-division tuning.
pub(crate) fn make_edo_pitch(
    reference_hz: Fraction,
    equave: (Fraction, Fraction),
    divisions: i64,
    step: i64,
    provenance: &str,
) -> Value {
    make_record(vec![
        ("kind", Value::from_string(PITCH_KIND)),
        ("pitch_kind", Value::from_string("edo")),
        ("reference_hz", Value::from_fraction(reference_hz)),
        ("equave", ratio_pair(equave.0, equave.1)),
        ("divisions", Value::from_int(divisions)),
        ("step", Value::from_int(step)),
        ("provenance", Value::from_string(provenance)),
    ])
}

/// Build a `music.duration` of the given seconds.
pub(crate) fn make_duration(seconds: Fraction, provenance: &str) -> Value {
    make_record(vec![
        ("kind", Value::from_string(DURATION_KIND)),
        ("seconds", Value::from_fraction(seconds)),
        ("provenance", Value::from_string(provenance)),
    ])
}

/// Build a `music.note` pairing a pitch with a duration.
pub(crate) fn make_note(pitch: Value, duration: Value, provenance: &str) -> Value {
    make_record(vec![
        ("kind", Value::from_string(NOTE_KIND)),
        ("pitch", pitch),
        ("duration", duration),
        ("provenance", Value::from_string(provenance)),
    ])
}

/// Build a `music.rest` of the given duration.
pub(crate) fn make_rest(duration: Value, provenance: &str) -> Value {
    make_record(vec![
        ("kind", Value::from_string(REST_KIND)),
        ("duration", duration),
        ("provenance", Value::from_string(provenance)),
    ])
}

/// Build a `music.tuning` (equal division of `equave` into `divisions` steps).
pub(crate) fn make_tuning(
    reference_hz: Fraction,
    equave: (Fraction, Fraction),
    divisions: i64,
    provenance: &str,
) -> Value {
    make_record(vec![
        ("kind", Value::from_string(TUNING_KIND)),
        ("reference_hz", Value::from_fraction(reference_hz)),
        ("equave", ratio_pair(equave.0, equave.1)),
        ("divisions", Value::from_int(divisions)),
        ("provenance", Value::from_string(provenance)),
    ])
}

/// Build a `music.scope` that binds a tuning over a body of musical content.
pub(crate) fn make_scope(tuning: Value, body: Value, provenance: &str) -> Value {
    make_record(vec![
        ("kind", Value::from_string(SCOPE_KIND)),
        ("scope_kind", Value::from_string("tuning")),
        ("tuning", tuning),
        ("body", body),
        ("provenance", Value::from_string(provenance)),
    ])
}

/// Extract `(reference_hz, equave, divisions)` from a `music.tuning`.
pub(crate) fn tuning_components(value: &Value) -> Option<(Fraction, (Fraction, Fraction), i64)> {
    if record_kind(value).as_deref() != Some(TUNING_KIND) {
        return None;
    }
    let reference = record_scalar(value, "reference_hz")?;
    let equave = ratio_components(record_field(value, "equave")?)?;
    let divisions = record_scalar(value, "divisions")?.to_i64()?;
    Some((reference, equave, divisions))
}

/// Resolve a `music.pitch` to a concrete frequency in Hz.
///
/// This is the boundary where an exact rational or a symbolic equal-division
/// step is approximated to `f64`.
pub(crate) fn resolve_pitch_hz(value: &Value) -> Option<f64> {
    if record_kind(value).as_deref() != Some(PITCH_KIND) {
        return None;
    }
    match record_string(value, "pitch_kind").as_deref() {
        Some("hz") => record_scalar(value, "hz")?.to_f64(),
        Some("edo") => {
            let reference = record_scalar(value, "reference_hz")?.to_f64()?;
            let (num, den) = ratio_components(record_field(value, "equave")?)?;
            let num = num.to_f64()?;
            let den = den.to_f64()?;
            let divisions = record_scalar(value, "divisions")?.to_f64()?;
            let step = record_scalar(value, "step")?.to_f64()?;
            if divisions == 0.0 || den == 0.0 || num <= 0.0 {
                return None;
            }
            Some(reference * (num / den).powf(step / divisions))
        }
        _ => None,
    }
}

/// Resolve a `music.duration` to seconds.
pub(crate) fn resolve_duration_seconds(value: &Value) -> Option<f64> {
    if record_kind(value).as_deref() != Some(DURATION_KIND) {
        return None;
    }
    record_scalar(value, "seconds")?.to_f64()
}

/// Resolve `step` within `tuning` (a `music.tuning`) to a frequency in Hz.
pub(crate) fn resolve_step_hz(tuning: &Value, step: f64) -> Option<f64> {
    let (reference, (num, den), divisions) = tuning_components(tuning)?;
    let reference = reference.to_f64()?;
    let num = num.to_f64()?;
    let den = den.to_f64()?;
    if divisions == 0 || den == 0.0 || num <= 0.0 {
        return None;
    }
    Some(reference * (num / den).powf(step / divisions as f64))
}

/// Produce an `MUSIC@EXPLAIN` description for a Layer 0 value, if applicable.
pub(crate) fn describe_leaf(value: &Value) -> Option<String> {
    match record_kind(value).as_deref()? {
        PITCH_KIND => {
            let hz = resolve_pitch_hz(value)
                .map(|f| format!("{:.3} Hz", f))
                .unwrap_or_else(|| "unresolved".to_string());
            let detail = match record_string(value, "pitch_kind").as_deref() {
                Some("edo") => format!(
                    "equal-division step {} of {} (symbolic until playback)",
                    record_string(value, "step").unwrap_or_default(),
                    record_string(value, "divisions").unwrap_or_default(),
                ),
                _ => "direct/just-intonation frequency (exact rational)".to_string(),
            };
            Some(format!("Pitch: {} - {}.", hz, detail))
        }
        DURATION_KIND => Some(format!(
            "Duration: {} seconds.",
            record_string(value, "seconds").unwrap_or_default()
        )),
        NOTE_KIND => {
            let pitch = record_field(value, "pitch")
                .and_then(resolve_pitch_hz)
                .map(|f| format!("{:.3} Hz", f))
                .unwrap_or_else(|| "unresolved".to_string());
            let dur = record_field(value, "duration")
                .and_then(resolve_duration_seconds)
                .map(|s| format!("{} s", s))
                .unwrap_or_else(|| "unresolved".to_string());
            Some(format!(
                "Note: {} for {}. Playback boundary: converted to AudioStructure::Tone at MUSIC@PLAY.",
                pitch, dur
            ))
        }
        REST_KIND => {
            let dur = record_field(value, "duration")
                .and_then(resolve_duration_seconds)
                .map(|s| format!("{} s", s))
                .unwrap_or_else(|| "unresolved".to_string());
            Some(format!(
                "Rest: {}. Playback boundary: converted to AudioStructure::Rest at MUSIC@PLAY.",
                dur
            ))
        }
        TUNING_KIND => Some(format!(
            "Tuning: equal division into {} steps. Not directly playable - \
             use MUSIC@STEP to obtain a pitch.",
            record_string(value, "divisions").unwrap_or_default()
        )),
        SCOPE_KIND => {
            let divisions = record_field(value, "tuning")
                .and_then(tuning_components)
                .map(|(_, _, d)| d.to_string())
                .unwrap_or_else(|| "?".to_string());
            Some(format!(
                "Tuning scope: bare integers inside are read as steps of a \
                 {}-division tuning (numerator = step, denominator = duration). \
                 Explicit notes and pitches resolve as usual.",
                divisions
            ))
        }
        _ => None,
    }
}
