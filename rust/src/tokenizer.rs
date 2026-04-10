use crate::types::Token;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {

        if chars[i].is_whitespace() {
            if chars[i] == '\n' {

                if tokens.last() != Some(&Token::LineBreak) {
                    tokens.push(Token::LineBreak);
                }
            }
            i += 1;
            continue;
        }


        if chars[i] == '#' {
            let had_token_before = !tokens.is_empty() && tokens.last() != Some(&Token::LineBreak);

            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }

            if !had_token_before && i < chars.len() && chars[i] == '\n' {
                i += 1;
            }
            continue;
        }

        if chars[i] == ':' {
            return Err("':' (code block start) has been removed. Use '{' and '}' or '(' and ')' for code blocks.".to_string());
        }
        if chars[i] == ';' {
            return Err("';' (code block end) has been removed. Use '{' and '}' or '(' and ')' for code blocks.".to_string());
        }
        if let Some((token, consumed)) = parse_token_from_single_char(chars[i]) {
            tokens.push(token);
            i += consumed;
            continue;
        }


        if chars[i] == '=' {

            if i + 1 < chars.len() && chars[i + 1] == '=' {
                tokens.push(Token::Pipeline);
                i += 2;
                continue;
            }

            if i + 1 < chars.len() && chars[i + 1] == '>' {
                tokens.push(Token::NilCoalesce);
                i += 2;
                continue;
            }

            tokens.push(Token::Symbol("=".into()));
            i += 1;
            continue;
        }


        if chars[i] == '<' {

            if i + 1 < chars.len() && chars[i + 1] == '=' {
                tokens.push(Token::Symbol("<=".into()));
                i += 2;
                continue;
            }

            tokens.push(Token::Symbol("<".into()));
            i += 1;
            continue;
        }


        if chars[i] == '>' {
            if i + 2 < chars.len() && chars[i + 1] == '>' && chars[i + 2] == '>' {
                return Err("'>>>' (chevron default) has been removed.".to_string());
            }
            if i + 1 < chars.len() && chars[i + 1] == '>' {
                return Err("'>>' (chevron branch) has been removed.".to_string());
            }
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                return Err("The '>=' operator has been removed. Use '<= NOT' or reverse operands with '<=' instead.".to_string());
            }
            return Err("The '>' operator has been removed. Use '< NOT' or reverse operands with '<' instead.".to_string());
        }


        match parse_string_from_quote(&chars[i..]) {
            QuoteParseResult::StringSuccess(token, consumed) => {
                tokens.push(token);
                i += consumed;
                continue;
            }
            QuoteParseResult::Unclosed => {
                let quote_char = chars[i];
                return Err(format!("Unclosed literal starting with {}", quote_char));
            }
            QuoteParseResult::NotQuote => {

            }
        }


        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && !is_special_char(chars[i]) {
            i += 1;
        }

        if i == start {

            return Err(format!("Unexpected character: {}", chars[i]));
        }

        let token_str: String = chars[start..i].iter().collect();


        if let Some(token) = parse_keyword_from_string(&token_str) {
            tokens.push(token);
            continue;
        }


        if let Some(token) = parse_number_from_string(&token_str) {
            tokens.push(token);
            continue;
        }


        tokens.push(Token::Symbol(token_str.into()));
    }


    if tokens.last() == Some(&Token::LineBreak) {
        tokens.pop();
    }

    check_bracket_matching(input)?;
    check_single_line_block_constraint(&tokens)?;
    check_cond_clause_per_line_constraint(&tokens)?;
    Ok(tokens)
}



fn is_special_char(c: char) -> bool {
    matches!(
        c,
        '[' | ']' | '{' | '}' | '(' | ')' | '#' | '\'' | '>' | '=' | '~' | '$'
    )
}

fn parse_token_from_single_char(c: char) -> Option<(Token, usize)> {
    match c {
        '[' => Some((Token::VectorStart, 1)),
        ']' => Some((Token::VectorEnd, 1)),
        '{' | '(' => Some((Token::BlockStart, 1)),
        '}' | ')' => Some((Token::BlockEnd, 1)),

        '$' => Some((Token::CondClauseSep, 1)),
        '~' => Some((Token::SafeMode, 1)),

        _ => None,
    }
}

fn check_bracket_matching(input: &str) -> Result<(), String> {
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut in_comment = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i: usize = 0;

    while i < chars.len() {
        let c: char = chars[i];

        if c == '\n' {
            in_comment = false;
            i += 1;
            continue;
        }

        if in_comment {
            i += 1;
            continue;
        }

        if c == '#' {
            in_comment = true;
            i += 1;
            continue;
        }

        if c == '\'' {
            if in_string {

                if i + 1 >= chars.len() || chars[i + 1].is_whitespace() || is_special_char(chars[i + 1]) {
                    in_string = false;
                }
            } else {
                in_string = true;
            }
            i += 1;
            continue;
        }

        if in_string {
            i += 1;
            continue;
        }

        match c {
            '[' | '{' | '(' => stack.push(c),
            ']' => match stack.pop() {
                Some('[') => {}
                Some(open) => {
                    return Err(format!(
                        "Mismatched brackets: '{}' is closed by ']', expected '{}'",
                        open,
                        closing_bracket(open)
                    ));
                }
                None => {
                    return Err("Unexpected ']' without matching '['".to_string());
                }
            },
            '}' => match stack.pop() {
                Some('{') => {}
                Some(open) => {
                    return Err(format!(
                        "Mismatched brackets: '{}' is closed by '}}', expected '{}'",
                        open,
                        closing_bracket(open)
                    ));
                }
                None => {
                    return Err("Unexpected '}' without matching '{'".to_string());
                }
            },
            ')' => match stack.pop() {
                Some('(') => {}
                Some(open) => {
                    return Err(format!(
                        "Mismatched brackets: '{}' is closed by ')', expected '{}'",
                        open,
                        closing_bracket(open)
                    ));
                }
                None => {
                    return Err("Unexpected ')' without matching '('".to_string());
                }
            },
            _ => {}
        }
        i += 1;
    }

    if let Some(open) = stack.last() {
        return Err(format!(
            "Unclosed '{}': expected '{}'",
            open,
            closing_bracket(*open)
        ));
    }

    Ok(())
}

fn closing_bracket(open: char) -> char {
    match open {
        '[' => ']',
        '{' => '}',
        '(' => ')',
        _ => '?',
    }
}

fn check_single_line_block_constraint(tokens: &[Token]) -> Result<(), String> {
    let mut depth: i32 = 0;

    for token in tokens {
        match token {
            Token::BlockStart => depth += 1,
            Token::BlockEnd => depth -= 1,
            Token::LineBreak if depth > 0 => {
                return Err(
                    "ParseError: Code block must be on a single line. Use named words to break up long definitions.".to_string()
                );
            }
            _ => {}
        }
    }

    Ok(())
}

fn check_cond_clause_per_line_constraint(tokens: &[Token]) -> Result<(), String> {
    let mut i: usize = 0;
    let mut cond_clause_blocks_in_line: usize = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::LineBreak => {
                cond_clause_blocks_in_line = 0;
                i += 1;
            }
            Token::BlockStart => {
                let mut depth: i32 = 1;
                let mut j: usize = i + 1;
                let mut has_clause_sep: bool = false;
                while j < tokens.len() && depth > 0 {
                    match &tokens[j] {
                        Token::BlockStart => depth += 1,
                        Token::BlockEnd => depth -= 1,
                        Token::CondClauseSep if depth == 1 => has_clause_sep = true,
                        _ => {}
                    }
                    j += 1;
                }

                if has_clause_sep {
                    cond_clause_blocks_in_line += 1;
                    if cond_clause_blocks_in_line > 1 {
                        return Err("COND: $ clauses must be written one clause per line".to_string());
                    }
                }

                i = j;
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(())
}


enum QuoteParseResult {

    StringSuccess(Token, usize),

    Unclosed,

    NotQuote,
}

fn parse_string_from_quote(chars: &[char]) -> QuoteParseResult {
    if chars.is_empty() {
        return QuoteParseResult::NotQuote;
    }

    let quote_char = chars[0];

    match quote_char {
        '\'' => parse_token_from_string_literal(chars),
        _ => QuoteParseResult::NotQuote,
    }
}


fn parse_token_from_string_literal(chars: &[char]) -> QuoteParseResult {
    if chars.is_empty() || chars[0] != '\'' {
        return QuoteParseResult::NotQuote;
    }

    let mut string = String::new();
    let mut i = 1;


    while i < chars.len() {
        if chars[i] == '\'' {

            if i + 1 >= chars.len() || is_delimiter(chars[i + 1]) {
                return QuoteParseResult::StringSuccess(Token::String(string.into()), i + 1);
            } else {

                string.push(chars[i]);
                i += 1;
            }
        } else {
            string.push(chars[i]);
            i += 1;
        }
    }


    QuoteParseResult::Unclosed
}


fn is_delimiter(c: char) -> bool {
    c.is_whitespace() || is_special_char(c)
}




fn parse_keyword_from_string(s: &str) -> Option<Token> {
    match s {
        "." => Some(Token::Symbol(".".into())),
        ".." => Some(Token::Symbol("..".into())),
        _ => None,
    }
}


fn parse_number_from_string(s: &str) -> Option<Token> {

    if s.is_empty() {
        return None;
    }

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;


    if chars[i] == '-' || chars[i] == '+' {
        if chars.len() == 1 {

            return None;
        }
        if !chars[i + 1].is_ascii_digit() {

            return None;
        }
        i += 1;
    }


    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return None;
    }

    let start = i;


    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }


    if i < chars.len() && chars[i] == '/' {
        let _slash_pos = i;
        i += 1;



        if i >= chars.len() || !chars[i].is_ascii_digit() {


            return None;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }


        if i == chars.len() {
            return Some(Token::Number(s.into()));
        } else {

            return None;
        }
    }


    let mut has_dot = false;
    if i < chars.len() && chars[i] == '.' {
        has_dot = true;
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }


    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        i += 1;
        if i < chars.len() && (chars[i] == '-' || chars[i] == '+') {
            i += 1;
        }
        if i >= chars.len() || !chars[i].is_ascii_digit() {

            return None;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }


    if i == start && !has_dot {

        return None;
    }


    if i == chars.len() {
        Some(Token::Number(s.into()))
    } else {

        None
    }
}
