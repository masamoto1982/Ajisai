// rust/src/main.rs (オプション：ローカルテスト用)

use ajisai_core::interpreter::Interpreter;

fn main() {
    let mut interp = Interpreter::new();
    
    println!("=== Ajisai BLOOM Test ===\n");
    
    // テスト1: 基本的な計算
    println!("Test 1: 1 2 +");
    match interp.eval_interactive("1 2 +") {
        Ok(output) => {
            println!("Output: {:?}", output);
            println!("Stack: {:?}\n", interp.stack);
        }
        Err(e) => println!("Error: {}\n", e),
    }
    
    // テスト2: Vectorとして保護
    println!("Test 2: [ 1 2 + ]");
    match interp.eval_interactive("[ 1 2 + ]") {
        Ok(output) => {
            println!("Output: {:?}", output);
            println!("Stack: {:?}", interp.stack);
            println!("(Should remain as vector, not executed)\n");
        }
        Err(e) => println!("Error: {}\n", e),
    }
    
    // テスト3: 明示的なBLOOM
    println!("Test 3: BLOOM");
    match interp.eval_interactive("BLOOM") {
        Ok(output) => {
            println!("Output: {:?}", output);
            println!("Stack: {:?}", interp.stack);
            println!("(Now should be executed to 3)\n");
        }
        Err(e) => println!("Error: {}\n", e),
    }
    
    // スタッククリア
    interp.stack.clear();
    
    // テスト4: ガード節
    println!("Test 4: [ 5 ] DUP 0 > : [ 'positive' ] : [ 'negative' ]");
    match interp.eval_interactive("[ 5 ] DUP 0 > : [ 'positive' ] : [ 'negative' ]") {
        Ok(output) => {
            println!("Output: {:?}", output);
            println!("Stack: {:?}\n", interp.stack);
        }
        Err(e) => println!("Error: {}\n", e),
    }
    
    // スタッククリア
    interp.stack.clear();
    
    // テスト5: カスタムワード定義
    println!("Test 5: [ 1 + ] 'INC' DEF");
    match interp.eval_interactive("[ 1 + ] 'INC' DEF") {
        Ok(output) => {
            println!("Output: {:?}", output);
            println!("Dictionary contains INC: {}\n", interp.dictionary.contains_key("INC"));
        }
        Err(e) => println!("Error: {}\n", e),
    }
    
    // テスト6: カスタムワード使用
    println!("Test 6: [ 5 ] INC");
    match interp.eval_interactive("[ 5 ] INC") {
        Ok(output) => {
            println!("Output: {:?}", output);
            println!("Stack: {:?}\n", interp.stack);
        }
        Err(e) => println!("Error: {}\n", e),
    }
}
