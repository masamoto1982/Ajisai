// rust/src/bin/test_runner.rs

#[cfg(feature = "testing")]
fn main() {
    println!("Running Ajisai language tests...");
    
    // 実際のテストは `cargo test` で実行されるため、
    // ここはテスト結果の確認用
    println!("Use 'cargo test' to run the actual tests.");
}

#[cfg(not(feature = "testing"))]
fn main() {
    println!("Testing feature is not enabled.");
}
