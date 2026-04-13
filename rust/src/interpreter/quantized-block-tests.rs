use crate::interpreter::quantized_block::{is_quantizable_block, quantize_code_block};
use crate::interpreter::Interpreter;
use crate::types::Token;

#[test]
fn quantizes_simple_block() {
    let mut interp = Interpreter::new();
    let tokens = vec![Token::Number("1".into()), Token::Symbol("+".into())];
    assert!(is_quantizable_block(&tokens));
    assert!(quantize_code_block(&tokens, &mut interp).is_some());
}
