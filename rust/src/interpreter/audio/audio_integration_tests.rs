//! Integration test suite for `crate::interpreter::audio`.

use crate::interpreter::Interpreter;

#[tokio::test]
async fn test_play_integration() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("[ 440 ] MUSIC@PLAY").await;
    assert!(result.is_ok(), "PLAY should succeed: {:?}", result);

    let output = interp.collect_output();


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


    assert!(
        interp.get_stack().is_empty(),
        "Stack should be empty after .. MUSIC@SEQ MUSIC@PLAY"
    );
}

#[tokio::test]
async fn test_multitrack_play_consumes_all() {

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


    assert!(
        interp.get_stack().is_empty(),
        "Stack should be completely empty after playing 3 tracks"
    );
}

#[tokio::test]
async fn test_play_with_duration() {


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


#[tokio::test]
async fn test_chord_marks_as_simultaneous() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();


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


    let result = interp
        .execute("[ 440 ] [ 0.05 0.1 0.8 0.2 ] MUSIC@ADSR MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "ADSR MUSIC@PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();

    assert!(
        output.contains("AUDIO:"),
        "Should contain AUDIO command"
    );
}

#[tokio::test]
async fn test_adsr_requires_4_elements() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("[ 440 ] [ 0.1 0.2 0.3 ] MUSIC@ADSR").await;
    assert!(result.is_err(), "ADSR with 3 elements should fail");
}

#[tokio::test]
async fn test_adsr_sustain_validation() {

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

    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("[ 440 ] MUSIC@SINE MUSIC@PLAY").await;
    assert!(
        result.is_ok(),
        "SINE MUSIC@PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();

    assert!(
        !output.contains("\"waveform\":\"sine\""),
        "Sine waveform should not be serialized as it's default"
    );
}

#[tokio::test]
async fn test_seq_group_builds_sequential_structure() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute("[ 440 550 ] MUSIC@SEQ-GROUP MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "SEQ-GROUP MUSIC@PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(output.contains("AUDIO:"), "Should contain AUDIO command");
    assert!(
        output.contains("\"type\":\"seq\""),
        "SEQ-GROUP should produce a seq structure: {}",
        output
    );
}

#[tokio::test]
async fn test_sim_group_builds_simultaneous_structure() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute("[ 440 550 ] MUSIC@SIM-GROUP MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "SIM-GROUP MUSIC@PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(output.contains("AUDIO:"), "Should contain AUDIO command");
    assert!(
        output.contains("\"type\":\"sim\""),
        "SIM-GROUP should produce a sim structure: {}",
        output
    );
}

#[tokio::test]
async fn test_chord_builds_simultaneous_group() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

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
        output.contains("\"type\":\"sim\""),
        "CHORD should produce a sim structure: {}",
        output
    );
}

#[tokio::test]
async fn test_seq_group_rejects_empty_vector() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("440 MUSIC@SEQ-GROUP").await;
    assert!(result.is_err(), "SEQ-GROUP on a scalar should fail");
}

#[tokio::test]
async fn test_raw_vector_legacy_playback_preserved() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("[ 440 550 ] MUSIC@PLAY").await;
    assert!(
        result.is_ok(),
        "Legacy raw vector playback should still succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(output.contains("AUDIO:"), "Should contain AUDIO command");
}

#[tokio::test]
async fn test_explain_describes_explicit_group() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute("[ 440 550 ] MUSIC@SEQ-GROUP MUSIC@EXPLAIN")
        .await;
    assert!(
        result.is_ok(),
        "SEQ-GROUP MUSIC@EXPLAIN should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(
        output.contains("Sequential group"),
        "EXPLAIN should describe a sequential group: {}",
        output
    );
    assert!(
        output.contains("explicit MUSIC@SEQ-GROUP"),
        "EXPLAIN should report explicit provenance: {}",
        output
    );
    assert!(
        output.contains("Playback boundary"),
        "EXPLAIN should describe the playback boundary: {}",
        output
    );
}

#[tokio::test]
async fn test_explain_describes_chord_group() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute("[ 440 550 660 ] MUSIC@CHORD MUSIC@EXPLAIN")
        .await;
    assert!(
        result.is_ok(),
        "CHORD MUSIC@EXPLAIN should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(
        output.contains("Chord group"),
        "EXPLAIN should describe a chord group: {}",
        output
    );
    assert!(
        output.contains("explicit MUSIC@CHORD"),
        "EXPLAIN should report chord provenance: {}",
        output
    );
}

#[tokio::test]
async fn test_explain_describes_raw_vector_ambiguity() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("[ 440 550 ] MUSIC@EXPLAIN").await;
    assert!(
        result.is_ok(),
        "Raw vector MUSIC@EXPLAIN should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(
        output.contains("Raw Vector"),
        "EXPLAIN should flag a raw vector: {}",
        output
    );
    assert!(
        output.contains("MUSIC@SEQ-GROUP"),
        "EXPLAIN should suggest explicit constructors: {}",
        output
    );
}

#[tokio::test]
async fn test_combined_chord_adsr_waveform() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();


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

#[tokio::test]
async fn test_hz_dur_note_plays_as_tone() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute("440 MUSIC@HZ 1 MUSIC@DUR MUSIC@NOTE MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "HZ/DUR/NOTE/PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(output.contains("AUDIO:"), "Should contain AUDIO command");
    assert!(
        output.contains("\"type\":\"tone\""),
        "A note should lower to a tone: {}",
        output
    );
}

#[tokio::test]
async fn test_just_intonation_pitch_is_exact() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    // 1980/3 reduces to exactly 660 Hz - a just-intonation perfect fifth
    // above 440 Hz (ratio 3/2), carried as an exact rational.
    let result = interp
        .execute("1980/3 MUSIC@HZ 1 MUSIC@DUR MUSIC@NOTE MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "Just-intonation pitch should play: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(
        output.contains("\"frequency\":660"),
        "1980/3 should resolve to exactly 660 Hz: {}",
        output
    );
}

#[tokio::test]
async fn test_rest_plays_as_rest() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute("1 MUSIC@DUR MUSIC@REST MUSIC@PLAY")
        .await;
    assert!(result.is_ok(), "REST/PLAY should succeed: {:?}", result);

    let output = interp.collect_output();
    assert!(
        output.contains("\"type\":\"rest\""),
        "A music.rest should lower to a rest: {}",
        output
    );
}

#[tokio::test]
async fn test_edo_step_plays_as_tone() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    // Step 12 of 12-EDO anchored at 440 Hz is one octave up (880 Hz).
    let result = interp
        .execute("440 12 MUSIC@EDO 12 MUSIC@STEP 1 MUSIC@DUR MUSIC@NOTE MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "EDO/STEP/NOTE/PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(
        output.contains("\"type\":\"tone\""),
        "An EDO step should lower to a tone: {}",
        output
    );
}

#[tokio::test]
async fn test_edr_non_octave_tuning_plays() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    // 13 equal divisions of the 3/1 tritave (Bohlen-Pierce).
    let result = interp
        .execute("440 [ 3 1 ] 13 MUSIC@EDR 13 MUSIC@STEP 1 MUSIC@DUR MUSIC@NOTE MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "EDR/STEP/NOTE/PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(
        output.contains("\"type\":\"tone\""),
        "A non-octave EDR step should lower to a tone: {}",
        output
    );
}

#[tokio::test]
async fn test_edr_rejects_fractional_divisions() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("440 [ 3 1 ] 13/2 MUSIC@EDR").await;
    assert!(
        result.is_err(),
        "EDR with non-integer divisions should fail"
    );
}

#[tokio::test]
async fn test_note_rejects_raw_scalars() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("440 1 MUSIC@NOTE").await;
    assert!(
        result.is_err(),
        "NOTE on raw scalars (not music.pitch/music.duration) should fail"
    );
}

#[tokio::test]
async fn test_hz_respects_keep_mode() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    // ,, is the KEEP modifier: the operand must remain on the stack.
    let result = interp.execute("440 ,, MUSIC@HZ").await;
    assert!(result.is_ok(), "HZ in KEEP mode should succeed: {:?}", result);

    assert_eq!(
        interp.get_stack().len(),
        2,
        "KEEP mode should leave the operand and push the pitch"
    );
}

#[tokio::test]
async fn test_hz_rejects_stack_target_mode() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    // .. is the whole-stack target modifier, unsupported by constructors.
    let result = interp.execute("440 .. MUSIC@HZ").await;
    assert!(
        result.is_err(),
        "HZ should reject the whole-stack target mode"
    );
}

#[tokio::test]
async fn test_notes_combine_under_stack_play() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute(
            "440 MUSIC@HZ 1 MUSIC@DUR MUSIC@NOTE \
             550 MUSIC@HZ 1 MUSIC@DUR MUSIC@NOTE .. MUSIC@PLAY",
        )
        .await;
    assert!(
        result.is_ok(),
        "Two notes under .. MUSIC@PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(
        output.contains("\"type\":\"seq\"") && output.contains("\"type\":\"tone\""),
        "Stacked notes should combine into a sequence of tones: {}",
        output
    );
}

#[tokio::test]
async fn test_explain_describes_pitch_and_note() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    interp
        .execute("440 MUSIC@HZ MUSIC@EXPLAIN")
        .await
        .unwrap();
    let pitch_output = interp.collect_output();
    assert!(
        pitch_output.contains("Pitch:"),
        "EXPLAIN should describe a pitch: {}",
        pitch_output
    );

    let mut interp2 = Interpreter::new();
    interp2.execute("'music' IMPORT").await.unwrap();
    interp2
        .execute("440 MUSIC@HZ 1 MUSIC@DUR MUSIC@NOTE MUSIC@EXPLAIN")
        .await
        .unwrap();
    let note_output = interp2.collect_output();
    assert!(
        note_output.contains("Note:"),
        "EXPLAIN should describe a note: {}",
        note_output
    );
}

#[tokio::test]
async fn test_raw_tensor_vector_plays_all_children() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("[ 440 550 660 ] MUSIC@PLAY").await;
    assert!(result.is_ok(), "Raw vector PLAY should succeed: {:?}", result);

    let output = interp.collect_output();
    assert!(
        output.contains("\"frequency\":440")
            && output.contains("\"frequency\":550")
            && output.contains("\"frequency\":660"),
        "A raw numeric vector should play every element: {}",
        output
    );
}

#[tokio::test]
async fn test_voice_group_explains_as_voice() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute("[ 440 550 ] MUSIC@VOICE MUSIC@PLAY")
        .await;
    assert!(result.is_ok(), "VOICE PLAY should succeed: {:?}", result);
    let output = interp.collect_output();
    assert!(
        output.contains("\"type\":\"seq\""),
        "A voice should play as a sequence: {}",
        output
    );

    let mut interp2 = Interpreter::new();
    interp2.execute("'music' IMPORT").await.unwrap();
    interp2
        .execute("[ 440 550 ] MUSIC@VOICE MUSIC@EXPLAIN")
        .await
        .unwrap();
    let explain = interp2.collect_output();
    assert!(
        explain.contains("Voice group") && explain.contains("explicit MUSIC@VOICE"),
        "EXPLAIN should describe a voice group: {}",
        explain
    );
}

#[tokio::test]
async fn test_measure_and_phrase_groups() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    interp
        .execute("[ 440 550 ] MUSIC@MEASURE MUSIC@EXPLAIN")
        .await
        .unwrap();
    assert!(
        interp.collect_output().contains("Measure group"),
        "EXPLAIN should describe a measure group"
    );

    interp
        .execute("[ 440 550 ] MUSIC@PHRASE MUSIC@EXPLAIN")
        .await
        .unwrap();
    assert!(
        interp.collect_output().contains("Phrase group"),
        "EXPLAIN should describe a phrase group"
    );

    interp
        .execute("[ 440 550 ] MUSIC@TRACK MUSIC@EXPLAIN")
        .await
        .unwrap();
    assert!(
        interp.collect_output().contains("Track group"),
        "EXPLAIN should describe a track group"
    );
}

#[tokio::test]
async fn test_with_tuning_resolves_bare_integers_as_steps() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    // Step 0 of 12-EDO anchored at 440 Hz is exactly 440 Hz; step 12 is the
    // octave above. Bare integers in the scope are read as tuning steps.
    let result = interp
        .execute("440 12 MUSIC@EDO [ 0 12 ] MUSIC@WITH-TUNING MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "WITH-TUNING PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(output.contains("AUDIO:"), "Should contain AUDIO command");
    assert!(
        output.contains("\"frequency\":440"),
        "Step 0 should resolve to exactly the 440 Hz reference: {}",
        output
    );
}

#[tokio::test]
async fn test_with_tuning_plays_a_full_scale() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp
        .execute("440 12 MUSIC@EDO [ 0 2 4 5 7 9 11 12 ] MUSIC@WITH-TUNING MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "WITH-TUNING scale PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    let tones = output.matches("\"type\":\"tone\"").count();
    assert_eq!(tones, 8, "An 8-step scale should yield 8 tones: {}", output);
}

#[tokio::test]
async fn test_with_tuning_rejects_non_tuning_operand() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("[ 0 2 4 ] [ 0 2 4 ] MUSIC@WITH-TUNING").await;
    assert!(
        result.is_err(),
        "WITH-TUNING without a music.tuning operand should fail"
    );
}

#[tokio::test]
async fn test_explain_describes_tuning_scope() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    interp
        .execute("440 12 MUSIC@EDO [ 0 2 4 ] MUSIC@WITH-TUNING MUSIC@EXPLAIN")
        .await
        .unwrap();
    let output = interp.collect_output();
    assert!(
        output.contains("Tuning scope"),
        "EXPLAIN should describe a tuning scope: {}",
        output
    );
}
