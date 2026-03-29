// rust/src/interpreter/audio/execute-audio-commands.rs
//
// 【責務】
// 音楽DSLのワード実装。SEQ, SIM, SLOT, GAIN, PAN, CHORD, ADSR, 波形ワード。

use super::audio_types::{update_play_mode, PlayMode, WaveformType};
use super::super::Interpreter;
use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};
use num_traits::ToPrimitive;

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
    apply_waveform(interp, WaveformType::Sine)
}

/// MUSIC@SQUARE ワード - 矩形波を設定
pub fn op_square(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Square)
}

/// MUSIC@SAW ワード - のこぎり波を設定
pub fn op_saw(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Sawtooth)
}

/// MUSIC@TRI ワード - 三角波を設定
pub fn op_tri(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Triangle)
}

/// 波形を設定するヘルパー関数
fn apply_waveform(interp: &mut Interpreter, _waveform: WaveformType) -> Result<()> {
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
