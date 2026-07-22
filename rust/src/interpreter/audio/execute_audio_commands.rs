use super::super::Interpreter;
use super::audio_types::{update_play_mode, PlayMode, WaveformType};
use super::music_group::{explain_value, make_group, operand_children, GroupMode};
use super::music_values::{
    make_duration, make_edo_pitch, make_hz_pitch, make_note, make_rest, make_scope, make_tuning,
    parse_ratio, record_kind, tuning_components, DURATION_KIND, PITCH_KIND, TUNING_KIND,
};
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_integer_from_value;
use crate::interpreter::{ConsumptionMode, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::Value;
use num_traits::ToPrimitive;

fn require_audio(interp: &mut Interpreter, word: &str) -> Result<()> {
    interp.require_host_capability(word, crate::interpreter::HostCapability::Audio)
}

pub fn op_seq(interp: &mut Interpreter) -> Result<()> {
    update_play_mode(interp, PlayMode::Sequential);
    Ok(())
}

pub fn op_sim(interp: &mut Interpreter) -> Result<()> {
    update_play_mode(interp, PlayMode::Simultaneous);
    Ok(())
}

fn extract_scalar_from_value(val: &Value) -> Option<Fraction> {
    if let Some(f) = val.as_scalar() {
        return Some(f.clone());
    }
    if val.is_vector() && val.len() == 1 {
        let child = val.child(0)?;
        return extract_scalar_from_value(&child);
    }
    None
}

pub fn op_slot(interp: &mut Interpreter) -> Result<()> {
    require_audio(interp, "MUSIC@SLOT")?;
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let frac = extract_scalar_from_value(&val)
        .ok_or_else(|| AjisaiError::from("SLOT requires a single number"))?;

    let seconds = frac
        .to_f64()
        .ok_or_else(|| AjisaiError::from("SLOT value too large"))?;

    if seconds <= 0.0 {
        return Err(AjisaiError::from("SLOT duration must be positive"));
    }

    if seconds < 0.01 {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@SLOT duration {}s is very short\n",
            seconds
        ));
    }
    if seconds > 10.0 {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@SLOT duration {}s is very long\n",
            seconds
        ));
    }

    let payload = format!("{{\"slot_duration\":{}}}", seconds);
    interp.emit_host_effect(crate::interpreter::HostEffect::Config(payload.clone()));
    interp
        .output_buffer
        .push_str(&format!("CONFIG:{}\n", payload));

    Ok(())
}

pub fn op_gain(interp: &mut Interpreter) -> Result<()> {
    require_audio(interp, "MUSIC@GAIN")?;
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let frac = extract_scalar_from_value(&val)
        .ok_or_else(|| AjisaiError::from("GAIN requires a number"))?;

    let gain = frac
        .to_f64()
        .ok_or_else(|| AjisaiError::from("GAIN value too large"))?;

    let clamped = gain.clamp(0.0, 1.0);

    if (gain - clamped).abs() > f64::EPSILON {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@GAIN {} clamped to {}\n",
            gain, clamped
        ));
    }

    let payload = format!("{{\"gain\":{}}}", clamped);
    interp.emit_host_effect(crate::interpreter::HostEffect::Effect(payload.clone()));
    interp
        .output_buffer
        .push_str(&format!("EFFECT:{}\n", payload));

    Ok(())
}

pub fn op_gain_reset(interp: &mut Interpreter) -> Result<()> {
    require_audio(interp, "MUSIC@GAIN-RESET")?;
    interp.emit_host_effect(crate::interpreter::HostEffect::Effect(
        "{\"gain\":1.0}".to_string(),
    ));
    interp.output_buffer.push_str("EFFECT:{\"gain\":1.0}\n");
    Ok(())
}

pub fn op_pan(interp: &mut Interpreter) -> Result<()> {
    require_audio(interp, "MUSIC@PAN")?;
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let frac = extract_scalar_from_value(&val)
        .ok_or_else(|| AjisaiError::from("PAN requires a number"))?;

    let pan = frac
        .to_f64()
        .ok_or_else(|| AjisaiError::from("PAN value too large"))?;

    let clamped = pan.clamp(-1.0, 1.0);

    if (pan - clamped).abs() > f64::EPSILON {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@PAN {} clamped to {}\n",
            pan, clamped
        ));
    }

    let payload = format!("{{\"pan\":{}}}", clamped);
    interp.emit_host_effect(crate::interpreter::HostEffect::Effect(payload.clone()));
    interp
        .output_buffer
        .push_str(&format!("EFFECT:{}\n", payload));

    Ok(())
}

pub fn op_pan_reset(interp: &mut Interpreter) -> Result<()> {
    require_audio(interp, "MUSIC@PAN-RESET")?;
    interp.emit_host_effect(crate::interpreter::HostEffect::Effect(
        "{\"pan\":0.0}".to_string(),
    ));
    interp.output_buffer.push_str("EFFECT:{\"pan\":0.0}\n");
    Ok(())
}

pub fn op_fx_reset(interp: &mut Interpreter) -> Result<()> {
    require_audio(interp, "MUSIC@FX-RESET")?;
    interp.emit_host_effect(crate::interpreter::HostEffect::Effect(
        "{\"gain\":1.0,\"pan\":0.0}".to_string(),
    ));
    interp
        .output_buffer
        .push_str("EFFECT:{\"gain\":1.0,\"pan\":0.0}\n");
    Ok(())
}

/// Reject the whole-stack operation target mode for value constructors, which
/// only have a well-defined effect on a fixed number of stack-top operands.
fn reject_stack_mode(interp: &Interpreter, word: &str) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: word.into(),
            mode: "Stack".into(),
        });
    }
    Ok(())
}

/// Clone the top `count` operands (stack order) without consuming them, so a
/// constructor can validate before committing.
fn peek_operands(interp: &Interpreter, count: usize) -> Result<Vec<Value>> {
    let len = interp.stack.len();
    if len < count {
        return Err(AjisaiError::StackUnderflow);
    }
    Ok(interp.stack.as_slice()[len - count..].to_vec())
}

/// Consume the operands (only in `Consume` mode) and push the constructed value.
fn consume_and_push(interp: &mut Interpreter, count: usize, result: Value) {
    if interp.consumption_mode == ConsumptionMode::Consume {
        let len = interp.stack.len();
        let _ = interp.stack.drain(len - count..);
    }
    interp.stack.push(result);
}

fn require_scalar(value: &Value, message: &str) -> Result<Fraction> {
    value
        .as_scalar()
        .cloned()
        .ok_or_else(|| AjisaiError::from(message))
}

fn require_positive_divisions(value: &Value, word: &str) -> Result<i64> {
    let divisions = extract_integer_from_value(value).map_err(|_| {
        AjisaiError::from(format!(
            "MUSIC@{} divisions must be a positive integer",
            word
        ))
    })?;
    if divisions < 1 {
        return Err(AjisaiError::from(format!(
            "MUSIC@{} divisions must be a positive integer",
            word
        )));
    }
    Ok(divisions)
}

/// Build an explicit sequential music group from a vector.
pub fn op_seq_group(interp: &mut Interpreter) -> Result<()> {
    build_group(interp, GroupMode::Sequential, "generic", "SEQ-GROUP")
}

/// Build an explicit simultaneous music group from a vector.
pub fn op_sim_group(interp: &mut Interpreter) -> Result<()> {
    build_group(interp, GroupMode::Simultaneous, "generic", "SIM-GROUP")
}

/// Build an explicit chord group (simultaneous playback) from a vector.
pub fn op_chord(interp: &mut Interpreter) -> Result<()> {
    build_group(interp, GroupMode::Chord, "chord", "CHORD")
}

/// Build a music group with the role of a single melodic voice.
pub fn op_voice(interp: &mut Interpreter) -> Result<()> {
    build_group(interp, GroupMode::Sequential, "voice", "VOICE")
}

/// Build a music group with the role of an instrument track.
pub fn op_track(interp: &mut Interpreter) -> Result<()> {
    build_group(interp, GroupMode::Sequential, "track", "TRACK")
}

/// Build a music group with the role of a measure (bar).
pub fn op_measure(interp: &mut Interpreter) -> Result<()> {
    build_group(interp, GroupMode::Sequential, "measure", "MEASURE")
}

/// Build a music group with the role of a phrase.
pub fn op_phrase(interp: &mut Interpreter) -> Result<()> {
    build_group(interp, GroupMode::Sequential, "phrase", "PHRASE")
}

fn build_group(interp: &mut Interpreter, mode: GroupMode, role: &str, word: &str) -> Result<()> {
    reject_stack_mode(interp, word)?;
    let operands = peek_operands(interp, 1)?;
    let children = operand_children(&operands[0], word)?;
    let provenance = format!("explicit:MUSIC@{}", word);
    consume_and_push(interp, 1, make_group(mode, role, &provenance, children));
    Ok(())
}

/// Build a `music.pitch` from a direct frequency in Hz. The frequency is kept
/// as an exact rational, so just-intonation ratios survive losslessly.
pub fn op_hz(interp: &mut Interpreter) -> Result<()> {
    reject_stack_mode(interp, "HZ")?;
    let operands = peek_operands(interp, 1)?;

    let hz = require_scalar(&operands[0], "MUSIC@HZ requires a number (frequency in Hz)")?;
    let approx = hz
        .to_f64()
        .ok_or_else(|| AjisaiError::from("MUSIC@HZ frequency is too large"))?;
    if approx < 0.0 {
        return Err(AjisaiError::from("MUSIC@HZ frequency must be non-negative"));
    }

    consume_and_push(interp, 1, make_hz_pitch(hz, "explicit:MUSIC@HZ"));
    Ok(())
}

/// Build a `music.duration` from a number of seconds.
pub fn op_dur(interp: &mut Interpreter) -> Result<()> {
    reject_stack_mode(interp, "DUR")?;
    let operands = peek_operands(interp, 1)?;

    let seconds = require_scalar(
        &operands[0],
        "MUSIC@DUR requires a number (duration in seconds)",
    )?;
    let approx = seconds
        .to_f64()
        .ok_or_else(|| AjisaiError::from("MUSIC@DUR duration is too large"))?;
    if approx <= 0.0 {
        return Err(AjisaiError::from("MUSIC@DUR duration must be positive"));
    }

    consume_and_push(interp, 1, make_duration(seconds, "explicit:MUSIC@DUR"));
    Ok(())
}

/// Combine a `music.pitch` and a `music.duration` into a `music.note`.
pub fn op_note(interp: &mut Interpreter) -> Result<()> {
    reject_stack_mode(interp, "NOTE")?;
    let operands = peek_operands(interp, 2)?;

    if record_kind(&operands[0]).as_deref() != Some(PITCH_KIND) {
        return Err(AjisaiError::from(
            "MUSIC@NOTE expects a music.pitch (use MUSIC@HZ or MUSIC@STEP) as the first operand",
        ));
    }
    if record_kind(&operands[1]).as_deref() != Some(DURATION_KIND) {
        return Err(AjisaiError::from(
            "MUSIC@NOTE expects a music.duration (use MUSIC@DUR) as the second operand",
        ));
    }

    let note = make_note(
        operands[0].clone(),
        operands[1].clone(),
        "explicit:MUSIC@NOTE",
    );
    consume_and_push(interp, 2, note);
    Ok(())
}

/// Build a `music.rest` from a `music.duration`.
pub fn op_rest(interp: &mut Interpreter) -> Result<()> {
    reject_stack_mode(interp, "REST")?;
    let operands = peek_operands(interp, 1)?;

    if record_kind(&operands[0]).as_deref() != Some(DURATION_KIND) {
        return Err(AjisaiError::from(
            "MUSIC@REST expects a music.duration (use MUSIC@DUR)",
        ));
    }

    let rest = make_rest(operands[0].clone(), "explicit:MUSIC@REST");
    consume_and_push(interp, 1, rest);
    Ok(())
}

/// Build a `music.tuning` for equal division of the octave (EDO).
pub fn op_edo(interp: &mut Interpreter) -> Result<()> {
    reject_stack_mode(interp, "EDO")?;
    let operands = peek_operands(interp, 2)?;

    let reference = require_scalar(
        &operands[0],
        "MUSIC@EDO requires a reference frequency as the first operand",
    )?;
    let reference_hz = reference
        .to_f64()
        .ok_or_else(|| AjisaiError::from("MUSIC@EDO reference frequency is too large"))?;
    if reference_hz <= 0.0 {
        return Err(AjisaiError::from(
            "MUSIC@EDO reference frequency must be positive",
        ));
    }

    let divisions = require_positive_divisions(&operands[1], "EDO")?;

    let tuning = make_tuning(
        reference,
        (Fraction::from(2), Fraction::from(1)),
        divisions,
        "explicit:MUSIC@EDO",
    );
    consume_and_push(interp, 2, tuning);
    Ok(())
}

/// Build a `music.tuning` for equal division of an arbitrary ratio (EDR),
/// supporting non-octave tunings such as the Bohlen-Pierce tritave (3/1).
pub fn op_edr(interp: &mut Interpreter) -> Result<()> {
    reject_stack_mode(interp, "EDR")?;
    let operands = peek_operands(interp, 3)?;

    let reference = require_scalar(
        &operands[0],
        "MUSIC@EDR requires a reference frequency as the first operand",
    )?;
    let reference_hz = reference
        .to_f64()
        .ok_or_else(|| AjisaiError::from("MUSIC@EDR reference frequency is too large"))?;
    if reference_hz <= 0.0 {
        return Err(AjisaiError::from(
            "MUSIC@EDR reference frequency must be positive",
        ));
    }

    let (num, den) = parse_ratio(&operands[1]).ok_or_else(|| {
        AjisaiError::from("MUSIC@EDR equave must be a ratio: a number or a [ num den ] vector")
    })?;
    let num_f = num
        .to_f64()
        .ok_or_else(|| AjisaiError::from("MUSIC@EDR equave is too large"))?;
    let den_f = den
        .to_f64()
        .ok_or_else(|| AjisaiError::from("MUSIC@EDR equave is too large"))?;
    if num_f <= 0.0 || den_f <= 0.0 {
        return Err(AjisaiError::from(
            "MUSIC@EDR equave components must be positive",
        ));
    }

    let divisions = require_positive_divisions(&operands[2], "EDR")?;

    let tuning = make_tuning(reference, (num, den), divisions, "explicit:MUSIC@EDR");
    consume_and_push(interp, 3, tuning);
    Ok(())
}

/// Resolve a step within a `music.tuning` into a `music.pitch`.
pub fn op_step(interp: &mut Interpreter) -> Result<()> {
    reject_stack_mode(interp, "STEP")?;
    let operands = peek_operands(interp, 2)?;

    let (reference, equave, divisions) = tuning_components(&operands[0]).ok_or_else(|| {
        AjisaiError::from(
            "MUSIC@STEP expects a music.tuning (use MUSIC@EDO or MUSIC@EDR) as the first operand",
        )
    })?;

    let step = extract_integer_from_value(&operands[1])
        .map_err(|_| AjisaiError::from("MUSIC@STEP step index must be an integer"))?;

    let pitch = make_edo_pitch(reference, equave, divisions, step, "explicit:MUSIC@STEP");
    consume_and_push(interp, 2, pitch);
    Ok(())
}

/// Bind a tuning over a body of musical content. Inside the resulting scope
/// bare integers are interpreted as steps of the tuning at MUSIC@PLAY time.
pub fn op_with_tuning(interp: &mut Interpreter) -> Result<()> {
    reject_stack_mode(interp, "WITH-TUNING")?;
    let operands = peek_operands(interp, 2)?;

    if record_kind(&operands[0]).as_deref() != Some(TUNING_KIND) {
        return Err(AjisaiError::from(
            "MUSIC@WITH-TUNING expects a music.tuning (use MUSIC@EDO or MUSIC@EDR) as the first operand",
        ));
    }
    if operands[1].is_nil() {
        return Err(AjisaiError::from(
            "MUSIC@WITH-TUNING requires a non-empty body as the second operand",
        ));
    }

    let scope = make_scope(
        operands[0].clone(),
        operands[1].clone(),
        "explicit:MUSIC@WITH-TUNING",
    );
    consume_and_push(interp, 2, scope);
    Ok(())
}

/// Explain how a value would be interpreted by MUSIC@PLAY. Diagnostic only:
/// the inspected value is always left on the stack.
pub fn op_explain(interp: &mut Interpreter) -> Result<()> {
    reject_stack_mode(interp, "EXPLAIN")?;
    let operands = peek_operands(interp, 1)?;

    let explanation = explain_value(&operands[0]);
    if !interp.output_buffer.is_empty() && !interp.output_buffer.ends_with('\n') {
        interp.output_buffer.push('\n');
    }
    interp.output_buffer.push_str(&explanation);
    interp.output_buffer.push('\n');

    Ok(())
}

pub fn op_adsr(interp: &mut Interpreter) -> Result<()> {
    let params = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let target = interp.stack.pop().ok_or_else(|| {
        interp.stack.push(params.clone());
        AjisaiError::StackUnderflow
    })?;

    if !params.is_vector() {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from("ADSR parameters must be a vector"));
    }

    if params.len() != 4 {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from(
            "ADSR requires exactly 4 values: [attack decay sustain release]",
        ));
    }

    if !target.is_vector() {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from("ADSR target must be a vector"));
    }

    let param_at = |i: usize, label: &str| -> Result<f64> {
        params
            .child(i)
            .and_then(|v| v.as_scalar().and_then(|f| f.to_f64()))
            .ok_or_else(|| AjisaiError::from(format!("ADSR {} must be a number", label)))
    };
    let attack = param_at(0, "attack")?;
    let decay = param_at(1, "decay")?;
    let sustain = param_at(2, "sustain")?;
    let release = param_at(3, "release")?;

    if attack < 0.0 || decay < 0.0 || release < 0.0 {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from("ADSR times must be non-negative"));
    }
    if !(0.0..=1.0).contains(&sustain) {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from(
            "ADSR sustain must be between 0.0 and 1.0",
        ));
    }

    interp.stack.push(target);
    Ok(())
}

pub fn op_sine(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Sine)
}

pub fn op_square(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Square)
}

pub fn op_saw(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Sawtooth)
}

pub fn op_tri(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Triangle)
}

fn apply_waveform(interp: &mut Interpreter, _waveform: WaveformType) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if !val.is_vector() {
        return Err(AjisaiError::from("Waveform word requires a vector"));
    }

    interp.stack.push(val);
    Ok(())
}
