//! Phase-1 contract tests for the `SERIAL` module. Hardware is never exercised;
//! these assert the emitted `SERIAL:` command lines and the misuse error paths.

use crate::interpreter::Interpreter;

async fn run(code: &str) -> (Result<(), crate::error::AjisaiError>, String) {
    let mut interp = Interpreter::new();
    interp.execute("'serial' IMPORT").await.unwrap();
    let result = interp.execute(code).await;
    let output = interp.collect_output();
    (result, output)
}

#[tokio::test]
async fn open_emits_command_and_keeps_handle() {
    let (result, output) = run("'COM3' SERIAL@OPEN").await;
    assert!(result.is_ok(), "OPEN should succeed: {:?}", result);
    assert!(output.contains("SERIAL:"), "should emit a SERIAL command");
    assert!(output.contains("\"op\":\"open\""), "op should be open");
    assert!(
        output.contains("\"portId\":\"COM3\""),
        "portId should round-trip"
    );
}

#[tokio::test]
async fn configure_emits_baud_rate() {
    let (result, output) = run("'COM3' SERIAL@OPEN 115200 SERIAL@CONFIGURE").await;
    assert!(result.is_ok(), "CONFIGURE should succeed: {:?}", result);
    assert!(
        output.contains("\"op\":\"configure\""),
        "op should be configure"
    );
    assert!(
        output.contains("\"baudRate\":115200"),
        "baud rate should be emitted"
    );
}

#[tokio::test]
async fn write_emits_byte_array() {
    let (result, output) = run("'COM3' SERIAL@OPEN [ 72 73 ] SERIAL@WRITE").await;
    assert!(result.is_ok(), "WRITE should succeed: {:?}", result);
    assert!(output.contains("\"op\":\"write\""), "op should be write");
    assert!(
        output.contains("\"bytes\":[72,73]"),
        "bytes should be emitted as array"
    );
}

#[tokio::test]
async fn flush_then_close_emit_commands() {
    let (result, output) = run("'COM3' SERIAL@OPEN SERIAL@FLUSH SERIAL@CLOSE").await;
    assert!(result.is_ok(), "FLUSH/CLOSE should succeed: {:?}", result);
    assert!(
        output.contains("\"op\":\"flush\""),
        "op should include flush"
    );
    assert!(
        output.contains("\"op\":\"close\""),
        "op should include close"
    );
}

#[tokio::test]
async fn list_ports_emits_query() {
    let (result, output) = run("SERIAL@LIST-PORTS").await;
    assert!(result.is_ok(), "LIST-PORTS should succeed: {:?}", result);
    assert!(
        output.contains("\"op\":\"listPorts\""),
        "should emit listPorts query"
    );
}

#[tokio::test]
async fn write_rejects_out_of_range_byte() {
    let (result, _) = run("'COM3' SERIAL@OPEN [ 999 ] SERIAL@WRITE").await;
    assert!(
        result.is_err(),
        "byte 999 is out of range and must error (misuse)"
    );
}

#[tokio::test]
async fn open_rejects_non_text_port_id() {
    let (result, _) = run("42 SERIAL@OPEN").await;
    assert!(result.is_err(), "numeric port-id must error (misuse)");
}

#[tokio::test]
async fn open_underflow_errors() {
    let (result, _) = run("SERIAL@OPEN").await;
    assert!(result.is_err(), "OPEN with empty stack must underflow");
}
