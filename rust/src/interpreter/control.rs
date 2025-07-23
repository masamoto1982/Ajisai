use std::rc::Rc;

use crate::{
    interpreter::Interpreter,
    types::{Token, Type, Word},
};

// 「記憶の固定化」を行うDEFワードの新しい実装
pub fn op_def(interpreter: &mut Interpreter) -> Result<(), String> {
    // 1. スタックから名前(String)とエピソード記憶(Vector)をポップ
    let name = interpreter.stack.pop_string()?;
    let episode_vec = interpreter.stack.pop_vector()?;

    let mut tokens: Vec<Token> = Vec::new();

    // 2. Vector(データ)をToken(コード)に変換（コンパイル）
    for data in episode_vec.borrow().iter() {
        let token = match data {
            Type::Number(n) => Token::Number(n.as_ref().clone()),
            Type::String(s) => Token::String(s.as_ref().clone()),
            Type::Symbol(s) => Token::Word(s.as_ref().clone()), // SymbolをWord Tokenに変換
            Type::Vector(v) => {
                 // Vectorリテラルをコード内で再現するために、
                 // Vectorをスタックに積むための命令列を生成する。
                tokens.push(Token::VectorStart);
                // ネストしたVectorの中身も再帰的にTokenに変換する必要があるが、
                // この実装をシンプルに保つため、ここではサポートしない。
                // 実際には、この部分で再帰的な変換ロジックが必要になる。
                 if !v.borrow().is_empty() {
                    return Err("Defining non-empty vectors inside a user-defined word is not yet supported.".to_string());
                 }
                tokens.push(Token::VectorEnd);
                continue; // 次のデータへ
            }
            _ => return Err(format!("Cannot define a word from type {:?}", data)),
        };
        tokens.push(token);
    }

    // 3. 変換したトークン列を新しいワードとして辞書に登録
    let word = Word::UserDefined(Rc::new(tokens));
    interpreter.dictionary.insert(name, Rc::new(word));

    Ok(())
}

pub fn op_if(interpreter: &mut Interpreter) -> Result<(), String> {
    let false_branch = interpreter.stack.pop_quotation()?;
    let true_branch = interpreter.stack.pop_quototation()?;
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
