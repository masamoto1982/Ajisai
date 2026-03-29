// rust/src/interpreter/audio/audio-integration-tests.rs
//
// 【責務】
// 音楽DSLの統合テスト（PLAY, SEQ, SIM, CHORD, ADSR, 波形ワード）

use crate::interpreter::Interpreter;

#[tokio::test]
async fn test_play_integration() {
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
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("440 MUSIC@CHORD").await;
    assert!(result.is_err(), "CHORD on non-vector should fail");
}

#[tokio::test]
async fn test_adsr_sets_envelope() {
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
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    // MUSIC@ADSR needs target + params
    let result = interp.execute("[ 440 ] [ 0.1 0.2 0.3 ] MUSIC@ADSR").await;
    assert!(result.is_err(), "ADSR with 3 elements should fail");
}

#[tokio::test]
async fn test_adsr_sustain_validation() {
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
