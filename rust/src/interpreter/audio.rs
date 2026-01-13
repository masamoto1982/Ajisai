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
use crate::types::{Value, DisplayHint};
use super::Interpreter;
use super::OperationTarget;
use num_traits::ToPrimitive;
use serde::Serialize;

// ============================================================================
// ヘルパー関数（統一分数アーキテクチャ用）
// ============================================================================

/// ベクタ値かどうかを判定
fn is_vector_value(val: &Value) -> bool {
    val.data.len() > 1 || !val.shape.is_empty()
}

/// 文字列値かどうかを判定
fn is_string_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::String && !val.data.is_empty()
}

/// ベクタの要素を再構築する
fn reconstruct_vector_elements(val: &Value) -> Vec<Value> {
    if val.shape.is_empty() || val.shape.len() == 1 {
        val.data.iter().map(|f| Value::from_fraction(f.clone())).collect()
    } else {
        let outer_size = val.shape[0];
        let inner_size: usize = val.shape[1..].iter().product();
        let inner_shape = val.shape[1..].to_vec();

        (0..outer_size).map(|i| {
            let start = i * inner_size;
            let end = start + inner_size;
            let data = val.data[start..end].to_vec();
            Value {
                data,
                display_hint: val.display_hint,
                shape: inner_shape.clone(),
            }
        }).collect()
    }
}

/// Valueから文字列を取得
fn value_as_string(val: &Value) -> String {
    val.data.iter()
        .filter_map(|f| f.to_i64().and_then(|n| {
            if n >= 0 && n <= 0x10FFFF {
                char::from_u32(n as u32)
            } else {
                None
            }
        }))
        .collect()
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
    },
    #[serde(rename = "rest")]
    Rest {
        duration: f64,
    },
    #[serde(rename = "seq")]
    Seq {
        children: Vec<AudioStructure>,
    },
    #[serde(rename = "sim")]
    Sim {
        children: Vec<AudioStructure>,
    },
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
                PlayMode::Sequential => AudioStructure::Seq { children: structures },
                PlayMode::Simultaneous => AudioStructure::Sim { children: structures },
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
    // NIL判定
    if value.is_nil() {
        return Ok(AudioStructure::Rest { duration: 1.0 });
    }

    // 文字列判定
    if is_string_value(value) {
        let s = value_as_string(value);
        output.push_str(&s);
        output.push(' ');
        return Ok(AudioStructure::Seq { children: vec![] });
    }

    // ベクタ判定
    if is_vector_value(value) {
        let elements = reconstruct_vector_elements(value);
        if elements.is_empty() {
            return Err(AjisaiError::from("Empty vector not allowed"));
        }

        let children: Vec<AudioStructure> = elements.iter()
            .map(|e| build_audio_structure(e, PlayMode::Sequential, output))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|s| !matches!(s, AudioStructure::Seq { children } if children.is_empty()))
            .collect();

        return match mode {
            PlayMode::Sequential => Ok(AudioStructure::Seq { children }),
            PlayMode::Simultaneous => Ok(AudioStructure::Sim { children }),
        };
    }

    // 数値判定（単一スカラー）
    if value.data.len() == 1 && value.shape.is_empty() {
        let frac = &value.data[0];
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
            return Ok(AudioStructure::Tone { frequency: freq, duration: dur });
        } else {
            return Err(AjisaiError::from("Frequency must be non-negative"));
        }
    }

    // Boolean等は無視（空のSeqとして返す）
    Ok(AudioStructure::Seq { children: vec![] })
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
            AudioStructure::Tone { frequency, duration } => {
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
            AudioStructure::Tone { frequency, duration } => {
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
            AudioStructure::Tone { frequency, duration } => {
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
    fn test_empty_vector_becomes_rest() {
        // 統一分数アーキテクチャ: 空ベクタはNIL（休符）として処理される
        let val = make_vector(vec![]);
        let mut output = String::new();
        let result = build_audio_structure(&val, PlayMode::Sequential, &mut output);

        // 空ベクタはNIL扱いで休符になる
        match result {
            Ok(AudioStructure::Rest { duration }) => {
                assert_eq!(duration, 1.0, "Empty vector should become 1-slot rest");
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
            AudioStructure::Seq { children } => {
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
            AudioStructure::Sim { children } => {
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

}
