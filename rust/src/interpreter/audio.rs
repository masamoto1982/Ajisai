// rust/src/interpreter/audio.rs (修正版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, BracketType};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, One};
use serde_json::json;

pub fn op_sound(interp: &mut Interpreter) -> Result<()> {
    let music_data = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    
    match &music_data.val_type {
        ValueType::Vector(tracks, _) => {
            let mut audio_commands = Vec::new();
            
            for (track_index, track) in tracks.iter().enumerate() {
                match &track.val_type {
                    ValueType::String(s) => {
                        // 文字列は出力に表示
                        interp.output_buffer.push_str(&format!("{}\n", s));
                    },
                    ValueType::Vector(notes, _) => {
                        // トラックデータを処理
                        let track_data = process_track(notes)?;
                        if !track_data.is_empty() {
                            audio_commands.push(json!({
                                "track": track_index,
                                "notes": track_data
                            }));
                        }
                    },
                    _ => {
                        // その他の型は単一トラックとして処理
                        let single_note = process_single_note(track)?;
                        if let Some(note_data) = single_note {
                            audio_commands.push(json!({
                                "track": 0,
                                "notes": vec![note_data]
                            }));
                        }
                    }
                }
            }
            
            if !audio_commands.is_empty() {
                // JavaScriptに音声コマンドを送信
                let audio_json = json!({
                    "type": "sound",
                    "tracks": audio_commands
                });
                interp.output_buffer.push_str(&format!("AUDIO:{}\n", audio_json.to_string()));
            }
            
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector", "other type")),
    }
}

fn process_track(notes: &[Value]) -> Result<Vec<serde_json::Value>> {
    let mut track_data = Vec::new();
    
    for note in notes {
        if let Some(note_data) = process_single_note(note)? {
            track_data.push(note_data);
        }
    }
    
    Ok(track_data)
}

fn process_single_note(note: &Value) -> Result<Option<serde_json::Value>> {
    match &note.val_type {
        ValueType::Number(frac) => {
            // 分数の場合
            if frac.denominator == BigInt::one() {
                // 単音
                if let Some(freq) = frac.numerator.to_f64() {
                    Ok(Some(json!({
                        "type": "single",
                        "frequency": freq,
                        "duration": "normal"
                    })))
                } else {
                    Ok(None)
                }
            } else {
                // 二和音 - 約分された値を音楽的に有用な周波数に変換
                if let (Some(num), Some(den)) = (
                    frac.numerator.to_f64(),
                    frac.denominator.to_f64()
                ) {
                    let (freq1, freq2) = convert_fraction_to_frequencies(num, den);
                    Ok(Some(json!({
                        "type": "chord",
                        "frequencies": [freq1, freq2],
                        "duration": "normal"
                    })))
                } else {
                    Ok(None)
                }
            }
        },
        ValueType::Boolean(is_long) => {
            // 真偽値は長さ指定のみ（単体では音なし）
            Ok(Some(json!({
                "type": "duration_marker",
                "long": is_long
            })))
        },
        ValueType::Vector(elements, _) if elements.len() == 2 => {
            // [音程 長さ] のペア
            let note_part = &elements[0];
            let duration_part = &elements[1];
            
            let duration = match &duration_part.val_type {
                ValueType::Boolean(true) => "long",
                ValueType::Boolean(false) => "short",
                _ => "normal"
            };
            
            match &note_part.val_type {
                ValueType::Number(frac) => {
                    if frac.denominator == BigInt::one() {
                        // 単音
                        if let Some(freq) = frac.numerator.to_f64() {
                            Ok(Some(json!({
                                "type": "single",
                                "frequency": freq,
                                "duration": duration
                            })))
                        } else {
                            Ok(None)
                        }
                    } else {
                        // 二和音
                        if let (Some(num), Some(den)) = (
                            frac.numerator.to_f64(),
                            frac.denominator.to_f64()
                        ) {
                            let (freq1, freq2) = convert_fraction_to_frequencies(num, den);
                            Ok(Some(json!({
                                "type": "chord",
                                "frequencies": [freq1, freq2],
                                "duration": duration
                            })))
                        } else {
                            Ok(None)
                        }
                    }
                },
                ValueType::Nil => {
                    // 休符
                    Ok(Some(json!({
                        "type": "rest",
                        "duration": duration
                    })))
                },
                _ => Ok(None)
            }
        },
        ValueType::String(_) => {
            // 文字列は上位レベルで処理済み
            Ok(None)
        },
        ValueType::Nil => {
            // 休符
            Ok(Some(json!({
                "type": "rest",
                "duration": "normal"
            })))
        },
        _ => Ok(None)
    }
}

// 約分された分数を音楽的に有用な周波数に変換
fn convert_fraction_to_frequencies(num: f64, den: f64) -> (f64, f64) {
    // 基準周波数（C4 = 261.63Hz）
    let base_freq = 261.63;
    
    // 分数の比率を音楽的な間隔に変換
    let ratio = num / den;
    
    // 比率を適切な周波数範囲にマッピング
    let freq1 = base_freq * num.powf(0.5); // 分子に基づく周波数
    let freq2 = base_freq * den.powf(0.5); // 分母に基づく周波数
    
    // 可聴域に収まるように調整
    let freq1_adjusted = freq1.max(100.0).min(2000.0);
    let freq2_adjusted = freq2.max(100.0).min(2000.0);
    
    (freq1_adjusted, freq2_adjusted)
}
