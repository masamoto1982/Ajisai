

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
