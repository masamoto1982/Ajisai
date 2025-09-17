// rust/src/parser.rs - 統一S式パーサー

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
        console::log_1(&JsValue::from_str("=== parse S-expressions ==="));
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
            Token::VectorStart => self.parse_s_expression(),
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
    
    fn parse_s_expression(&mut self) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_s_expression ==="));
        
        // Skip '['
        self.position += 1;
        
        // 空のベクトル
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
            return Ok(Expression::Vector(vec![]));
        }
        
        // 最初の要素を確認
        if self.is_at_end() {
            return Err("Unexpected end in S-expression".to_string());
        }
        
        let first = self.parse_expression()?;
        
        // 特殊フォームのチェック
        match &first {
            Expression::Symbol(s) => match s.as_str() {
                // 制御構造
                "REPEAT" => self.parse_repeat(),
                "DELAY" => self.parse_delay(),
                "IF" => self.parse_if(),
                
                // ワード定義
                "DEF" => self.parse_def(),
                "DEL" => self.parse_del(),
                "LINE" => self.parse_line(),
                
                // アクション
                _ => self.parse_action(first),
            },
            _ => {
                // データベクトルとして扱う
                self.parse_data_vector(first)
            }
        }
    }
    
    fn parse_action(&mut self, action: Expression) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str(&format!("=== parse_action: {:?} ===", action)));
        
        let mut args = Vec::new();
        
        // 引数を収集
        while !self.is_at_end() && !matches!(self.current_token(), Some(Token::VectorEnd)) {
            args.push(self.parse_expression()?);
        }
        
        // ']'をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        Ok(Expression::SExpression {
            action: Box::new(action),
            args,
        })
    }
    
    fn parse_repeat(&mut self) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_repeat ==="));
        
        // [3x] などの反復仕様を解析
        let spec = self.parse_repeat_spec()?;
        
        // ボディを解析
        let body = self.parse_expression()?;
        
        // ']'をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        Ok(Expression::Repeat {
            spec,
            body: Box::new(body),
        })
    }
    
    fn parse_repeat_spec(&mut self) -> Result<RepeatSpec, String> {
        // [ 3x ] や [ FOREVER ] などを解析
        if self.is_at_end() {
            return Ok(RepeatSpec::Once);
        }
        
        // '['をスキップ
        if matches!(self.current_token(), Some(Token::VectorStart)) {
            self.position += 1;
        }
        
        // 空の場合
        if matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
            return Ok(RepeatSpec::Once);
        }
        
        let spec = match self.current_token() {
            Some(Token::RepeatUnit(r)) => {
                self.position += 1;
                match r {
                    crate::types::RepeatControl::Times(n) => RepeatSpec::Times(*n),
                    crate::types::RepeatControl::Forever => RepeatSpec::Forever,
                    _ => RepeatSpec::Once,
                }
            },
            Some(Token::Symbol(s)) if s == "FOREVER" => {
                self.position += 1;
                RepeatSpec::Forever
            },
            _ => RepeatSpec::Once,
        };
        
        // ']'をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        Ok(spec)
    }
    
    fn parse_delay(&mut self) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_delay ==="));
        
        // [2s] などの時間仕様を解析
        let spec = self.parse_time_spec()?;
        
        // ボディを解析
        let body = self.parse_expression()?;
        
        // ']'をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        Ok(Expression::Delay {
            spec,
            body: Box::new(body),
        })
    }
    
    fn parse_time_spec(&mut self) -> Result<TimeSpec, String> {
        // [ 2s ] や [ 500ms ] などを解析
        if self.is_at_end() {
            return Ok(TimeSpec::Immediate);
        }
        
        // '['をスキップ
        if matches!(self.current_token(), Some(Token::VectorStart)) {
            self.position += 1;
        }
        
        // 空の場合
        if matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
            return Ok(TimeSpec::Immediate);
        }
        
        let spec = match self.current_token() {
            Some(Token::TimeUnit(t)) => {
                self.position += 1;
                match t {
                    crate::types::TimeControl::Seconds(s) => TimeSpec::Seconds(*s),
                    crate::types::TimeControl::Milliseconds(ms) => TimeSpec::Milliseconds(*ms),
                    _ => TimeSpec::Immediate,
                }
            },
            _ => {
                // 空の配列
                if !matches!(self.current_token(), Some(Token::VectorEnd)) {
                    self.position += 1;  // 何かあれば読み飛ばす
                }
                TimeSpec::Immediate
            },
        };
        
        // ']'をスキップ  
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        Ok(spec)
    }
    
    fn parse_if(&mut self) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_if ==="));
        
        // 条件
        let condition = self.parse_expression()?;
        
        // then節
        let then_branch = self.parse_expression()?;
        
        // else節（オプション）
        let else_branch = if !self.is_at_end() && 
                           !matches!(self.current_token(), Some(Token::VectorEnd)) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        
        // ']'をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        Ok(Expression::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
        })
    }
    
    fn parse_def(&mut self) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_def ==="));
        
        // [ WORD_NAME ]
        let word_name = self.parse_word_name()?;
        
        // LINE構文の配列
        let mut lines = Vec::new();
        while !self.is_at_end() && !matches!(self.current_token(), Some(Token::VectorEnd)) {
            let line_expr = self.parse_expression()?;
            lines.push(line_expr);
        }
        
        // ']'をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        // DEF専用の式を返す
        Ok(Expression::SExpression {
            action: Box::new(Expression::Symbol("DEF".to_string())),
            args: vec![Expression::Symbol(word_name), Expression::Vector(lines)],
        })
    }
    
    fn parse_del(&mut self) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_del ==="));
        
        // [ WORD_NAME ]
        let word_name = self.parse_word_name()?;
        
        // ']'をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        Ok(Expression::SExpression {
            action: Box::new(Expression::Symbol("DEL".to_string())),
            args: vec![Expression::Symbol(word_name)],
        })
    }
    
    fn parse_line(&mut self) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_line ==="));
        
        // [ 反復制御 ]
        let repeat = self.parse_repeat_spec()?;
        
        // [ 時間制御 ]
        let timing = self.parse_time_spec()?;
        
        // [ 条件 ] （空の場合もある）
        let condition = if !self.is_at_end() && 
                           matches!(self.current_token(), Some(Token::VectorStart)) {
            let cond_expr = self.parse_expression()?;
            match cond_expr {
                Expression::Vector(ref v) if v.is_empty() => None,
                _ => Some(Box::new(cond_expr)),
            }
        } else {
            None
        };
        
        // アクション
        let action = self.parse_expression()?;
        
        // ']'をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        Ok(Expression::Line {
            repeat,
            timing,
            condition,
            action: Box::new(action),
        })
    }
    
    fn parse_word_name(&mut self) -> Result<String, String> {
        // [ WORD_NAME ] を解析
        if !matches!(self.current_token(), Some(Token::VectorStart)) {
            return Err("Expected [ for word name".to_string());
        }
        self.position += 1;
        
        let name = match self.current_token() {
            Some(Token::Symbol(s)) => s.clone(),
            _ => return Err("Expected word name symbol".to_string()),
        };
        self.position += 1;
        
        if !matches!(self.current_token(), Some(Token::VectorEnd)) {
            return Err("Expected ] after word name".to_string());
        }
        self.position += 1;
        
        Ok(name)
    }
    
    fn parse_data_vector(&mut self, first: Expression) -> Result<Expression, String> {
        console::log_1(&JsValue::from_str("=== parse_data_vector ==="));
        
        let mut elements = vec![first];
        
        // 残りの要素を収集
        while !self.is_at_end() && !matches!(self.current_token(), Some(Token::VectorEnd)) {
            elements.push(self.parse_expression()?);
        }
        
        // ']'をスキップ
        if !self.is_at_end() && matches!(self.current_token(), Some(Token::VectorEnd)) {
            self.position += 1;
        }
        
        Ok(Expression::Vector(elements))
    }
}
