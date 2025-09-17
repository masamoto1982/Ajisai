// rust/src/parser.rs - 逆ポーランド記法S式対応版

use crate::types::{Expression, RepeatSpec, TimeSpec, Token, Fraction};
use web_sys::console;
use wasm_bindgen::JsValue;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, position: 0 }
    }
    
    pub fn parse(&mut self) -> Result<Vec<Expression>, String> {
        console::log_1(&JsValue::from_str("=== parse RPN S-expressions ==="));
        let mut expressions = Vec::new();
        
        while self.position < self.tokens.len() {
            if self.is_at_end() {
                break;
            }
            expressions.push(self.parse_expression()?);
        }
        
        Ok(expressions)
    }
    
    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }
    
    fn current_token(&self) -> Option<&Token> {
        if self.position < self.tokens.len() {
            Some(&self.tokens[self.position])
        } else {
            None
        }
    }
    
    fn parse_expression(&mut self) -> Result<Expression, String> {
        if self.is_at_end() {
            return Err("Unexpected end of input".to_string());
        }
        
        let token = self.tokens[self.position].clone();
        match token {
            Token::VectorStart => self.parse_vector_or_rpn(),
            Token::Number(s) => {
                self.position += 1;
                Ok(Expression::Number(Fraction::from_str(&s)?))
            },
            Token::String(s) => {
                self.position += 1;
                Ok(Expression::String(s))
            },
            Token::Boolean(b) => {
                self.position += 1;
                Ok(Expression::Boolean(b))
            },
            Token::Nil => {
                self.position += 1;
                Ok(Expression::Nil)
            },
            Token::Symbol(s) => {
                self.position += 1;
                Ok(Expression::Symbol(s))
            },
            Token::RepeatUnit(r) => {
                self.position += 1;
                Ok(Expression::Symbol(format!("{}", r)))
            },
            Token::TimeUnit(t) => {
                self.position += 1;
                Ok(Expression::Symbol(format!("{}", t)))
            },
            _ => Err(format!("Unexpected token: {:?}", token)),
        }
    }
    
    fn parse_vector_or_rpn(&mut self) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_vector_or_rpn ==="));
        
        // '[' をスキップ
        self.position += 1;
        
        // 空のベクトル
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
            return Ok(Expression::Vector(vec![]));
        }
        
        // 要素を収集
        let mut elements = Vec::new();
        while !self.is_at_end() && !matches!(self.current_token(), Some(Token::VectorEnd)) {
            elements.push(self.parse_expression()?);
        }
        
        // ']' をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        // 逆ポーランド記法の判定: 最後がシンボルなら演算
        if let Some(Expression::Symbol(action)) = elements.last() {
            console::log_1(&JsValue::from_str(&format!("RPN operation detected: {}", action)));
            
            let args = elements[..elements.len()-1].to_vec();
            
            // 特殊制御構造の処理
            match action.as_str() {
                "IF" => self.parse_rpn_if(args),
                "REPEAT" => self.parse_rpn_repeat(args),
                "DELAY" => self.parse_rpn_delay(args),
                "DEF" => self.parse_rpn_def(args),
                "DEL" => self.parse_rpn_del(args),
                "LINE" => self.parse_rpn_line(args),
                _ => {
                    // 通常のS式
                    Ok(Expression::SExpression {
                        action: Box::new(Expression::Symbol(action.clone())),
                        args,
                    })
                }
            }
        } else {
            // データベクトル
            console::log_1(&JsValue::from_str("Data vector detected"));
            Ok(Expression::Vector(elements))
        }
    }
    
    fn parse_rpn_if(&self, args: Vec<Expression>) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_rpn_if ==="));
        
        match args.len() {
            2 => {
                // [ condition then-branch IF ]
                Ok(Expression::If {
                    condition: Box::new(args[0].clone()),
                    then_branch: Box::new(args[1].clone()),
                    else_branch: None,
                })
            },
            3 => {
                // [ condition then-branch else-branch IF ]
                Ok(Expression::If {
                    condition: Box::new(args[0].clone()),
                    then_branch: Box::new(args[1].clone()),
                    else_branch: Some(Box::new(args[2].clone())),
                })
            },
            _ => Err("IF requires 2 or 3 arguments".to_string()),
        }
    }
    
    fn parse_rpn_repeat(&self, args: Vec<Expression>) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_rpn_repeat ==="));
        
        if args.len() != 2 {
            return Err("REPEAT requires exactly 2 arguments".to_string());
        }
        
        // [ repeat-spec body REPEAT ]
        let spec = self.parse_repeat_spec_from_expression(&args[0])?;
        
        Ok(Expression::Repeat {
            spec,
            body: Box::new(args[1].clone()),
        })
    }
    
    fn parse_rpn_delay(&self, args: Vec<Expression>) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_rpn_delay ==="));
        
        if args.len() != 2 {
            return Err("DELAY requires exactly 2 arguments".to_string());
        }
        
        // [ time-spec body DELAY ]
        let spec = self.parse_time_spec_from_expression(&args[0])?;
        
        Ok(Expression::Delay {
            spec,
            body: Box::new(args[1].clone()),
        })
    }
    
    fn parse_rpn_def(&self, args: Vec<Expression>) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_rpn_def ==="));
        
        if args.len() != 2 {
            return Err("DEF requires exactly 2 arguments".to_string());
        }
        
        // [ word-name lines DEF ]
        let word_name = match &args[0] {
            Expression::Symbol(name) => name.clone(),
            Expression::Vector(v) if v.len() == 1 => {
                match &v[0] {
                    Expression::Symbol(name) => name.clone(),
                    _ => return Err("Word name must be a symbol".to_string()),
                }
            },
            _ => return Err("Word name must be a symbol or [symbol]".to_string()),
        };
        
        Ok(Expression::SExpression {
            action: Box::new(Expression::Symbol("DEF".to_string())),
            args: vec![Expression::Symbol(word_name), args[1].clone()],
        })
    }
    
    fn parse_rpn_del(&self, args: Vec<Expression>) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_rpn_del ==="));
        
        if args.len() != 1 {
            return Err("DEL requires exactly 1 argument".to_string());
        }
        
        // [ word-name DEL ]
        let word_name = match &args[0] {
            Expression::Symbol(name) => name.clone(),
            Expression::Vector(v) if v.len() == 1 => {
                match &v[0] {
                    Expression::Symbol(name) => name.clone(),
                    _ => return Err("Word name must be a symbol".to_string()),
                }
            },
            _ => return Err("Word name must be a symbol or [symbol]".to_string()),
        };
        
        Ok(Expression::SExpression {
            action: Box::new(Expression::Symbol("DEL".to_string())),
            args: vec![Expression::Symbol(word_name)],
        })
    }
    
    fn parse_rpn_line(&self, args: Vec<Expression>) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_rpn_line ==="));
        
        if args.len() < 3 {
            return Err("LINE requires at least 3 arguments".to_string());
        }
        
        // [ repeat timing condition action LINE ] or [ repeat timing action LINE ]
        let repeat = self.parse_repeat_spec_from_expression(&args[0])?;
        let timing = self.parse_time_spec_from_expression(&args[1])?;
        
        let (condition, action) = if args.len() == 4 {
            // 条件あり
            (Some(Box::new(args[2].clone())), Box::new(args[3].clone()))
        } else if args.len() == 3 {
            // 条件なし
            (None, Box::new(args[2].clone()))
        } else {
            return Err("LINE requires 3 or 4 arguments".to_string());
        };
        
        Ok(Expression::Line {
            repeat,
            timing,
            condition,
            action,
        })
    }
    
    fn parse_repeat_spec_from_expression(&self, expr: &Expression) -> Result<RepeatSpec, String> {
        match expr {
            Expression::Symbol(s) => match s.as_str() {
                "FOREVER" => Ok(RepeatSpec::Forever),
                "ONCE" => Ok(RepeatSpec::Once),
                s if s.ends_with('x') => {
                    let num_str = &s[..s.len()-1];
                    let n = num_str.parse::<u32>()
                        .map_err(|_| "Invalid repeat count".to_string())?;
                    Ok(RepeatSpec::Times(n))
                },
                _ => Ok(RepeatSpec::Once),
            },
            Expression::Vector(v) if v.len() == 1 => {
                self.parse_repeat_spec_from_expression(&v[0])
            },
            Expression::Vector(v) if v.is_empty() => Ok(RepeatSpec::Once),
            _ => Ok(RepeatSpec::Once),
        }
    }
    
    fn parse_time_spec_from_expression(&self, expr: &Expression) -> Result<TimeSpec, String> {
        match expr {
            Expression::Symbol(s) => {
                if s.ends_with('s') && !s.ends_with("ms") {
                    let num_str = &s[..s.len()-1];
                    let seconds = num_str.parse::<f64>()
                        .map_err(|_| "Invalid time specification".to_string())?;
                    Ok(TimeSpec::Seconds(seconds))
                } else if s.ends_with("ms") {
                    let num_str = &s[..s.len()-2];
                    let ms = num_str.parse::<u32>()
                        .map_err(|_| "Invalid millisecond specification".to_string())?;
                    Ok(TimeSpec::Milliseconds(ms))
                } else {
                    Ok(TimeSpec::Immediate)
                }
            },
            Expression::Vector(v) if v.len() == 1 => {
                self.parse_time_spec_from_expression(&v[0])
            },
            Expression::Vector(v) if v.is_empty() => Ok(TimeSpec::Immediate),
            _ => Ok(TimeSpec::Immediate),
        }
    }
}
