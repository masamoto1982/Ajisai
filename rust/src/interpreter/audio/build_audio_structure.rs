use super::super::{Interpreter, OperationTargetMode};
use super::audio_types::{
    update_play_mode, AudioStructure, Envelope, PlayCommand, PlayMode, WaveformType,
};
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{is_string_value, value_as_string};
use crate::types::{Value, ValueData};
use num_traits::ToPrimitive;

pub fn op_play(interp: &mut Interpreter) -> Result<()> {
    let mode = super::lookup_play_mode(interp);
    let target = interp.operation_target_mode;

    match target {
        OperationTargetMode::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let structure = build_audio_structure(&val, mode, &mut interp.output_buffer)?;
            if let Some(json) = emit_play_command(&structure, &mut interp.output_buffer) {
                interp
                    .host_effects
                    .push(crate::interpreter::HostEffect::Audio(json));
            }
        }
        OperationTargetMode::Stack => {
            let values: Vec<Value> = interp.stack.drain(..).collect();
            if values.is_empty() {
                return Err(AjisaiError::StackUnderflow);
            }

            let structures: Vec<AudioStructure> = values
                .iter()
                .map(|v| build_audio_structure(v, PlayMode::Sequential, &mut interp.output_buffer))
                .collect::<Result<Vec<_>>>()?;

            let combined = match mode {
                PlayMode::Sequential => AudioStructure::Seq {
                    children: structures,
                    envelope: None,
                    waveform: WaveformType::Sine,
                },
                PlayMode::Simultaneous => AudioStructure::Sim {
                    children: structures,
                    envelope: None,
                    waveform: WaveformType::Sine,
                },
            };

            if let Some(json) = emit_play_command(&combined, &mut interp.output_buffer) {
                interp
                    .host_effects
                    .push(crate::interpreter::HostEffect::Audio(json));
            }
        }
    }

    update_play_mode(interp, PlayMode::Sequential);
    interp.operation_target_mode = OperationTargetMode::StackTop;

    Ok(())
}

pub(crate) fn build_audio_structure(
    value: &Value,
    mode: PlayMode,
    output: &mut String,
) -> Result<AudioStructure> {
    build_audio_structure_ctx(value, mode, output, None)
}

/// `tuning`, when set, marks an active `MUSIC@WITH-TUNING` scope: bare scalars
/// are read as steps of that tuning (numerator = step, denominator = duration)
/// and vectors are treated as musical sequences rather than lyrics text.
fn build_audio_structure_ctx(
    value: &Value,
    mode: PlayMode,
    output: &mut String,
    tuning: Option<&Value>,
) -> Result<AudioStructure> {
    let envelope: Option<Envelope> = None;
    let waveform = WaveformType::default();

    if value.is_nil() {
        return Ok(AudioStructure::Rest { duration: 1.0 });
    }

    if let Some(kind) = super::music_values::record_kind(value) {
        match kind.as_str() {
            super::music_values::NOTE_KIND => {
                return note_to_structure(value, envelope, waveform, output);
            }
            super::music_values::REST_KIND => {
                let duration = super::music_values::record_field(value, "duration")
                    .and_then(super::music_values::resolve_duration_seconds)
                    .ok_or_else(|| AjisaiError::from("Invalid music.rest: missing duration"))?;
                if duration <= 0.0 {
                    return Err(AjisaiError::from("Rest duration must be positive"));
                }
                return Ok(AudioStructure::Rest { duration });
            }
            super::music_values::PITCH_KIND => {
                let frequency = super::music_values::resolve_pitch_hz(value)
                    .ok_or_else(|| AjisaiError::from("Invalid music.pitch"))?;
                return tone_or_rest(frequency, 1.0, envelope, waveform, output);
            }
            super::music_values::SCOPE_KIND => {
                let scope_tuning = super::music_values::record_field(value, "tuning")
                    .ok_or_else(|| AjisaiError::from("Invalid music.scope: missing tuning"))?;
                let body = super::music_values::record_field(value, "body")
                    .ok_or_else(|| AjisaiError::from("Invalid music.scope: missing body"))?;
                return build_audio_structure_ctx(body, mode, output, Some(scope_tuning));
            }
            super::music_values::DURATION_KIND => {
                return Err(AjisaiError::from(
                    "A music.duration is not directly playable; pair it with a pitch via MUSIC@NOTE",
                ));
            }
            super::music_values::TUNING_KIND => {
                return Err(AjisaiError::from(
                    "A music.tuning is not directly playable; obtain a pitch with MUSIC@STEP",
                ));
            }
            _ => {}
        }
    }

    if let Some(group) = super::music_group::parse_group(value) {
        let group_children: Vec<AudioStructure> = group
            .children
            .iter()
            .map(|e| build_audio_structure_ctx(e, PlayMode::Sequential, output, tuning))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|s| !matches!(s, AudioStructure::Seq { children, .. } if children.is_empty()))
            .collect();

        return match group.mode {
            super::music_group::GroupMode::Sequential => Ok(AudioStructure::Seq {
                children: group_children,
                envelope,
                waveform,
            }),
            super::music_group::GroupMode::Simultaneous | super::music_group::GroupMode::Chord => {
                Ok(AudioStructure::Sim {
                    children: group_children,
                    envelope,
                    waveform,
                })
            }
        };
    }

    // Outside a tuning scope a bare vector is treated as lyrics text; inside a
    // scope it is an explicit musical sequence and skips this branch.
    if tuning.is_none() && is_string_value(value) {
        let s = value_as_string(value).unwrap_or_default();
        output.push_str(&s);
        output.push('\n');
        return Ok(AudioStructure::Seq {
            children: vec![],
            envelope: None,
            waveform: WaveformType::Sine,
        });
    }

    if matches!(value.data, ValueData::Vector(_) | ValueData::Tensor { .. }) {
        let len = value.len();
        if len == 0 {
            return Err(AjisaiError::from("Empty vector not allowed"));
        }

        let audio_children: Vec<AudioStructure> = (0..len)
            .filter_map(|i| value.child(i))
            .map(|child| build_audio_structure_ctx(&child, PlayMode::Sequential, output, tuning))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|s| !matches!(s, AudioStructure::Seq { children, .. } if children.is_empty()))
            .collect();

        return match mode {
            PlayMode::Sequential => Ok(AudioStructure::Seq {
                children: audio_children,
                envelope,
                waveform,
            }),
            PlayMode::Simultaneous => Ok(AudioStructure::Sim {
                children: audio_children,
                envelope,
                waveform,
            }),
        };
    }

    if let Some(frac) = value.as_scalar() {
        let dur = frac
            .denominator()
            .to_f64()
            .ok_or_else(|| AjisaiError::from("Duration too large"))?;

        let frequency = if let Some(tuning_value) = tuning {
            let step = frac
                .numerator()
                .to_f64()
                .ok_or_else(|| AjisaiError::from("Step index too large"))?;
            super::music_values::resolve_step_hz(tuning_value, step)
                .ok_or_else(|| AjisaiError::from("Invalid music.tuning in scope"))?
        } else {
            frac.numerator()
                .to_f64()
                .ok_or_else(|| AjisaiError::from("Frequency too large"))?
        };

        return tone_or_rest(frequency, dur, envelope, waveform, output);
    }

    Ok(AudioStructure::Seq {
        children: vec![],
        envelope: None,
        waveform: WaveformType::Sine,
    })
}

fn tone_or_rest(
    frequency: f64,
    duration: f64,
    envelope: Option<Envelope>,
    waveform: WaveformType,
    output: &mut String,
) -> Result<AudioStructure> {
    if duration <= 0.0 {
        return Err(AjisaiError::from("Duration must be positive"));
    }
    if frequency < 0.0 {
        return Err(AjisaiError::from("Frequency must be non-negative"));
    }
    if frequency == 0.0 {
        return Ok(AudioStructure::Rest { duration });
    }
    check_audible_range(frequency, output);
    Ok(AudioStructure::Tone {
        frequency,
        duration,
        envelope,
        waveform,
    })
}

fn note_to_structure(
    note: &Value,
    envelope: Option<Envelope>,
    waveform: WaveformType,
    output: &mut String,
) -> Result<AudioStructure> {
    let pitch = super::music_values::record_field(note, "pitch")
        .ok_or_else(|| AjisaiError::from("Invalid music.note: missing pitch"))?;
    let frequency = super::music_values::resolve_pitch_hz(pitch)
        .ok_or_else(|| AjisaiError::from("Invalid music.note: unresolvable pitch"))?;
    let duration = super::music_values::record_field(note, "duration")
        .and_then(super::music_values::resolve_duration_seconds)
        .ok_or_else(|| AjisaiError::from("Invalid music.note: unresolvable duration"))?;
    tone_or_rest(frequency, duration, envelope, waveform, output)
}

fn check_audible_range(freq: f64, output: &mut String) {
    const MIN_AUDIBLE: f64 = 20.0;
    const MAX_AUDIBLE: f64 = 20000.0;

    if freq < MIN_AUDIBLE {
        output.push_str(&format!(
            "Warning: {}Hz is below audible range (< 20Hz)\n",
            freq
        ));
    } else if freq > MAX_AUDIBLE {
        output.push_str(&format!(
            "Warning: {}Hz is above audible range (> 20kHz)\n",
            freq
        ));
    }
}

/// Emit the `play` command. Writes the legacy `AUDIO:{json}` protocol line into
/// `output` and returns the JSON payload so the caller can also record a
/// structured `HostEffect::Audio` (the conformance observation channel).
fn emit_play_command(structure: &AudioStructure, output: &mut String) -> Option<String> {
    let command = PlayCommand {
        command_type: "play".to_string(),
        structure: structure.clone(),
    };

    match serde_json::to_string(&command) {
        Ok(json) => {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("AUDIO:");
            output.push_str(&json);
            output.push('\n');
            Some(json)
        }
        Err(_) => None,
    }
}
