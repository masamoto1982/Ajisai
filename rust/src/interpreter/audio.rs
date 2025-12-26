// rust/src/interpreter/audio.rs
//
// 【責務】
// AUDIO ワードの実装。ベクタの要素を音声として再生する。
// 分数は分子と分母をそれぞれ周波数（Hz）として同時に鳴らす（DTMF方式）。
// 文字列はOutputエリアに出力し、NILは休符として扱う。
//
// 【仕様】
// - [ 1209/697 ] → 1209Hz と 697Hz を同時再生
// - [ 440 ] → 440Hz（分母が1の場合は単音）
// - [ NIL ] → 休符（無音）
// - [ 'Hello' ] → Outputエリアに出力
//
// 【時間軸の入れ子】
// ネストされたベクタは全て順次再生として解釈される。
// - 1次元: ノート（0.5秒）
// - 2次元: フレーズ（ノートの集まりを順次再生）
// - 3次元: セクション（フレーズを順次再生）
// - 4次元: 楽曲（セクションを順次再生）
//
// 例: [ [ 440 880 ] [ 550 660 ] ] AUDIO
//     → 440Hz → 880Hz → 550Hz → 660Hz（全て順次再生）
//
// 【オペレーションターゲット】
// - StackTop（デフォルト）: スタックトップのベクタを1トラックとして順次再生
// - Stack: スタック全体の各要素を複数トラックとして同時再生

use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueType};
use super::Interpreter;
use super::OperationTarget;
use num_traits::{ToPrimitive, One};
use serde::Serialize;

// ============================================================================
// AudioCommand 構造体（JSON出力用）
// ============================================================================

#[derive(Debug, Serialize)]
struct AudioNote {
    #[serde(rename = "type")]
    note_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequencies: Option<Vec<f64>>,
    duration: String,
}

#[derive(Debug, Serialize)]
struct AudioTrack {
    track: usize,
    notes: Vec<AudioNote>,
}

#[derive(Debug, Serialize)]
struct AudioCommand {
    #[serde(rename = "type")]
    command_type: String,
    tracks: Vec<AudioTrack>,
}

// ============================================================================
// AUDIO ワード実装
// ============================================================================

/// AUDIO ワードのエントリポイント
pub fn op_audio(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            // スタックトップのベクタを1トラックとして順次再生
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            process_audio_value(&val, &mut interp.output_buffer)?;
            Ok(())
        }
        OperationTarget::Stack => {
            // スタック全体の各要素を複数トラックとして同時再生
            let stack_values: Vec<Value> = interp.stack.drain(..).collect();
            if stack_values.is_empty() {
                return Err(AjisaiError::StackUnderflow);
            }
            process_audio_stack(&stack_values, &mut interp.output_buffer)?;
            Ok(())
        }
    }
}

/// スタックトップのベクタを処理（全て順次再生）
fn process_audio_value(value: &Value, output: &mut String) -> Result<()> {
    match &value.val_type {
        ValueType::Vector(elements) => {
            // 再帰的に全ての要素を収集して1トラックとして順次再生
            let notes = collect_notes_recursive(elements, output)?;
            if !notes.is_empty() {
                let tracks = vec![AudioTrack { track: 0, notes }];
                output_audio_command(output, tracks);
            }
            Ok(())
        }
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

/// スタック全体を複数トラックとして処理（同時再生）
fn process_audio_stack(stack_values: &[Value], output: &mut String) -> Result<()> {
    let mut tracks = Vec::new();

    for (track_index, value) in stack_values.iter().enumerate() {
        let notes = match &value.val_type {
            ValueType::Vector(elements) => {
                // 各スタック要素は1トラックとして、内部は順次再生
                collect_notes_recursive(elements, output)?
            }
            _ => {
                // 非ベクタ要素は単一ノートのトラックとして扱う
                collect_notes_from_value(value, output)?
            }
        };

        if !notes.is_empty() {
            tracks.push(AudioTrack {
                track: track_index,
                notes,
            });
        }
    }

    if !tracks.is_empty() {
        output_audio_command(output, tracks);
    }

    Ok(())
}

/// ベクタ要素を再帰的に走査してノートリストを構築（時間軸の入れ子）
fn collect_notes_recursive(elements: &[Value], output: &mut String) -> Result<Vec<AudioNote>> {
    let mut notes = Vec::new();

    for element in elements {
        let mut element_notes = collect_notes_from_value(element, output)?;
        notes.append(&mut element_notes);
    }

    Ok(notes)
}

/// 単一の値からノートリストを構築（再帰的）
fn collect_notes_from_value(value: &Value, output: &mut String) -> Result<Vec<AudioNote>> {
    match &value.val_type {
        ValueType::Number(frac) => {
            // 分数 → 分子と分母を周波数として解釈
            let numerator = frac.numerator.to_f64()
                .ok_or_else(|| AjisaiError::from("Numerator too large for frequency"))?;
            let denominator = frac.denominator.to_f64()
                .ok_or_else(|| AjisaiError::from("Denominator too large for frequency"))?;

            if numerator <= 0.0 {
                return Err(AjisaiError::from("Frequency must be positive"));
            }

            if denominator.is_one() || denominator == 1.0 {
                // 整数 → 単音
                Ok(vec![AudioNote {
                    note_type: "single".to_string(),
                    frequency: Some(numerator),
                    frequencies: None,
                    duration: "normal".to_string(),
                }])
            } else {
                // 分数 → 和音（分子と分母を同時に鳴らす）
                if denominator <= 0.0 {
                    return Err(AjisaiError::from("Frequency must be positive"));
                }
                Ok(vec![AudioNote {
                    note_type: "chord".to_string(),
                    frequency: None,
                    frequencies: Some(vec![numerator, denominator]),
                    duration: "normal".to_string(),
                }])
            }
        }
        ValueType::Nil => {
            // NIL → 休符
            Ok(vec![AudioNote {
                note_type: "rest".to_string(),
                frequency: None,
                frequencies: None,
                duration: "normal".to_string(),
            }])
        }
        ValueType::String(s) => {
            // 文字列 → Outputに出力（音ではない）
            output.push_str(s);
            output.push(' ');
            Ok(vec![])
        }
        ValueType::Vector(inner) => {
            // ネストされたベクタ → 再帰的に処理（順次再生）
            collect_notes_recursive(inner, output)
        }
        _ => {
            // その他の型は無視
            Ok(vec![])
        }
    }
}

/// AudioCommand を JSON として output_buffer に出力
fn output_audio_command(output: &mut String, tracks: Vec<AudioTrack>) {
    let command = AudioCommand {
        command_type: "sound".to_string(),
        tracks,
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
        Value {
            val_type: ValueType::Number(Fraction::new(
                BigInt::from(n),
                BigInt::from(1),
            )),
        }
    }

    fn make_fraction(num: i64, den: i64) -> Value {
        Value {
            val_type: ValueType::Number(Fraction::new(
                BigInt::from(num),
                BigInt::from(den),
            )),
        }
    }

    fn make_nil() -> Value {
        Value { val_type: ValueType::Nil }
    }

    fn make_string(s: &str) -> Value {
        Value { val_type: ValueType::String(s.to_string()) }
    }

    fn make_vector(elements: Vec<Value>) -> Value {
        Value { val_type: ValueType::Vector(elements) }
    }

    #[test]
    fn test_single_tone() {
        let elements = vec![make_number(440)];
        let mut output = String::new();
        let notes = collect_notes_recursive(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].note_type, "single");
        assert_eq!(notes[0].frequency, Some(440.0));
    }

    #[test]
    fn test_chord_dtmf() {
        let elements = vec![make_fraction(1209, 697)];
        let mut output = String::new();
        let notes = collect_notes_recursive(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].note_type, "chord");
        assert_eq!(notes[0].frequencies, Some(vec![1209.0, 697.0]));
    }

    #[test]
    fn test_rest() {
        let elements = vec![make_nil()];
        let mut output = String::new();
        let notes = collect_notes_recursive(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].note_type, "rest");
    }

    #[test]
    fn test_string_output() {
        let elements = vec![make_string("Hello")];
        let mut output = String::new();
        let notes = collect_notes_recursive(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 0);
        assert!(output.contains("Hello"));
    }

    #[test]
    fn test_sequence() {
        let elements = vec![
            make_fraction(1209, 697),  // DTMF 1
            make_nil(),                 // 休符
            make_fraction(1336, 770),  // DTMF 5
        ];
        let mut output = String::new();
        let notes = collect_notes_recursive(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].note_type, "chord");
        assert_eq!(notes[1].note_type, "rest");
        assert_eq!(notes[2].note_type, "chord");
    }

    #[test]
    fn test_nested_vector_flattens_to_sequence() {
        // [ [ 440 880 ] [ 550 660 ] ] → 4ノート順次再生
        let elements = vec![
            make_vector(vec![make_number(440), make_number(880)]),
            make_vector(vec![make_number(550), make_number(660)]),
        ];
        let mut output = String::new();
        let notes = collect_notes_recursive(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 4);
        assert_eq!(notes[0].frequency, Some(440.0));
        assert_eq!(notes[1].frequency, Some(880.0));
        assert_eq!(notes[2].frequency, Some(550.0));
        assert_eq!(notes[3].frequency, Some(660.0));
    }

    #[test]
    fn test_deeply_nested_vector() {
        // [ [ [ 440 ] [ 880 ] ] [ [ 550 ] ] ] → 3ノート順次再生
        let elements = vec![
            make_vector(vec![
                make_vector(vec![make_number(440)]),
                make_vector(vec![make_number(880)]),
            ]),
            make_vector(vec![
                make_vector(vec![make_number(550)]),
            ]),
        ];
        let mut output = String::new();
        let notes = collect_notes_recursive(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].frequency, Some(440.0));
        assert_eq!(notes[1].frequency, Some(880.0));
        assert_eq!(notes[2].frequency, Some(550.0));
    }

    #[test]
    fn test_mixed_nested_with_string() {
        // [ [ 440 'Hello' ] [ 880 ] ] → 2ノート + 文字列出力
        let elements = vec![
            make_vector(vec![make_number(440), make_string("Hello")]),
            make_vector(vec![make_number(880)]),
        ];
        let mut output = String::new();
        let notes = collect_notes_recursive(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].frequency, Some(440.0));
        assert_eq!(notes[1].frequency, Some(880.0));
        assert!(output.contains("Hello"));
    }

    #[tokio::test]
    async fn test_audio_integration() {
        use crate::interpreter::Interpreter;

        let mut interp = Interpreter::new();
        let result = interp.execute("[ 440 ] AUDIO").await;
        assert!(result.is_ok(), "AUDIO should succeed: {:?}", result);

        let output = interp.get_output();
        assert!(output.contains("AUDIO:"), "Output should contain AUDIO command: {}", output);
        assert!(output.contains("\"type\":\"single\""), "Should contain single note");
        assert!(output.contains("\"frequency\":440"), "Should contain frequency 440");
    }
}
