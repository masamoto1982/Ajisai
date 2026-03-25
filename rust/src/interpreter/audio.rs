// rust/src/interpreter/audio.rs
//
// 【責務】
// 音楽DSLの実装。SEQ（順次再生）とSIM（同時再生）の2つの操作で音楽を表現する。
// 分数システム（周波数/音長）を活用し、ベクタの要素を音声として再生する。
//
// 【分数の解釈】
// - n/d = nHz を dスロット再生
// - n = n/1 の省略（nHz を 1スロット）
// - 0/d = dスロット休符
//
// 【値の種類と動作】
// - 整数 n: nHz を 1スロット再生
// - 分数 n/d: nHz を dスロット再生
// - 0/d: dスロット休符
// - NIL: 1スロット休符
// - 文字列: Outputに出力（歌詞）、時間消費なし
//
// 【オペレーションターゲット】
// - . MUSIC@PLAY（デフォルト）: スタックトップを再生
// - .. MUSIC@PLAY: スタック全体を再生（マルチトラック）

use super::Interpreter;
use super::OperationTargetMode;
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{is_string_value, is_vector_value, value_as_string};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData, ValueExt};
use num_traits::ToPrimitive;
use serde::Serialize;

// ============================================================================
// Audio types (module-local, decoupled from core types)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WaveformType {
    #[default]
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Envelope {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.0,
            sustain: 1.0,
            release: 0.01,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioHint {
    pub chord: bool,
    pub envelope: Option<Envelope>,
    pub waveform: WaveformType,
}

impl ValueExt for AudioHint {
    fn clone_box(&self) -> Box<dyn ValueExt> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ============================================================================
// Music module state (stored in interpreter's module_state)
// ============================================================================

pub(crate) struct MusicState {
    pub play_mode: PlayMode,
}

pub(crate) fn lookup_play_mode(interp: &Interpreter) -> PlayMode {
    interp
        .module_state
        .get("MUSIC")
        .and_then(|s| s.downcast_ref::<MusicState>())
        .map(|s| s.play_mode)
        .unwrap_or_default()
}

fn update_play_mode(interp: &mut Interpreter, mode: PlayMode) {
    if let Some(state) = interp.module_state.get_mut("MUSIC") {
        if let Some(ms) = state.downcast_mut::<MusicState>() {
            ms.play_mode = mode;
            return;
        }
    }
    interp.module_state.insert(
        "MUSIC".to_string(),
        Box::new(MusicState { play_mode: mode }),
    );
}

// ============================================================================
// PlayMode - 再生モード
// ============================================================================

/// 再生モード（SEQ/SIM指定用）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PlayMode {
    #[default]
    Sequential, // 順次再生
    Simultaneous, // 同時再生
}

// ============================================================================
// AudioStructure - 音声構造
// ============================================================================

/// 音声構造（再帰的）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum AudioStructure {
    #[serde(rename = "tone")]
    Tone {
        frequency: f64,
        duration: f64, // スロット数
        #[serde(skip_serializing_if = "Option::is_none")]
        envelope: Option<Envelope>,
        #[serde(skip_serializing_if = "is_default_waveform")]
        waveform: WaveformType,
    },
    #[serde(rename = "rest")]
    Rest { duration: f64 },
    #[serde(rename = "seq")]
    Seq {
        children: Vec<AudioStructure>,
        #[serde(skip_serializing_if = "Option::is_none")]
        envelope: Option<Envelope>,
        #[serde(skip_serializing_if = "is_default_waveform")]
        waveform: WaveformType,
    },
    #[serde(rename = "sim")]
    Sim {
        children: Vec<AudioStructure>,
        #[serde(skip_serializing_if = "Option::is_none")]
        envelope: Option<Envelope>,
        #[serde(skip_serializing_if = "is_default_waveform")]
        waveform: WaveformType,
    },
}

/// デフォルト波形（Sine）かどうかを判定（シリアライズ用）
fn is_default_waveform(wf: &WaveformType) -> bool {
    *wf == WaveformType::Sine
}

// ============================================================================
// PlayCommand - JSON出力用
// ============================================================================

#[derive(Debug, Serialize)]
struct PlayCommand {
    #[serde(rename = "type")]
    command_type: String,
    structure: AudioStructure,
}

// ============================================================================
// MUSIC@SEQ ワード実装
// ============================================================================

/// MUSIC@SEQ ワード - 順次再生モードを設定
pub fn op_seq(interp: &mut Interpreter) -> Result<()> {
    update_play_mode(interp, PlayMode::Sequential);
    Ok(())
}

// ============================================================================
// MUSIC@SIM ワード実装
// ============================================================================

/// MUSIC@SIM ワード - 同時再生モードを設定
pub fn op_sim(interp: &mut Interpreter) -> Result<()> {
    update_play_mode(interp, PlayMode::Simultaneous);
    Ok(())
}

// ============================================================================
// MUSIC@SLOT ワード実装
// ============================================================================

/// ヘルパー関数: 値からスカラー値を取得（単一要素ベクタからも再帰的に取得）
fn extract_scalar_from_value(val: &Value) -> Option<Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Some(f.clone()),
        ValueData::Vector(children) if children.len() == 1 => extract_scalar_from_value(&children[0]),
        _ => None,
    }
}

/// MUSIC@SLOT ワード - スロットデュレーションを設定
/// スタック: [ seconds ] --
/// 1スロットあたりの秒数を設定する
pub fn op_slot(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // スカラー値を取得（単一要素ベクタからも取得可能）
    let frac =
        extract_scalar_from_value(&val).ok_or_else(|| AjisaiError::from("SLOT requires a single number"))?;

    // f64に変換
    let seconds = frac
        .to_f64()
        .ok_or_else(|| AjisaiError::from("SLOT value too large"))?;

    // バリデーション
    if seconds <= 0.0 {
        return Err(AjisaiError::from("SLOT duration must be positive"));
    }

    // 極端に小さい/大きい値の警告
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

    // CONFIG コマンドを出力
    interp
        .output_buffer
        .push_str(&format!("CONFIG:{{\"slot_duration\":{}}}\n", seconds));

    Ok(())
}

// ============================================================================
// MUSIC@GAIN ワード実装
// ============================================================================

/// MUSIC@GAIN ワード - 音量を設定
/// スタック: [ value ] --
/// 0.0〜1.0の範囲で音量を設定（範囲外はクランプ）
pub fn op_gain(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let frac = extract_scalar_from_value(&val).ok_or_else(|| AjisaiError::from("GAIN requires a number"))?;

    let gain = frac
        .to_f64()
        .ok_or_else(|| AjisaiError::from("GAIN value too large"))?;

    // 0.0〜1.0にクランプ
    let clamped = gain.clamp(0.0, 1.0);

    if (gain - clamped).abs() > f64::EPSILON {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@GAIN {} clamped to {}\n",
            gain, clamped
        ));
    }

    // EFFECTコマンドを出力
    interp
        .output_buffer
        .push_str(&format!("EFFECT:{{\"gain\":{}}}\n", clamped));

    Ok(())
}

/// MUSIC@GAIN-RESET ワード - 音量をデフォルト（1.0）に戻す
pub fn op_gain_reset(interp: &mut Interpreter) -> Result<()> {
    interp.output_buffer.push_str("EFFECT:{\"gain\":1.0}\n");
    Ok(())
}

// ============================================================================
// MUSIC@PAN ワード実装
// ============================================================================

/// MUSIC@PAN ワード - 定位を設定
/// スタック: [ value ] --
/// -1.0（左）〜1.0（右）の範囲で定位を設定（範囲外はクランプ）
pub fn op_pan(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let frac = extract_scalar_from_value(&val).ok_or_else(|| AjisaiError::from("PAN requires a number"))?;

    let pan = frac
        .to_f64()
        .ok_or_else(|| AjisaiError::from("PAN value too large"))?;

    // -1.0〜1.0にクランプ
    let clamped = pan.clamp(-1.0, 1.0);

    if (pan - clamped).abs() > f64::EPSILON {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@PAN {} clamped to {}\n",
            pan, clamped
        ));
    }

    // EFFECTコマンドを出力
    interp
        .output_buffer
        .push_str(&format!("EFFECT:{{\"pan\":{}}}\n", clamped));

    Ok(())
}

/// MUSIC@PAN-RESET ワード - 定位をデフォルト（0.0=中央）に戻す
pub fn op_pan_reset(interp: &mut Interpreter) -> Result<()> {
    interp.output_buffer.push_str("EFFECT:{\"pan\":0.0}\n");
    Ok(())
}

/// MUSIC@FX-RESET ワード - 全エフェクトをデフォルトに戻す
pub fn op_fx_reset(interp: &mut Interpreter) -> Result<()> {
    interp
        .output_buffer
        .push_str("EFFECT:{\"gain\":1.0,\"pan\":0.0}\n");
    Ok(())
}

// ============================================================================
// MUSIC@CHORD ワード実装
// ============================================================================

/// MUSIC@CHORD ワード - ベクタを同時再生（和音）としてマーク
pub fn op_chord(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // ベクタでなければエラー
    if !val.is_vector() {
        return Err(AjisaiError::from("CHORD requires a vector"));
    }

    // TODO: AudioHint metadata will be managed by SemanticRegistry
    // For now, chord marking is a no-op on the value itself
    interp.stack.push(val);
    Ok(())
}

// ============================================================================
// MUSIC@ADSR ワード実装
// ============================================================================

/// MUSIC@ADSR ワード - MUSIC@ADSRエンベロープを設定
/// スタック: [ target ] [ attack decay sustain release ] -- [ target ]'
/// 対象ベクタにADSRエンベロープを適用
pub fn op_adsr(interp: &mut Interpreter) -> Result<()> {
    // MUSIC@ADSRパラメータを取得
    let params = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 対象ベクタを取得
    let target = interp.stack.pop().ok_or_else(|| {
        // パラメータをスタックに戻してからエラー
        interp.stack.push(params.clone());
        AjisaiError::StackUnderflow
    })?;

    // パラメータがベクタでなければエラー
    let param_children = match &params.data {
        ValueData::Vector(v) => v,
        _ => {
            interp.stack.push(target);
            interp.stack.push(params);
            return Err(AjisaiError::from("ADSR parameters must be a vector"));
        }
    };

    // 4要素でなければエラー
    if param_children.len() != 4 {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from(
            "ADSR requires exactly 4 values: [attack decay sustain release]",
        ));
    }

    // 対象がベクタでなければエラー
    if !target.is_vector() {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from("ADSR target must be a vector"));
    }

    // 各値を取得
    let attack = param_children[0]
        .as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR attack must be a number"))?;
    let decay = param_children[1]
        .as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR decay must be a number"))?;
    let sustain = param_children[2]
        .as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR sustain must be a number"))?;
    let release = param_children[3]
        .as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR release must be a number"))?;

    // 値の検証
    if attack < 0.0 || decay < 0.0 || release < 0.0 {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from("ADSR times must be non-negative"));
    }
    if sustain < 0.0 || sustain > 1.0 {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from(
            "ADSR sustain must be between 0.0 and 1.0",
        ));
    }

    // TODO: AudioHint metadata (envelope) will be managed by SemanticRegistry
    // For now, ADSR setting is a no-op on the value itself
    interp.stack.push(target);
    Ok(())
}

// ============================================================================
// 波形ワード実装
// ============================================================================

/// MUSIC@SINE ワード - 正弦波を設定
pub fn op_sine(interp: &mut Interpreter) -> Result<()> {
    update_waveform_in_hint(interp, WaveformType::Sine)
}

/// MUSIC@SQUARE ワード - 矩形波を設定
pub fn op_square(interp: &mut Interpreter) -> Result<()> {
    update_waveform_in_hint(interp, WaveformType::Square)
}

/// MUSIC@SAW ワード - のこぎり波を設定
pub fn op_saw(interp: &mut Interpreter) -> Result<()> {
    update_waveform_in_hint(interp, WaveformType::Sawtooth)
}

/// MUSIC@TRI ワード - 三角波を設定
pub fn op_tri(interp: &mut Interpreter) -> Result<()> {
    update_waveform_in_hint(interp, WaveformType::Triangle)
}

/// 波形を設定するヘルパー関数
fn update_waveform_in_hint(interp: &mut Interpreter, _waveform: WaveformType) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // ベクタでなければエラー
    if !val.is_vector() {
        return Err(AjisaiError::from("Waveform word requires a vector"));
    }

    // TODO: AudioHint metadata (waveform) will be managed by SemanticRegistry
    // For now, waveform setting is a no-op on the value itself
    interp.stack.push(val);
    Ok(())
}

// ============================================================================
// MUSIC@PLAY ワード実装
// ============================================================================

/// MUSIC@PLAY ワードのエントリポイント
pub fn op_play(interp: &mut Interpreter) -> Result<()> {
    let mode = lookup_play_mode(interp);
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
fn build_audio_structure(
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

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::fraction::Fraction;
    use num_bigint::BigInt;

    fn create_number(n: i64) -> Value {
        Value::from_fraction(Fraction::new(BigInt::from(n), BigInt::from(1)))
    }

    fn create_fraction(num: i64, den: i64) -> Value {
        Value::from_fraction(Fraction::create_unreduced(
            BigInt::from(num),
            BigInt::from(den),
        ))
    }

    fn create_nil() -> Value {
        Value::nil()
    }

    fn create_vector(elements: Vec<Value>) -> Value {
        Value::from_vector(elements)
    }

    #[test]
    fn test_tone_from_integer() {
        // 440 → 440Hz, 1スロット
        let val = create_number(440);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Tone {
                frequency,
                duration,
                ..
            } => {
                assert_eq!(frequency, 440.0);
                assert_eq!(duration, 1.0);
            }
            _ => panic!("Expected Tone"),
        }
    }

    #[test]
    fn test_tone_from_fraction() {
        // 440/3 → 440Hz, 3スロット (440 and 3 are coprime, no normalization)
        let val = create_fraction(440, 3);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Tone {
                frequency,
                duration,
                ..
            } => {
                assert_eq!(frequency, 440.0);
                assert_eq!(duration, 3.0);
            }
            _ => panic!("Expected Tone"),
        }
    }

    #[test]
    fn test_tone_from_fraction_unreduced() {
        // 440/2 is preserved as-is (no GCD reduction) for music DSL
        // This becomes 440Hz, 2スロット
        let val = create_fraction(440, 2);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Tone {
                frequency,
                duration,
                ..
            } => {
                assert_eq!(frequency, 440.0);
                assert_eq!(duration, 2.0);
            }
            _ => panic!("Expected Tone"),
        }
    }

    #[test]
    fn test_rest_from_zero() {
        // 0/4 is preserved as-is for music DSL → 4-slot rest
        let val = create_fraction(0, 4);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Rest { duration } => {
                assert_eq!(duration, 4.0);
            }
            _ => panic!("Expected Rest"),
        }
    }

    #[test]
    fn test_rest_from_zero_coprime() {
        // 0/3 is preserved as-is for music DSL → 3-slot rest
        let val = create_fraction(0, 3);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Rest { duration } => {
                assert_eq!(duration, 3.0);
            }
            _ => panic!("Expected Rest"),
        }
    }

    #[test]
    fn test_rest_from_nil() {
        // NIL → 1スロット休符
        let val = create_nil();
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Rest { duration } => {
                assert_eq!(duration, 1.0);
            }
            _ => panic!("Expected Rest"),
        }
    }

    #[test]
    fn test_nil_becomes_rest() {
        // NIL値は休符として処理される
        let val = Value::nil();
        let mut output = String::new();
        let result = build_audio_structure(&val, PlayMode::Sequential, &mut output);

        // NILは休符になる
        match result {
            Ok(AudioStructure::Rest { duration }) => {
                assert_eq!(duration, 1.0, "NIL should become 1-slot rest");
            }
            Ok(other) => panic!("Expected Rest, got {:?}", other),
            Err(e) => panic!("Expected success (Rest), got error: {:?}", e),
        }
    }

    #[test]
    fn test_seq_structure() {
        // Without DisplayHint, is_string_value returns true for all vectors,
        // so a vector of scalars [440, 550] is treated as lyrics (codepoints).
        // build_audio_structure outputs the characters and returns an empty Seq.
        let elements = vec![create_number(440), create_number(550)];
        let val = create_vector(elements);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Seq { children, .. } => {
                assert_eq!(children.len(), 0, "String-treated vector yields empty seq");
            }
            _ => panic!("Expected Seq"),
        }
    }

    #[test]
    fn test_sim_structure() {
        // Without DisplayHint, is_string_value returns true for all vectors,
        // so a vector of scalars is treated as lyrics (codepoints).
        let elements = vec![create_number(440), create_number(550)];
        let val = create_vector(elements);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Simultaneous, &mut output).unwrap();

        // Even with Simultaneous mode, the string check happens first
        // and returns an empty Seq (lyrics path).
        match structure {
            AudioStructure::Seq { children, .. } => {
                assert_eq!(children.len(), 0, "String-treated vector yields empty seq");
            }
            _ => panic!("Expected Seq (string-treated lyrics path)"),
        }
    }

    #[test]
    fn test_negative_frequency_error() {
        let val = create_number(-440);
        let mut output = String::new();
        let result = build_audio_structure(&val, PlayMode::Sequential, &mut output);

        assert!(result.is_err());
    }

    #[test]
    fn test_audible_range_warning_low() {
        let val = create_number(10); // 10Hz - below audible
        let mut output = String::new();
        let _ = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        assert!(
            output.contains("Warning:"),
            "Should warn about inaudible frequency"
        );
        assert!(
            output.contains("below audible range"),
            "Should mention below range"
        );
    }

    #[test]
    fn test_audible_range_warning_high() {
        let val = create_number(25000); // 25kHz - above audible
        let mut output = String::new();
        let _ = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        assert!(
            output.contains("Warning:"),
            "Should warn about inaudible frequency"
        );
        assert!(
            output.contains("above audible range"),
            "Should mention above range"
        );
    }

    #[test]
    fn test_audible_range_no_warning() {
        let val = create_number(440); // 440Hz - audible
        let mut output = String::new();
        let _ = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        assert!(
            !output.contains("Warning:"),
            "Should not warn for audible frequency"
        );
    }

    #[tokio::test]
    async fn test_play_integration() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 440 ] MUSIC@PLAY").await;
        assert!(result.is_ok(), "PLAY should succeed: {:?}", result);

        let output = interp.collect_output();
        // Without DisplayHint, is_string_value treats all vectors as strings,
        // so [440] is treated as lyrics (codepoint Ƹ). AUDIO command is emitted
        // with an empty structure.
        assert!(
            output.contains("AUDIO:"),
            "Output should contain AUDIO command: {}",
            output
        );
        assert!(
            output.contains("\"type\":\"play\""),
            "Should contain play type"
        );
    }

    #[tokio::test]
    async fn test_seq_play_integration() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp
            .execute("[ 440 550 660 ] MUSIC@SEQ MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "SEQ MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("\"type\":\"seq\""),
            "Should contain seq structure"
        );
    }

    #[tokio::test]
    async fn test_sim_play_integration() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp
            .execute("[ 440 550 660 ] MUSIC@SIM MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "SIM MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        // Without DisplayHint, is_string_value treats the vector as lyrics.
        // MUSIC@SIM sets mode but build_audio_structure hits the string path first,
        // producing an empty seq. The outer play command still emits AUDIO.
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    #[tokio::test]
    async fn test_multitrack_play() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp
            .execute("[ 440 550 ] [ 220 275 ] .. MUSIC@SIM MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "Multitrack MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("\"type\":\"sim\""),
            "Should contain sim structure for multitrack"
        );

        // スタックが空であることを確認（両方のベクタが消費されたはず）
        assert!(
            interp.get_stack().is_empty(),
            "Stack should be empty after .. MUSIC@SIM MUSIC@PLAY, but had {} elements",
            interp.get_stack().len()
        );
    }

    #[tokio::test]
    async fn test_multitrack_seq_play() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp
            .execute("[ 440 550 ] [ 220 275 ] .. MUSIC@SEQ MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "Multitrack MUSIC@SEQ MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("\"type\":\"seq\""),
            "Should contain seq structure for multitrack"
        );

        // スタックが空であることを確認
        assert!(
            interp.get_stack().is_empty(),
            "Stack should be empty after .. MUSIC@SEQ MUSIC@PLAY"
        );
    }

    #[tokio::test]
    async fn test_multitrack_play_consumes_all() {
        use crate::interpreter::Interpreter;

        // 3つのトラックをテスト
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp
            .execute("[ 440 ] [ 550 ] [ 660 ] .. MUSIC@SIM MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "3-track MUSIC@PLAY should succeed: {:?}",
            result
        );

        // スタックが完全に空であることを確認
        assert!(
            interp.get_stack().is_empty(),
            "Stack should be completely empty after playing 3 tracks"
        );
    }

    #[tokio::test]
    async fn test_play_with_duration() {
        use crate::interpreter::Interpreter;

        // Use coprime fractions: 440/3 and 660/7 don't normalize.
        // Without DisplayHint, the vector is treated as lyrics by
        // is_string_value, so no tone/duration data is produced.
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 440/3 550/1 660/7 ] MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "PLAY with duration should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    #[tokio::test]
    async fn test_play_with_zero_rest() {
        use crate::interpreter::Interpreter;

        // Without DisplayHint, the vector is treated as lyrics by is_string_value.
        // 0/2 rest and tone data are not produced — just lyrics output.
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 440 0/2 550 ] MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "PLAY with 0/n rest should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    // ============================================================================
    // 新機能テスト: MUSIC@CHORD, MUSIC@ADSR, MUSIC@SINE, MUSIC@SQUARE, MUSIC@SAW, MUSIC@TRI
    // ============================================================================

    #[tokio::test]
    async fn test_chord_marks_as_simultaneous() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // CHORD is now a no-op (AudioHint metadata removed from Value).
        // The vector is also treated as lyrics by is_string_value.
        let result = interp
            .execute("[ 440 550 660 ] MUSIC@CHORD MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "CHORD MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    #[tokio::test]
    async fn test_chord_requires_vector() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("440 MUSIC@CHORD").await;
        assert!(result.is_err(), "CHORD on non-vector should fail");
    }

    #[tokio::test]
    async fn test_adsr_sets_envelope() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // ADSR is now a no-op (AudioHint metadata removed from Value).
        // It validates parameters but doesn't attach envelope data.
        let result = interp
            .execute("[ 440 ] [ 0.05 0.1 0.8 0.2 ] MUSIC@ADSR MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "ADSR MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        // AUDIO command is emitted but without envelope data (ADSR is no-op).
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    #[tokio::test]
    async fn test_adsr_requires_4_elements() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // MUSIC@ADSR needs target + params
        let result = interp.execute("[ 440 ] [ 0.1 0.2 0.3 ] MUSIC@ADSR").await;
        assert!(result.is_err(), "ADSR with 3 elements should fail");
    }

    #[tokio::test]
    async fn test_adsr_sustain_validation() {
        use crate::interpreter::Interpreter;

        // Sustain > 1.0 should fail
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp
            .execute("[ 440 ] [ 0.1 0.1 1.5 0.1 ] MUSIC@ADSR")
            .await;
        assert!(result.is_err(), "ADSR with sustain > 1.0 should fail");
    }

    #[tokio::test]
    async fn test_square_waveform() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // Waveform setting is now a no-op (AudioHint metadata removed from Value).
        let result = interp.execute("[ 440 ] MUSIC@SQUARE MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "SQUARE MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    #[tokio::test]
    async fn test_saw_waveform() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // Waveform setting is now a no-op (AudioHint metadata removed from Value).
        let result = interp.execute("[ 440 ] MUSIC@SAW MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "SAW MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    #[tokio::test]
    async fn test_tri_waveform() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // Waveform setting is now a no-op (AudioHint metadata removed from Value).
        let result = interp.execute("[ 440 ] MUSIC@TRI MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "TRI MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    #[tokio::test]
    async fn test_sine_is_default_not_serialized() {
        use crate::interpreter::Interpreter;

        // MUSIC@SINE is the default, so it shouldn't be serialized
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 440 ] MUSIC@SINE MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "SINE MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        // Default sine should not appear in output (skip_serializing_if)
        assert!(
            !output.contains("\"waveform\":\"sine\""),
            "Sine waveform should not be serialized as it's default"
        );
    }

    #[tokio::test]
    async fn test_combined_chord_adsr_waveform() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // CHORD, ADSR, and waveform settings are all no-ops now
        // (AudioHint metadata removed from Value).
        let result = interp.execute("[ 440 550 660 ] MUSIC@CHORD [ 0.01 0.1 0.7 0.3 ] MUSIC@ADSR MUSIC@SQUARE MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "Combined CHORD ADSR SQUARE PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    // ============================================================================
    // MUSIC@SLOT ワードテスト
    // ============================================================================

    #[tokio::test]
    async fn test_slot_basic() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 0.25 ] MUSIC@SLOT").await;
        assert!(result.is_ok(), "SLOT should succeed: {:?}", result);

        let output = interp.collect_output();
        assert!(output.contains("CONFIG:"), "Should contain CONFIG command");
        assert!(
            output.contains("\"slot_duration\":0.25"),
            "Should set duration to 0.25"
        );
    }

    #[tokio::test]
    async fn test_slot_fraction() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 1/4 ] MUSIC@SLOT").await;
        assert!(
            result.is_ok(),
            "SLOT with fraction should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("\"slot_duration\":0.25"),
            "1/4 should become 0.25"
        );
    }

    #[tokio::test]
    async fn test_slot_negative_error() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // Create negative number via arithmetic: 0 - 0.5 = -0.5
        let result = interp.execute("[ 0 ] [ 0.5 ] - MUSIC@SLOT").await;
        assert!(result.is_err(), "Negative MUSIC@SLOT should fail");
    }

    #[tokio::test]
    async fn test_slot_zero_error() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 0 ] MUSIC@SLOT").await;
        assert!(result.is_err(), "Zero MUSIC@SLOT should fail");
    }

    #[tokio::test]
    async fn test_slot_warning_very_short() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 0.005 ] MUSIC@SLOT").await;
        assert!(
            result.is_ok(),
            "Very short MUSIC@SLOT should succeed with warning"
        );

        let output = interp.collect_output();
        assert!(
            output.contains("Warning:"),
            "Should contain warning for very short duration"
        );
        assert!(
            output.contains("very short"),
            "Warning should mention 'very short'"
        );
    }

    #[tokio::test]
    async fn test_slot_warning_very_long() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 15 ] MUSIC@SLOT").await;
        assert!(
            result.is_ok(),
            "Very long MUSIC@SLOT should succeed with warning"
        );

        let output = interp.collect_output();
        assert!(
            output.contains("Warning:"),
            "Should contain warning for very long duration"
        );
        assert!(
            output.contains("very long"),
            "Warning should mention 'very long'"
        );
    }

    #[tokio::test]
    async fn test_slot_one_second() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 1 ] MUSIC@SLOT").await;
        assert!(result.is_ok(), "1 MUSIC@SLOT should succeed: {:?}", result);

        let output = interp.collect_output();
        assert!(output.contains("CONFIG:"), "Should contain CONFIG command");
        assert!(
            output.contains("\"slot_duration\":1"),
            "Should set duration to 1"
        );
    }

    #[tokio::test]
    async fn test_chord_can_be_defined_and_reused() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // CHORD is now a no-op. Use code block for DEF since vector duality
        // no longer preserves MUSIC@ word names.
        let result = interp
            .execute(": [ 440 550 660 ] MUSIC@CHORD ; 'C_MAJOR' DEF")
            .await;
        assert!(result.is_ok(), "DEF should succeed: {:?}", result);

        // Use it
        let result = interp.execute("C_MAJOR MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "C_MAJOR MUSIC@PLAY should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        // CHORD is no-op and vectors are treated as lyrics, so AUDIO has empty structure
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    // ============================================================================
    // MUSIC@GAIN/PAN ワードテスト
    // ============================================================================

    #[tokio::test]
    async fn test_gain_basic() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("0.5 MUSIC@GAIN").await;
        assert!(result.is_ok(), "GAIN should succeed: {:?}", result);

        let output = interp.collect_output();
        assert!(output.contains("EFFECT:"), "Should contain EFFECT command");
        assert!(output.contains("\"gain\":0.5"), "Should set gain to 0.5");
    }

    #[tokio::test]
    async fn test_gain_clamp_high() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("1.5 MUSIC@GAIN").await;
        assert!(result.is_ok(), "GAIN should succeed with clamping");

        let output = interp.collect_output();
        assert!(output.contains("\"gain\":1"), "Should clamp to 1.0");
        assert!(output.contains("Warning:"), "Should warn about clamping");
    }

    #[tokio::test]
    async fn test_gain_clamp_low() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // Use arithmetic to get negative: 0 - 0.5 = -0.5
        let result = interp.execute("[ 0 ] [ 0.5 ] - MUSIC@GAIN").await;
        assert!(result.is_ok(), "GAIN should succeed with clamping");

        let output = interp.collect_output();
        assert!(output.contains("\"gain\":0"), "Should clamp to 0.0");
    }

    #[tokio::test]
    async fn test_gain_fraction() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("1/2 MUSIC@GAIN").await;
        assert!(result.is_ok(), "GAIN with fraction should succeed");

        let output = interp.collect_output();
        assert!(output.contains("\"gain\":0.5"), "1/2 should become 0.5");
    }

    #[tokio::test]
    async fn test_gain_reset() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("MUSIC@GAIN-RESET").await;
        assert!(result.is_ok(), "GAIN-RESET should succeed");

        let output = interp.collect_output();
        assert!(output.contains("\"gain\":1"), "Should reset to 1.0");
    }

    #[tokio::test]
    async fn test_pan_basic() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // Use arithmetic to get negative: 0 - 0.5 = -0.5
        let result = interp.execute("[ 0 ] [ 0.5 ] - MUSIC@PAN").await;
        assert!(result.is_ok(), "PAN should succeed");

        let output = interp.collect_output();
        assert!(output.contains("EFFECT:"), "Should contain EFFECT command");
        assert!(output.contains("\"pan\":-0.5"), "Should set pan to -0.5");
    }

    #[tokio::test]
    async fn test_pan_clamp() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("2 MUSIC@PAN").await;
        assert!(result.is_ok(), "PAN should succeed with clamping");

        let output = interp.collect_output();
        assert!(output.contains("\"pan\":1"), "Should clamp to 1.0");
        assert!(output.contains("Warning:"), "Should warn about clamping");
    }

    #[tokio::test]
    async fn test_pan_reset() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("MUSIC@PAN-RESET").await;
        assert!(result.is_ok(), "PAN-RESET should succeed");

        let output = interp.collect_output();
        assert!(output.contains("\"pan\":0"), "Should reset to 0.0");
    }

    #[tokio::test]
    async fn test_fx_reset() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("MUSIC@FX-RESET").await;
        assert!(result.is_ok(), "FX-RESET should succeed");

        let output = interp.collect_output();
        assert!(output.contains("\"gain\":1"), "Should reset gain to 1.0");
        assert!(output.contains("\"pan\":0"), "Should reset pan to 0.0");
    }

    #[tokio::test]
    async fn test_gain_then_play() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("0.5 MUSIC@GAIN [ 440 ] MUSIC@PLAY").await;
        assert!(result.is_ok(), "GAIN then MUSIC@PLAY should succeed");

        let output = interp.collect_output();
        assert!(output.contains("EFFECT:"), "Should have EFFECT command");
        assert!(output.contains("AUDIO:"), "Should have AUDIO command");
    }

    #[tokio::test]
    async fn test_combined_gain_pan_play() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp
            .execute("0.5 MUSIC@GAIN 0.7 MUSIC@PAN [ 440 ] MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "Combined MUSIC@GAIN MUSIC@PAN MUSIC@PLAY should succeed"
        );

        let output = interp.collect_output();
        assert!(output.contains("\"gain\":0.5"), "Should have gain 0.5");
        assert!(output.contains("\"pan\":0.7"), "Should have pan 0.7");
        assert!(output.contains("AUDIO:"), "Should have AUDIO command");
    }

    // ============================================================================
    // 歌詞（文字列混在ベクタ）テスト
    // ============================================================================

    #[tokio::test]
    async fn test_play_with_lyrics() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        // Without DisplayHint, the outer vector is treated as lyrics by
        // is_string_value. The entire vector (including nested string vectors)
        // is rendered as codepoint characters.
        let result = interp
            .execute("[ 440/2 'Hello' 550/2 'World' ] MUSIC@PLAY")
            .await;
        assert!(
            result.is_ok(),
            "PLAY with lyrics should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        // AUDIO command is emitted (with empty seq structure)
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }

    #[tokio::test]
    async fn test_play_with_duration_unreduced() {
        use crate::interpreter::Interpreter;

        // Without DisplayHint, is_string_value treats all vectors as strings,
        // so [440/2, 550/2] is treated as lyrics (codepoints).
        let mut interp = Interpreter::new();
        interp.execute("'music' IMPORT").await.unwrap();
        let result = interp.execute("[ 440/2 550/2 ] MUSIC@PLAY").await;
        assert!(
            result.is_ok(),
            "PLAY with unreduced fractions should succeed: {:?}",
            result
        );

        let output = interp.collect_output();
        assert!(
            output.contains("AUDIO:"),
            "Should contain AUDIO command"
        );
    }
}
