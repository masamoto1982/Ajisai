use crate::interpreter::{Interpreter, error::{LPLError, Result}};
use crate::types::ValueType;

pub fn op_dot(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    interp.append_output(&format!("{}", val));
    Ok(())
}

pub fn op_print(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.last()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    interp.append_output(&format!("{} ", val));
    Ok(())
}

pub fn op_cr(interp: &mut Interpreter) -> Result<()> {
    interp.append_output("\n");
    Ok(())
}

pub fn op_space(interp: &mut Interpreter) -> Result<()> {
    interp.append_output(" ");
    Ok(())
}

pub fn op_spaces(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    match val.val_type {
        ValueType::Number(n) => {
            if n.denominator == 1 && n.numerator >= 0 {
                interp.append_output(&" ".repeat(n.numerator as usize));
                Ok(())
            } else {
                Err(LPLError::from("SPACES requires a non-negative integer"))
            }
        },
        _ => Err(LPLError::type_error("number", "other type")),
    }
}

pub fn op_emit(interp: &mut Interpreter) -> Result<()> {
    let val = interp.bookshelf.pop()
        .ok_or(LPLError::BookshelfUnderflow)?;
    
    match val.val_type {
        ValueType::Number(n) => {
            if n.denominator == 1 && n.numerator >= 0 && n.numerator <= 255 {
                interp.append_output(&(n.numerator as u8 as char).to_string());
                Ok(())
            } else {
                Err(LPLError::from("EMIT requires an integer between 0 and 255"))
            }
        },
        _ => Err(LPLError::type_error("number", "other type")),
    }
}
