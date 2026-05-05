use crate::error::{AjisaiError, Result};
use crate::interpreter::comptime::policy::assert_comptime_safe_tokens;
use crate::interpreter::comptime::sandbox::run_precompute_block;
use crate::interpreter::comptime::value_to_tokens::stack_to_literal_tokens;
use crate::interpreter::Interpreter;
use crate::types::Token;
use std::collections::HashSet;

fn extract_block(tokens: &[Token], start: usize) -> Result<(Vec<Token>, usize)> {
    let mut depth = 0usize;
    let mut out = Vec::new();
    let mut i = start;
    while i < tokens.len() {
        match &tokens[i] {
            Token::BlockStart => {
                depth += 1;
                out.push(tokens[i].clone());
            }
            Token::BlockEnd => {
                if depth == 0 {
                    return Err(AjisaiError::from("PRECOMPUTE rejected: malformed block"));
                }
                depth -= 1;
                out.push(tokens[i].clone());
                if depth == 0 {
                    return Ok((out, i));
                }
            }
            _ => out.push(tokens[i].clone()),
        }
        i += 1;
    }
    Err(AjisaiError::from("PRECOMPUTE rejected: unterminated block"))
}

pub(crate) fn precompute_definition_tokens(
    interp: &mut Interpreter,
    tokens: &[Token],
) -> Result<Vec<Token>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        if matches!(tokens[i], Token::BlockStart) {
            let (block_tokens, block_end) = extract_block(tokens, i)?;
            if matches!(tokens.get(block_end + 1), Some(Token::Symbol(s)) if s.eq_ignore_ascii_case("PRECOMPUTE"))
            {
                let block_body_tokens = &block_tokens[1..block_tokens.len() - 1];
                assert_comptime_safe_tokens(interp, block_body_tokens, &mut HashSet::new())?;
                let stack = run_precompute_block(interp, block_body_tokens)?;
                out.extend(stack_to_literal_tokens(&stack)?);
                i = block_end + 2;
                continue;
            }
        }
        out.push(tokens[i].clone());
        i += 1;
    }

    Ok(out)
}
