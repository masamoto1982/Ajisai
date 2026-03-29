// rust/src/interpreter/audio/build-audio-structure.rs
//
// 【責務】
// 値からAudioStructureを構築し、PLAYコマンドを実行する。

use super::audio_types::{
    update_play_mode, AudioStructure, Envelope, PlayCommand, PlayMode, WaveformType,
};
use super::super::{Interpreter, OperationTargetMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{is_string_value, is_vector_value, value_as_string};
use crate::types::Value;
use num_traits::ToPrimitive;

// ============================================================================
// MUSIC@PLAY ワード実装
// ============================================================================

/// MUSIC@PLAY ワードのエントリポイント
pub fn op_play(interp: &mut Interpreter) -> Result<()> {
    let mode = super::lookup_play_mode(interp);
    let target = interp.operation_target_mode;

    match target {
        OperationTargetMode::StackTop => {
            // スタックトップのベクタを処理
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let structure = build_audio_structure(&val, mode, &mut interp.output_buffer)?;
            emit_play_command(&structure, &mut interp.output_buffer);
        }
        OperationTargetMode::Stack => {
            // スタック全体の各要素を処理
            let values: Vec<Value> = interp.stack.drain(..).collect();
            if values.is_empty() {
                return Err(AjisaiError::StackUnderflow);
            }

            // 各値を順次再生として構築
            let structures: Vec<AudioStructure> = values
                .iter()
                .map(|v| build_audio_structure(v, PlayMode::Sequential, &mut interp.output_buffer))
                .collect::<Result<Vec<_>>>()?;

            // モードに応じて結合
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

            emit_play_command(&combined, &mut interp.output_buffer);
        }
    }

    // リセット
    update_play_mode(interp, PlayMode::Sequential);
    interp.operation_target_mode = OperationTargetMode::StackTop;

    Ok(())
}

// ============================================================================
// build_audio_structure 関数
// ============================================================================

/// 値からAudioStructureを構築
pub(crate) fn build_audio_structure(
    value: &Value,
    mode: PlayMode,
    output: &mut String,
) -> Result<AudioStructure> {
    // TODO: AudioHint metadata will be read from SemanticRegistry
    // For now, use defaults since ext is no longer on Value
    let envelope: Option<Envelope> = None;
    let waveform = WaveformType::default();
    let is_chord = false;

    // NIL判定
    if value.is_nil() {
        return Ok(AudioStructure::Rest { duration: 1.0 });
    }

    // 文字列判定（歌詞: Outputに出力、時間消費なし）
    if is_string_value(value) {
        let s = value_as_string(value).unwrap_or_default();
        output.push_str(&s);
        output.push('\n');
        return Ok(AudioStructure::Seq {
            children: vec![],
            envelope: None,
            waveform: WaveformType::Sine,
        });
    }

    // ベクタ判定
    if is_vector_value(value) {
        if let Some(children) = value.as_vector() {
            if children.is_empty() {
                return Err(AjisaiError::from("Empty vector not allowed"));
            }

            let audio_children: Vec<AudioStructure> = children
                .iter()
                .map(|e| build_audio_structure(e, PlayMode::Sequential, output))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .filter(
                    |s| !matches!(s, AudioStructure::Seq { children, .. } if children.is_empty()),
                )
                .collect();

            // MUSIC@CHORDフラグがあれば同時再生、なければモードに従う
            let effective_mode = if is_chord {
                PlayMode::Simultaneous
            } else {
                mode
            };

            return match effective_mode {
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
    }

    // 数値判定（単一スカラー）
    if let Some(frac) = value.as_scalar() {
        let freq = frac
            .numerator()
            .to_f64()
            .ok_or_else(|| AjisaiError::from("Frequency too large"))?;
        let dur = frac
            .denominator()
            .to_f64()
            .ok_or_else(|| AjisaiError::from("Duration too large"))?;

        if dur <= 0.0 {
            return Err(AjisaiError::from("Duration must be positive"));
        }

        if freq == 0.0 {
            return Ok(AudioStructure::Rest { duration: dur });
        } else if freq > 0.0 {
            check_audible_range(freq, output);
            return Ok(AudioStructure::Tone {
                frequency: freq,
                duration: dur,
                envelope,
                waveform,
            });
        } else {
            return Err(AjisaiError::from("Frequency must be non-negative"));
        }
    }

    // Boolean等は無視（空のSeqとして返す）
    Ok(AudioStructure::Seq {
        children: vec![],
        envelope: None,
        waveform: WaveformType::Sine,
    })
}

// ============================================================================
// ヘルパー関数
// ============================================================================

/// 可聴域（20Hz〜20,000Hz）のチェックと警告出力
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

/// PlayCommand を JSON として output_buffer に出力
fn emit_play_command(structure: &AudioStructure, output: &mut String) {
    let command = PlayCommand {
        command_type: "play".to_string(),
        structure: structure.clone(),
    };

    if let Ok(json) = serde_json::to_string(&command) {
        // AUDIOコマンドが必ず行頭から始まるようにする
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str("AUDIO:");
        output.push_str(&json);
        output.push('\n');
    }
}
