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
// - [ [ 1 2 ] [ 3 4 ] ] → 2トラックを同時再生
//
// 【オペレーションターゲット】
// - StackTop（デフォルト）: スタックトップのベクタを再生
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
            // スタックトップのベクタを再生
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

/// スタックトップのベクタを処理
fn process_audio_value(value: &Value, output: &mut String) -> Result<()> {
    match &value.val_type {
        ValueType::Vector(elements) => {
            // ネストされたベクタがあるか確認
            let has_nested_vectors = elements.iter().any(|e| matches!(e.val_type, ValueType::Vector(_)));

            if has_nested_vectors {
                // ネストされたベクタ → 複数トラックとして処理
                let mut tracks = Vec::new();
                let mut track_index = 0;

                for element in elements {
                    match &element.val_type {
                        ValueType::Vector(inner) => {
                            let notes = build_notes_from_elements(inner, output)?;
                            if !notes.is_empty() {
                                tracks.push(AudioTrack {
                                    track: track_index,
                                    notes,
                                });
                                track_index += 1;
                            }
                        }
                        ValueType::String(s) => {
                            // 文字列はOutputに出力
                            output.push_str(s);
                            output.push(' ');
                        }
                        _ => {
                            // 単一要素はトラックとして扱う
                            let notes = build_notes_from_elements(&[element.clone()], output)?;
                            if !notes.is_empty() {
                                tracks.push(AudioTrack {
                                    track: track_index,
                                    notes,
                                });
                                track_index += 1;
                            }
                        }
                    }
                }

                if !tracks.is_empty() {
                    output_audio_command(output, tracks);
                }
            } else {
                // フラットなベクタ → 1トラックとして処理
                let notes = build_notes_from_elements(elements, output)?;
                if !notes.is_empty() {
                    let tracks = vec![AudioTrack { track: 0, notes }];
                    output_audio_command(output, tracks);
                }
            }
            Ok(())
        }
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

/// スタック全体を複数トラックとして処理
fn process_audio_stack(stack_values: &[Value], output: &mut String) -> Result<()> {
    let mut tracks = Vec::new();

    for (track_index, value) in stack_values.iter().enumerate() {
        match &value.val_type {
            ValueType::Vector(elements) => {
                let notes = build_notes_from_elements(elements, output)?;
                if !notes.is_empty() {
                    tracks.push(AudioTrack {
                        track: track_index,
                        notes,
                    });
                }
            }
            _ => {
                // 非ベクタ要素は単一ノートのトラックとして扱う
                let notes = build_notes_from_single_value(value, output)?;
                if !notes.is_empty() {
                    tracks.push(AudioTrack {
                        track: track_index,
                        notes,
                    });
                }
            }
        }
    }

    if !tracks.is_empty() {
        output_audio_command(output, tracks);
    }

    Ok(())
}

/// ベクタ要素からノートリストを構築
fn build_notes_from_elements(elements: &[Value], output: &mut String) -> Result<Vec<AudioNote>> {
    let mut notes = Vec::new();

    for element in elements {
        match &element.val_type {
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
                    notes.push(AudioNote {
                        note_type: "single".to_string(),
                        frequency: Some(numerator),
                        frequencies: None,
                        duration: "normal".to_string(),
                    });
                } else {
                    // 分数 → 和音（分子と分母を同時に鳴らす）
                    if denominator <= 0.0 {
                        return Err(AjisaiError::from("Frequency must be positive"));
                    }
                    notes.push(AudioNote {
                        note_type: "chord".to_string(),
                        frequency: None,
                        frequencies: Some(vec![numerator, denominator]),
                        duration: "normal".to_string(),
                    });
                }
            }
            ValueType::Nil => {
                // NIL → 休符
                notes.push(AudioNote {
                    note_type: "rest".to_string(),
                    frequency: None,
                    frequencies: None,
                    duration: "normal".to_string(),
                });
            }
            ValueType::String(s) => {
                // 文字列 → Outputに出力（音ではない）
                output.push_str(s);
                output.push(' ');
            }
            ValueType::Vector(_) => {
                // ネストされたベクタは無視（上位で処理済み）
            }
            _ => {
                // その他の型は無視
            }
        }
    }

    Ok(notes)
}

/// 単一の値からノートリストを構築
fn build_notes_from_single_value(value: &Value, output: &mut String) -> Result<Vec<AudioNote>> {
    build_notes_from_elements(&[value.clone()], output)
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

    #[test]
    fn test_single_tone() {
        let elements = vec![make_number(440)];
        let mut output = String::new();
        let notes = build_notes_from_elements(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].note_type, "single");
        assert_eq!(notes[0].frequency, Some(440.0));
    }

    #[test]
    fn test_chord_dtmf() {
        let elements = vec![make_fraction(1209, 697)];
        let mut output = String::new();
        let notes = build_notes_from_elements(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].note_type, "chord");
        assert_eq!(notes[0].frequencies, Some(vec![1209.0, 697.0]));
    }

    #[test]
    fn test_rest() {
        let elements = vec![make_nil()];
        let mut output = String::new();
        let notes = build_notes_from_elements(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].note_type, "rest");
    }

    #[test]
    fn test_string_output() {
        let elements = vec![make_string("Hello")];
        let mut output = String::new();
        let notes = build_notes_from_elements(&elements, &mut output).unwrap();

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
        let notes = build_notes_from_elements(&elements, &mut output).unwrap();

        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].note_type, "chord");
        assert_eq!(notes[1].note_type, "rest");
        assert_eq!(notes[2].note_type, "chord");
    }
}
