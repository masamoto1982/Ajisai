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
// - . PLAY（デフォルト）: スタックトップを再生
// - .. PLAY: スタック全体を再生（マルチトラック）

use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueData, DisplayHint, AudioHint, Envelope, WaveformType};
use super::Interpreter;
use super::OperationTarget;
use num_traits::ToPrimitive;
use serde::Serialize;

// ============================================================================
// ヘルパー関数（統一Value宇宙アーキテクチャ用）
// ============================================================================

/// ベクタ値かどうかを判定
fn is_vector_value(val: &Value) -> bool {
    matches!(&val.data, ValueData::Vector(_))
}

/// 文字列値かどうかを判定
fn is_string_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::String && !val.is_nil()
}

/// Valueから文字列を取得
fn value_as_string(val: &Value) -> String {
    fn collect_chars(val: &Value) -> Vec<char> {
        match &val.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(f) => {
                f.to_i64().and_then(|n| {
                    if n >= 0 && n <= 0x10FFFF {
                        char::from_u32(n as u32)
                    } else {
                        None
                    }
                }).map(|c| vec![c]).unwrap_or_default()
            }
            ValueData::Vector(children) => {
                children.iter().flat_map(|c| collect_chars(c)).collect()
            }
            ValueData::CodeBlock(_) => vec![],
        }
    }
    collect_chars(val).into_iter().collect()
}

/// ベクタの子要素を取得
fn get_vector_children(val: &Value) -> Option<&Vec<Value>> {
    if let ValueData::Vector(children) = &val.data {
        Some(children)
    } else {
        None
    }
}

// ============================================================================
// PlayMode - 再生モード
// ============================================================================

/// 再生モード（SEQ/SIM指定用）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PlayMode {
    #[default]
    Sequential,   // 順次再生
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
        duration: f64,  // スロット数
        #[serde(skip_serializing_if = "Option::is_none")]
        envelope: Option<Envelope>,
        #[serde(skip_serializing_if = "is_default_waveform")]
        waveform: WaveformType,
    },
    #[serde(rename = "rest")]
    Rest {
        duration: f64,
    },
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
// SEQ ワード実装
// ============================================================================

/// SEQ ワード - 順次再生モードを設定
pub fn op_seq(interp: &mut Interpreter) -> Result<()> {
    interp.play_mode = PlayMode::Sequential;
    Ok(())
}

// ============================================================================
// SIM ワード実装
// ============================================================================

/// SIM ワード - 同時再生モードを設定
pub fn op_sim(interp: &mut Interpreter) -> Result<()> {
    interp.play_mode = PlayMode::Simultaneous;
    Ok(())
}

// ============================================================================
// CHORD ワード実装
// ============================================================================

/// CHORD ワード - ベクタを同時再生（和音）としてマーク
pub fn op_chord(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // ベクタでなければエラー
    if !val.is_vector() {
        return Err(AjisaiError::from("CHORD requires a vector"));
    }

    // AudioHintを設定（既存のヒントがあればchordフラグを追加）
    let mut new_val = val;
    let hint = new_val.audio_hint.get_or_insert(AudioHint::default());
    hint.chord = true;

    interp.stack.push(new_val);
    Ok(())
}

// ============================================================================
// ADSR ワード実装
// ============================================================================

/// ADSR ワード - ADSRエンベロープを設定
/// スタック: [ target ] [ attack decay sustain release ] -- [ target ]'
/// 対象ベクタにADSRエンベロープを適用
pub fn op_adsr(interp: &mut Interpreter) -> Result<()> {
    // ADSRパラメータを取得
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
        return Err(AjisaiError::from("ADSR requires exactly 4 values: [attack decay sustain release]"));
    }

    // 対象がベクタでなければエラー
    if !target.is_vector() {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from("ADSR target must be a vector"));
    }

    // 各値を取得
    let attack = param_children[0].as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR attack must be a number"))?;
    let decay = param_children[1].as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR decay must be a number"))?;
    let sustain = param_children[2].as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR sustain must be a number"))?;
    let release = param_children[3].as_scalar()
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
        return Err(AjisaiError::from("ADSR sustain must be between 0.0 and 1.0"));
    }

    // AudioHintを設定（対象ベクタに適用）
    let mut new_val = target;
    let hint = new_val.audio_hint.get_or_insert(AudioHint::default());
    hint.envelope = Some(Envelope { attack, decay, sustain, release });

    interp.stack.push(new_val);
    Ok(())
}

// ============================================================================
// 波形ワード実装
// ============================================================================

/// SINE ワード - 正弦波を設定
pub fn op_sine(interp: &mut Interpreter) -> Result<()> {
    set_waveform(interp, WaveformType::Sine)
}

/// SQUARE ワード - 矩形波を設定
pub fn op_square(interp: &mut Interpreter) -> Result<()> {
    set_waveform(interp, WaveformType::Square)
}

/// SAW ワード - のこぎり波を設定
pub fn op_saw(interp: &mut Interpreter) -> Result<()> {
    set_waveform(interp, WaveformType::Sawtooth)
}

/// TRI ワード - 三角波を設定
pub fn op_tri(interp: &mut Interpreter) -> Result<()> {
    set_waveform(interp, WaveformType::Triangle)
}

/// 波形を設定するヘルパー関数
fn set_waveform(interp: &mut Interpreter, waveform: WaveformType) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // ベクタでなければエラー
    if !val.is_vector() {
        return Err(AjisaiError::from("Waveform word requires a vector"));
    }

    // AudioHintを設定
    let mut new_val = val;
    let hint = new_val.audio_hint.get_or_insert(AudioHint::default());
    hint.waveform = waveform;

    interp.stack.push(new_val);
    Ok(())
}

// ============================================================================
// PLAY ワード実装
// ============================================================================

/// PLAY ワードのエントリポイント
pub fn op_play(interp: &mut Interpreter) -> Result<()> {
    let mode = interp.play_mode;
    let target = interp.operation_target;

    match target {
        OperationTarget::StackTop => {
            // スタックトップのベクタを処理
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let structure = build_audio_structure(&val, mode, &mut interp.output_buffer)?;
            output_play_command(&structure, &mut interp.output_buffer);
        }
        OperationTarget::Stack => {
            // スタック全体の各要素を処理
            let values: Vec<Value> = interp.stack.drain(..).collect();
            if values.is_empty() {
                return Err(AjisaiError::StackUnderflow);
            }

            // 各値を順次再生として構築
            let structures: Vec<AudioStructure> = values.iter()
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

            output_play_command(&combined, &mut interp.output_buffer);
        }
    }

    // リセット
    interp.play_mode = PlayMode::Sequential;
    interp.operation_target = OperationTarget::StackTop;

    Ok(())
}

// ============================================================================
// build_audio_structure 関数
// ============================================================================

/// 値からAudioStructureを構築
fn build_audio_structure(
    value: &Value,
    mode: PlayMode,
    output: &mut String
) -> Result<AudioStructure> {
    // AudioHintを取得
    let audio_hint = value.get_audio_hint();
    let envelope = audio_hint.and_then(|h| h.envelope);
    let waveform = audio_hint.map(|h| h.waveform).unwrap_or_default();
    let is_chord = audio_hint.map(|h| h.chord).unwrap_or(false);

    // NIL判定
    if value.is_nil() {
        return Ok(AudioStructure::Rest { duration: 1.0 });
    }

    // 文字列判定
    if is_string_value(value) {
        let s = value_as_string(value);
        output.push_str(&s);
        output.push(' ');
        return Ok(AudioStructure::Seq { children: vec![], envelope: None, waveform: WaveformType::Sine });
    }

    // ベクタ判定
    if is_vector_value(value) {
        if let Some(children) = get_vector_children(value) {
            if children.is_empty() {
                return Err(AjisaiError::from("Empty vector not allowed"));
            }

            let audio_children: Vec<AudioStructure> = children.iter()
                .map(|e| build_audio_structure(e, PlayMode::Sequential, output))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .filter(|s| !matches!(s, AudioStructure::Seq { children, .. } if children.is_empty()))
                .collect();

            // CHORDフラグがあれば同時再生、なければモードに従う
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
        let freq = frac.numerator.to_f64()
            .ok_or_else(|| AjisaiError::from("Frequency too large"))?;
        let dur = frac.denominator.to_f64()
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
    Ok(AudioStructure::Seq { children: vec![], envelope: None, waveform: WaveformType::Sine })
}

// ============================================================================
// ヘルパー関数
// ============================================================================

/// 可聴域（20Hz〜20,000Hz）のチェックと警告出力
fn check_audible_range(freq: f64, output: &mut String) {
    const MIN_AUDIBLE: f64 = 20.0;
    const MAX_AUDIBLE: f64 = 20000.0;

    if freq < MIN_AUDIBLE {
        output.push_str(&format!("Warning: {}Hz is below audible range (< 20Hz)\n", freq));
    } else if freq > MAX_AUDIBLE {
        output.push_str(&format!("Warning: {}Hz is above audible range (> 20kHz)\n", freq));
    }
}

/// PlayCommand を JSON として output_buffer に出力
fn output_play_command(structure: &AudioStructure, output: &mut String) {
    let command = PlayCommand {
        command_type: "play".to_string(),
        structure: structure.clone(),
    };

    if let Ok(json) = serde_json::to_string(&command) {
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

    fn make_number(n: i64) -> Value {
        Value::from_fraction(Fraction::new(
            BigInt::from(n),
            BigInt::from(1),
        ))
    }

    fn make_fraction(num: i64, den: i64) -> Value {
        Value::from_fraction(Fraction::new(
            BigInt::from(num),
            BigInt::from(den),
        ))
    }

    fn make_nil() -> Value {
        Value::nil()
    }

    fn make_vector(elements: Vec<Value>) -> Value {
        Value::from_vector(elements)
    }

    #[test]
    fn test_tone_from_integer() {
        // 440 → 440Hz, 1スロット
        let val = make_number(440);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Tone { frequency, duration, .. } => {
                assert_eq!(frequency, 440.0);
                assert_eq!(duration, 1.0);
            }
            _ => panic!("Expected Tone"),
        }
    }

    #[test]
    fn test_tone_from_fraction() {
        // 440/3 → 440Hz, 3スロット (440 and 3 are coprime, no normalization)
        let val = make_fraction(440, 3);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Tone { frequency, duration, .. } => {
                assert_eq!(frequency, 440.0);
                assert_eq!(duration, 3.0);
            }
            _ => panic!("Expected Tone"),
        }
    }

    #[test]
    fn test_tone_from_fraction_normalized() {
        // 440/2 gets normalized to 220/1 by Fraction::new (GCD reduction)
        // This becomes 220Hz, 1スロット
        let val = make_fraction(440, 2);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Tone { frequency, duration, .. } => {
                // Fraction(440, 2) normalizes to Fraction(220, 1)
                assert_eq!(frequency, 220.0);
                assert_eq!(duration, 1.0);
            }
            _ => panic!("Expected Tone"),
        }
    }

    #[test]
    fn test_rest_from_zero() {
        // 0/4 gets normalized to 0/1 by Fraction::new (GCD reduction with 0)
        // This becomes 1-slot rest
        let val = make_fraction(0, 4);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Rest { duration } => {
                // Fraction(0, 4) normalizes to Fraction(0, 1)
                assert_eq!(duration, 1.0);
            }
            _ => panic!("Expected Rest"),
        }
    }

    #[test]
    fn test_rest_from_zero_coprime() {
        // 0/3 also normalizes to 0/1, same behavior
        let val = make_fraction(0, 3);
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
    fn test_rest_from_nil() {
        // NIL → 1スロット休符
        let val = make_nil();
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
        // [ 440 550 ] → Seq [ Tone(440), Tone(550) ]
        let elements = vec![make_number(440), make_number(550)];
        let val = make_vector(elements);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        match structure {
            AudioStructure::Seq { children, .. } => {
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected Seq"),
        }
    }

    #[test]
    fn test_sim_structure() {
        // [ 440 550 ] SIM → Sim [ Tone(440), Tone(550) ]
        let elements = vec![make_number(440), make_number(550)];
        let val = make_vector(elements);
        let mut output = String::new();
        let structure = build_audio_structure(&val, PlayMode::Simultaneous, &mut output).unwrap();

        match structure {
            AudioStructure::Sim { children, .. } => {
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected Sim"),
        }
    }

    #[test]
    fn test_negative_frequency_error() {
        let val = make_number(-440);
        let mut output = String::new();
        let result = build_audio_structure(&val, PlayMode::Sequential, &mut output);

        assert!(result.is_err());
    }

    #[test]
    fn test_audible_range_warning_low() {
        let val = make_number(10); // 10Hz - below audible
        let mut output = String::new();
        let _ = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        assert!(output.contains("Warning:"), "Should warn about inaudible frequency");
        assert!(output.contains("below audible range"), "Should mention below range");
    }

    #[test]
    fn test_audible_range_warning_high() {
        let val = make_number(25000); // 25kHz - above audible
        let mut output = String::new();
        let _ = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        assert!(output.contains("Warning:"), "Should warn about inaudible frequency");
        assert!(output.contains("above audible range"), "Should mention above range");
    }

    #[test]
    fn test_audible_range_no_warning() {
        let val = make_number(440); // 440Hz - audible
        let mut output = String::new();
        let _ = build_audio_structure(&val, PlayMode::Sequential, &mut output).unwrap();

        assert!(!output.contains("Warning:"), "Should not warn for audible frequency");
    }

    #[tokio::test]
    async fn test_play_integration() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 ] PLAY").await;
        assert!(result.is_ok(), "PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("AUDIO:"), "Output should contain AUDIO command: {}", output);
        assert!(output.contains("\"type\":\"play\""), "Should contain play type");
        assert!(output.contains("\"type\":\"tone\""), "Should contain tone type");
        assert!(output.contains("\"frequency\":440"), "Should contain frequency 440");
    }

    #[tokio::test]
    async fn test_seq_play_integration() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 550 660 ] SEQ PLAY").await;
        assert!(result.is_ok(), "SEQ PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"type\":\"seq\""), "Should contain seq structure");
    }

    #[tokio::test]
    async fn test_sim_play_integration() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 550 660 ] SIM PLAY").await;
        assert!(result.is_ok(), "SIM PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"type\":\"sim\""), "Should contain sim structure");
    }

    #[tokio::test]
    async fn test_multitrack_play() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 550 ] [ 220 275 ] .. SIM PLAY").await;
        assert!(result.is_ok(), "Multitrack PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"type\":\"sim\""), "Should contain sim structure for multitrack");

        // スタックが空であることを確認（両方のベクタが消費されたはず）
        assert!(interp.get_stack().is_empty(), "Stack should be empty after .. SIM PLAY, but had {} elements", interp.get_stack().len());
    }

    #[tokio::test]
    async fn test_multitrack_seq_play() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 550 ] [ 220 275 ] .. SEQ PLAY").await;
        assert!(result.is_ok(), "Multitrack SEQ PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"type\":\"seq\""), "Should contain seq structure for multitrack");

        // スタックが空であることを確認
        assert!(interp.get_stack().is_empty(), "Stack should be empty after .. SEQ PLAY");
    }

    #[tokio::test]
    async fn test_multitrack_play_consumes_all() {
        use crate::interpreter::Interpreter;

        // 3つのトラックをテスト
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 ] [ 550 ] [ 660 ] .. SIM PLAY").await;
        assert!(result.is_ok(), "3-track PLAY should succeed: {:?}", result);

        // スタックが完全に空であることを確認
        assert!(interp.get_stack().is_empty(), "Stack should be completely empty after playing 3 tracks");
    }

    #[tokio::test]
    async fn test_play_with_duration() {
        use crate::interpreter::Interpreter;

        // Use coprime fractions: 440/3 and 660/7 don't normalize
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440/3 550/1 660/7 ] PLAY").await;
        assert!(result.is_ok(), "PLAY with duration should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"duration\":3"), "Should contain duration 3");
        assert!(output.contains("\"duration\":7"), "Should contain duration 7");
    }

    #[tokio::test]
    async fn test_play_with_zero_rest() {
        use crate::interpreter::Interpreter;

        // Note: 0/n always normalizes to 0/1, so rest duration is always 1
        // This is a limitation of the current fraction normalization
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 0/2 550 ] PLAY").await;
        assert!(result.is_ok(), "PLAY with 0/n rest should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"type\":\"rest\""), "Should contain rest");
        // 0/2 normalizes to 0/1, so duration is 1, not 2
        assert!(output.contains("\"duration\":1"), "Should contain duration 1 for normalized rest");
    }

    // ============================================================================
    // 新機能テスト: CHORD, ADSR, SINE, SQUARE, SAW, TRI
    // ============================================================================

    #[tokio::test]
    async fn test_chord_marks_as_simultaneous() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 550 660 ] CHORD PLAY").await;
        assert!(result.is_ok(), "CHORD PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"type\":\"sim\""), "CHORD should produce sim structure");
    }

    #[tokio::test]
    async fn test_chord_requires_vector() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("440 CHORD").await;
        assert!(result.is_err(), "CHORD on non-vector should fail");
    }

    #[tokio::test]
    async fn test_adsr_sets_envelope() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        // ADSR takes two arguments: target vector and ADSR params
        let result = interp.execute("[ 440 ] [ 0.05 0.1 0.8 0.2 ] ADSR PLAY").await;
        assert!(result.is_ok(), "ADSR PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"envelope\""), "Should contain envelope");
        assert!(output.contains("\"attack\":0.05"), "Should contain attack value");
        assert!(output.contains("\"sustain\":0.8"), "Should contain sustain value");
    }

    #[tokio::test]
    async fn test_adsr_requires_4_elements() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        // ADSR needs target + params
        let result = interp.execute("[ 440 ] [ 0.1 0.2 0.3 ] ADSR").await;
        assert!(result.is_err(), "ADSR with 3 elements should fail");
    }

    #[tokio::test]
    async fn test_adsr_sustain_validation() {
        use crate::interpreter::Interpreter;

        // Sustain > 1.0 should fail
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 ] [ 0.1 0.1 1.5 0.1 ] ADSR").await;
        assert!(result.is_err(), "ADSR with sustain > 1.0 should fail");
    }

    #[tokio::test]
    async fn test_square_waveform() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 ] SQUARE PLAY").await;
        assert!(result.is_ok(), "SQUARE PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"waveform\":\"square\""), "Should contain square waveform");
    }

    #[tokio::test]
    async fn test_saw_waveform() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 ] SAW PLAY").await;
        assert!(result.is_ok(), "SAW PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"waveform\":\"sawtooth\""), "Should contain sawtooth waveform");
    }

    #[tokio::test]
    async fn test_tri_waveform() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 ] TRI PLAY").await;
        assert!(result.is_ok(), "TRI PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"waveform\":\"triangle\""), "Should contain triangle waveform");
    }

    #[tokio::test]
    async fn test_sine_is_default_not_serialized() {
        use crate::interpreter::Interpreter;

        // SINE is the default, so it shouldn't be serialized
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 ] SINE PLAY").await;
        assert!(result.is_ok(), "SINE PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        // Default sine should not appear in output (skip_serializing_if)
        assert!(!output.contains("\"waveform\":\"sine\""), "Sine waveform should not be serialized as it's default");
    }

    #[tokio::test]
    async fn test_combined_chord_adsr_waveform() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 550 660 ] CHORD [ 0.01 0.1 0.7 0.3 ] ADSR SQUARE PLAY").await;
        assert!(result.is_ok(), "Combined CHORD ADSR SQUARE PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"type\":\"sim\""), "Should be chord (sim)");
        assert!(output.contains("\"envelope\""), "Should have envelope");
        assert!(output.contains("\"waveform\":\"square\""), "Should be square wave");
    }

    #[tokio::test]
    async fn test_chord_can_be_defined_and_reused() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        // Define a chord
        let result = interp.execute("[ [ 440 550 660 ] CHORD ] 'C_MAJOR' DEF").await;
        assert!(result.is_ok(), "DEF should succeed: {:?}", result);

        // Use it
        let result = interp.execute("C_MAJOR PLAY").await;
        assert!(result.is_ok(), "C_MAJOR PLAY should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("\"type\":\"sim\""), "Defined chord should produce sim");
    }

}
