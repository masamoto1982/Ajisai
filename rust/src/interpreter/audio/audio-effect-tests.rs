

use crate::interpreter::Interpreter;


#[tokio::test]
async fn test_slot_basic() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("[ 0.25 ] MUSIC@SLOT").await;
    assert!(result.is_ok(), "SLOT should succeed: {:?}", result);

    let output = interp.collect_output();
    assert!(output.contains("CONFIG:"), "Should contain CONFIG command");
    assert!(
        output.contains("\"slot_duration\":0.25"),
        "Should set duration to 0.25"
    );
}

#[tokio::test]
async fn test_slot_fraction() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("[ 1/4 ] MUSIC@SLOT").await;
    assert!(
        result.is_ok(),
        "SLOT with fraction should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(
        output.contains("\"slot_duration\":0.25"),
        "1/4 should become 0.25"
    );
}

#[tokio::test]
async fn test_slot_negative_error() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("[ 0 ] [ 0.5 ] - MUSIC@SLOT").await;
    assert!(result.is_err(), "Negative MUSIC@SLOT should fail");
}

#[tokio::test]
async fn test_slot_zero_error() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("[ 0 ] MUSIC@SLOT").await;
    assert!(result.is_err(), "Zero MUSIC@SLOT should fail");
}

#[tokio::test]
async fn test_slot_warning_very_short() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("[ 0.005 ] MUSIC@SLOT").await;
    assert!(
        result.is_ok(),
        "Very short MUSIC@SLOT should succeed with warning"
    );

    let output = interp.collect_output();
    assert!(
        output.contains("Warning:"),
        "Should contain warning for very short duration"
    );
    assert!(
        output.contains("very short"),
        "Warning should mention 'very short'"
    );
}

#[tokio::test]
async fn test_slot_warning_very_long() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("[ 15 ] MUSIC@SLOT").await;
    assert!(
        result.is_ok(),
        "Very long MUSIC@SLOT should succeed with warning"
    );

    let output = interp.collect_output();
    assert!(
        output.contains("Warning:"),
        "Should contain warning for very long duration"
    );
    assert!(
        output.contains("very long"),
        "Warning should mention 'very long'"
    );
}

#[tokio::test]
async fn test_slot_one_second() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("[ 1 ] MUSIC@SLOT").await;
    assert!(result.is_ok(), "1 MUSIC@SLOT should succeed: {:?}", result);

    let output = interp.collect_output();
    assert!(output.contains("CONFIG:"), "Should contain CONFIG command");
    assert!(
        output.contains("\"slot_duration\":1"),
        "Should set duration to 1"
    );
}

#[tokio::test]
async fn test_chord_can_be_defined_and_reused() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();


    let result = interp
        .execute(": [ 440 550 660 ] MUSIC@CHORD ; 'C_MAJOR' DEF")
        .await;
    assert!(result.is_ok(), "DEF should succeed: {:?}", result);


    let result = interp.execute("C_MAJOR MUSIC@PLAY").await;
    assert!(
        result.is_ok(),
        "C_MAJOR MUSIC@PLAY should succeed: {:?}",
        result
    );

    let output = interp.collect_output();

    assert!(
        output.contains("AUDIO:"),
        "Should contain AUDIO command"
    );
}


#[tokio::test]
async fn test_gain_basic() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("0.5 MUSIC@GAIN").await;
    assert!(result.is_ok(), "GAIN should succeed: {:?}", result);

    let output = interp.collect_output();
    assert!(output.contains("EFFECT:"), "Should contain EFFECT command");
    assert!(output.contains("\"gain\":0.5"), "Should set gain to 0.5");
}

#[tokio::test]
async fn test_gain_clamp_high() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("1.5 MUSIC@GAIN").await;
    assert!(result.is_ok(), "GAIN should succeed with clamping");

    let output = interp.collect_output();
    assert!(output.contains("\"gain\":1"), "Should clamp to 1.0");
    assert!(output.contains("Warning:"), "Should warn about clamping");
}

#[tokio::test]
async fn test_gain_clamp_low() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("[ 0 ] [ 0.5 ] - MUSIC@GAIN").await;
    assert!(result.is_ok(), "GAIN should succeed with clamping");

    let output = interp.collect_output();
    assert!(output.contains("\"gain\":0"), "Should clamp to 0.0");
}

#[tokio::test]
async fn test_gain_fraction() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("1/2 MUSIC@GAIN").await;
    assert!(result.is_ok(), "GAIN with fraction should succeed");

    let output = interp.collect_output();
    assert!(output.contains("\"gain\":0.5"), "1/2 should become 0.5");
}

#[tokio::test]
async fn test_gain_reset() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("MUSIC@GAIN-RESET").await;
    assert!(result.is_ok(), "GAIN-RESET should succeed");

    let output = interp.collect_output();
    assert!(output.contains("\"gain\":1"), "Should reset to 1.0");
}

#[tokio::test]
async fn test_pan_basic() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();

    let result = interp.execute("[ 0 ] [ 0.5 ] - MUSIC@PAN").await;
    assert!(result.is_ok(), "PAN should succeed");

    let output = interp.collect_output();
    assert!(output.contains("EFFECT:"), "Should contain EFFECT command");
    assert!(output.contains("\"pan\":-0.5"), "Should set pan to -0.5");
}

#[tokio::test]
async fn test_pan_clamp() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("2 MUSIC@PAN").await;
    assert!(result.is_ok(), "PAN should succeed with clamping");

    let output = interp.collect_output();
    assert!(output.contains("\"pan\":1"), "Should clamp to 1.0");
    assert!(output.contains("Warning:"), "Should warn about clamping");
}

#[tokio::test]
async fn test_pan_reset() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("MUSIC@PAN-RESET").await;
    assert!(result.is_ok(), "PAN-RESET should succeed");

    let output = interp.collect_output();
    assert!(output.contains("\"pan\":0"), "Should reset to 0.0");
}

#[tokio::test]
async fn test_fx_reset() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("MUSIC@FX-RESET").await;
    assert!(result.is_ok(), "FX-RESET should succeed");

    let output = interp.collect_output();
    assert!(output.contains("\"gain\":1"), "Should reset gain to 1.0");
    assert!(output.contains("\"pan\":0"), "Should reset pan to 0.0");
}

#[tokio::test]
async fn test_gain_then_play() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("0.5 MUSIC@GAIN [ 440 ] MUSIC@PLAY").await;
    assert!(result.is_ok(), "GAIN then MUSIC@PLAY should succeed");

    let output = interp.collect_output();
    assert!(output.contains("EFFECT:"), "Should have EFFECT command");
    assert!(output.contains("AUDIO:"), "Should have AUDIO command");
}

#[tokio::test]
async fn test_combined_gain_pan_play() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp
        .execute("0.5 MUSIC@GAIN 0.7 MUSIC@PAN [ 440 ] MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "Combined MUSIC@GAIN MUSIC@PAN MUSIC@PLAY should succeed"
    );

    let output = interp.collect_output();
    assert!(output.contains("\"gain\":0.5"), "Should have gain 0.5");
    assert!(output.contains("\"pan\":0.7"), "Should have pan 0.7");
    assert!(output.contains("AUDIO:"), "Should have AUDIO command");
}


#[tokio::test]
async fn test_play_with_lyrics() {
    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();


    let result = interp
        .execute("[ 440/2 'Hello' 550/2 'World' ] MUSIC@PLAY")
        .await;
    assert!(
        result.is_ok(),
        "PLAY with lyrics should succeed: {:?}",
        result
    );

    let output = interp.collect_output();

    assert!(
        output.contains("AUDIO:"),
        "Should contain AUDIO command"
    );
}

#[tokio::test]
async fn test_play_with_duration_unreduced() {


    let mut interp = Interpreter::new();
    interp.execute("'music' IMPORT").await.unwrap();
    let result = interp.execute("[ 440/2 550/2 ] MUSIC@PLAY").await;
    assert!(
        result.is_ok(),
        "PLAY with unreduced fractions should succeed: {:?}",
        result
    );

    let output = interp.collect_output();
    assert!(
        output.contains("AUDIO:"),
        "Should contain AUDIO command"
    );
}
