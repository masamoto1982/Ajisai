// rust/src/interpreter/audio/audio-unit-tests.rs
//
// 【責務】
// 音楽DSLの単体テスト（build_audio_structure のユニットテスト）

use super::build_audio_structure::build_audio_structure;
use super::audio_types::{AudioStructure, PlayMode};
use crate::types::fraction::Fraction;
use crate::types::Value;
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
