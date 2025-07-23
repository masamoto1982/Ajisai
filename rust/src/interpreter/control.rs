use std::rc::Rc;

use crate::{
    interpreter::Interpreter,
    types::{Token, Type, Word},
};

pub fn op_def(interpreter: &mut Interpreter) -> Result<(), String> {
    let name = interpreter.stack.pop_string()?;
    let episode_vec = interpreter.stack.pop_vector()?;
    let mut tokens: Vec<Token> = Vec::new();

    for data in episode_vec.borrow().iter() {
        let token = match data {
            Type::Number(n) => Token::Number(n.as_ref().clone()),
            Type::String(s) => Token::String(s.as_ref().clone()),
            Type::Symbol(s) => Token::Word(s.as_ref().clone()),
            Type::Vector(_) => {
                return Err(
                    "Defining vectors inside a user-defined word is not yet supported.".to_string(),
                );
            }
            _ => return Err(format!("Cannot define a word from type {:?}", data)),
        };
        tokens.push(token);
    }

    let word = Word::UserDefined(Rc::new(tokens));
    interpreter.dictionary.insert(name, Rc::new(word));

    Ok(())
}

pub fn op_if(interpreter: &mut Interpreter) -> Result<(), String> {
    let false_branch = interpreter.stack.pop_quotation()?;
    let true_branch = interpreter.stack.pop_quotation()?;
    let cond = interpreter.stack.pop_bool()?;

    if cond {
        interpreter.run_tokens(true_branch);
    } else {
        interpreter.run_tokens(false_branch);
    }
    Ok(())
}

pub fn op_call(interpreter: &mut Interpreter) -> Result<(), String> {
    let quot = interpreter.stack.pop_quotation()?;
    interpreter.run_tokens(quot);
    Ok(())
}
