// rust/src/markdown/converter.rs
//
// MVL AST → Ajisai内部表現への変換
// 辞書システムと連携し、見出しを定義として登録

use super::ast::*;
use crate::error::AjisaiError;

/// MVLドキュメントをAjisaiコードに変換した結果
#[derive(Debug)]
pub struct ConversionResult {
    /// 定義リスト（名前, コード, 説明）
    pub definitions: Vec<WordDefinition>,
    /// 実行コード（mainまたは無名ブロック）
    pub main_code: Option<String>,
    /// 変換警告
    pub warnings: Vec<String>,
}

/// ワード定義
#[derive(Debug, Clone)]
pub struct WordDefinition {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
}

/// MVLドキュメントをAjisaiコードに変換
pub fn convert_to_ajisai(doc: &MvlDocument) -> Result<ConversionResult, AjisaiError> {
    let mut converter = Converter::new();
    converter.convert(doc)
}

struct Converter {
    definitions: Vec<WordDefinition>,
    warnings: Vec<String>,
}

impl Converter {
    fn new() -> Self {
        Converter {
            definitions: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn convert(&mut self, doc: &MvlDocument) -> Result<ConversionResult, AjisaiError> {
        // セクションを定義順に処理
        for name in &doc.section_order {
            if let Some(section) = doc.sections.get(name) {
                self.convert_section(section)?;
            }
        }

        // 無名ブロック（mainコード）を処理
        let main_code = if !doc.anonymous_blocks.is_empty() {
            Some(self.convert_blocks(&doc.anonymous_blocks)?)
        } else {
            // "main" セクションを探す
            doc.sections.get("main")
                .or_else(|| doc.sections.get("Main"))
                .or_else(|| doc.sections.get("MAIN"))
                .map(|s| self.convert_blocks(&s.blocks))
                .transpose()?
        };

        Ok(ConversionResult {
            definitions: std::mem::take(&mut self.definitions),
            main_code,
            warnings: std::mem::take(&mut self.warnings),
        })
    }

    fn convert_section(&mut self, section: &MvlSection) -> Result<(), AjisaiError> {
        // "main" セクションは定義ではなく実行コードとして扱う
        let lower_name = section.name.to_lowercase();
        if lower_name == "main" {
            return Ok(());
        }

        let code = self.convert_blocks(&section.blocks)?;

        if !code.is_empty() {
            self.definitions.push(WordDefinition {
                name: section.name.clone(),
                code,
                description: section.description.clone(),
            });
        }

        Ok(())
    }

    fn convert_blocks(&mut self, blocks: &[MvlBlock]) -> Result<String, AjisaiError> {
        #[allow(unused_variables)]
        let result: Vec<String> = Vec::new();
        let mut pipeline_stack: Vec<String> = Vec::new();

        for block in blocks {
            match block {
                MvlBlock::Vector(v) => {
                    let vec_code = self.convert_vector(v)?;
                    pipeline_stack.push(vec_code);
                }

                MvlBlock::Table(t) => {
                    let table_code = self.convert_table(t)?;
                    pipeline_stack.push(table_code);
                }

                MvlBlock::Code(c) => {
                    let code = self.convert_code(c)?;
                    if !pipeline_stack.is_empty() {
                        // パイプラインの続き：前のデータに操作を適用
                        let prev = pipeline_stack.pop().unwrap();
                        pipeline_stack.push(format!("{} {}", prev, code));
                    } else {
                        pipeline_stack.push(code);
                    }
                }

                MvlBlock::Pipeline => {
                    // パイプライン区切り：何もしない（次のブロックが操作として適用される）
                    // すでにスタックに入っているデータはそのまま
                }

                MvlBlock::Conditional(c) => {
                    let cond_code = self.convert_conditional(c)?;
                    if !pipeline_stack.is_empty() {
                        let prev = pipeline_stack.pop().unwrap();
                        pipeline_stack.push(format!("{} {}", prev, cond_code));
                    } else {
                        pipeline_stack.push(cond_code);
                    }
                }

                MvlBlock::Loop(l) => {
                    let loop_code = self.convert_loop(l)?;
                    if !pipeline_stack.is_empty() {
                        let prev = pipeline_stack.pop().unwrap();
                        pipeline_stack.push(format!("{} {}", prev, loop_code));
                    } else {
                        pipeline_stack.push(loop_code);
                    }
                }

                MvlBlock::InlineExpr(expr) => {
                    let ajisai_code = self.convert_inline_expr(expr)?;
                    if !pipeline_stack.is_empty() {
                        let prev = pipeline_stack.pop().unwrap();
                        pipeline_stack.push(format!("{} {}", prev, ajisai_code));
                    } else {
                        pipeline_stack.push(ajisai_code);
                    }
                }

                MvlBlock::Reference(name) => {
                    // 他のセクションへの参照 → ワード呼び出し
                    if !pipeline_stack.is_empty() {
                        let prev = pipeline_stack.pop().unwrap();
                        pipeline_stack.push(format!("{} {}", prev, name));
                    } else {
                        pipeline_stack.push(name.clone());
                    }
                }

                MvlBlock::Comment(_) => {
                    // コメントは無視
                }
            }
        }

        // パイプラインスタックを結合
        Ok(pipeline_stack.join(" "))
    }

    fn convert_vector(&mut self, v: &MvlVector) -> Result<String, AjisaiError> {
        let elements: Result<Vec<String>, _> = v.elements.iter()
            .map(|e| self.convert_element(e))
            .collect();

        Ok(format!("[ {} ]", elements?.join(" ")))
    }

    fn convert_element(&mut self, e: &MvlElement) -> Result<String, AjisaiError> {
        match e {
            MvlElement::Number(n) => Ok(n.clone()),
            MvlElement::String(s) => Ok(format!("'{}'", s)),
            MvlElement::Vector(v) => self.convert_vector(v),
            MvlElement::Expr(expr) => self.convert_inline_expr(expr),
            MvlElement::Boolean(b) => Ok(if *b { "TRUE".to_string() } else { "FALSE".to_string() }),
            MvlElement::Nil => Ok("NIL".to_string()),
        }
    }

    fn convert_table(&mut self, t: &MvlTable) -> Result<String, AjisaiError> {
        // テーブルを2D Vectorに変換
        let rows: Result<Vec<String>, _> = t.rows.iter()
            .map(|row| {
                let cells: Result<Vec<String>, _> = row.iter()
                    .map(|e| self.convert_element(e))
                    .collect();
                cells.map(|c| format!("[ {} ]", c.join(" ")))
            })
            .collect();

        Ok(format!("[ {} ]", rows?.join(" ")))
    }

    fn convert_code(&mut self, c: &MvlCode) -> Result<String, AjisaiError> {
        // コードブロックの言語指定を確認
        let lang = c.lang.as_deref().unwrap_or("ajisai");

        if lang.starts_with("ajisai") {
            // そのままAjisaiコードとして使用
            Ok(c.code.clone())
        } else {
            // 他の言語は警告を出してスキップ
            self.warnings.push(format!(
                "Line {}: Unknown language '{}', treating as Ajisai code",
                c.line, lang
            ));
            Ok(c.code.clone())
        }
    }

    fn convert_conditional(&mut self, c: &MvlConditional) -> Result<String, AjisaiError> {
        // 条件分岐をAjisaiのGuard構文に変換
        let mut parts = Vec::new();

        for branch in &c.branches {
            // 条件
            let cond = self.convert_inline_expr(&branch.condition)?;
            parts.push(format!(": {}", cond));

            // アクション
            let action = self.convert_block_single(&branch.action)?;
            parts.push(format!(": {}", action));
        }

        // デフォルト
        if let Some(ref default) = c.default {
            let default_action = self.convert_block_single(default)?;
            parts.push(format!(": {}", default_action));
        }

        Ok(parts.join("\n"))
    }

    fn convert_block_single(&mut self, block: &MvlBlock) -> Result<String, AjisaiError> {
        match block {
            MvlBlock::InlineExpr(expr) => self.convert_inline_expr(expr),
            MvlBlock::Code(c) => self.convert_code(c),
            MvlBlock::Vector(v) => self.convert_vector(v),
            MvlBlock::Reference(name) => Ok(name.clone()),
            _ => Ok(String::new()),
        }
    }

    fn convert_loop(&mut self, l: &MvlLoop) -> Result<String, AjisaiError> {
        // ループをAjisaiのTIMES構文に変換
        let count = match &l.count {
            LoopCount::Fixed(n) => format!("[ {} ]", n),
            LoopCount::Expr(expr) => self.convert_inline_expr(expr)?,
        };

        let body_parts: Result<Vec<String>, _> = l.body.iter()
            .map(|b| self.convert_block_single(b))
            .collect();

        let body = body_parts?.join(" ");

        Ok(format!("{} '{}' TIMES", count, body))
    }

    fn convert_inline_expr(&mut self, expr: &str) -> Result<String, AjisaiError> {
        // インライン式をAjisaiコードに変換
        // 中置記法をRPNに変換する（シンプルなケースのみ）

        let trimmed = expr.trim();

        // すでにRPNの場合はそのまま
        if self.looks_like_rpn(trimmed) {
            return Ok(trimmed.to_string());
        }

        // シンプルな中置記法の変換
        self.convert_infix_to_rpn(trimmed)
    }

    fn looks_like_rpn(&self, expr: &str) -> bool {
        // RPNっぽいかどうかの簡易判定
        // - 大文字のワードが含まれる
        // - [ ] で始まる
        // - 演算子が末尾にある

        let words: Vec<&str> = expr.split_whitespace().collect();
        if words.is_empty() {
            return true;
        }

        // 最後のトークンが演算子またはワードならRPNっぽい
        let last = words.last().unwrap();
        if matches!(*last, "+" | "-" | "*" | "/" | "=" | "<" | ">" | "<=" | ">=" |
                   "AND" | "OR" | "NOT" | "MAP" | "FILTER" | "FOLD" | "GET" | "LENGTH" |
                   "CONCAT" | "REVERSE" | "TAKE" | "DUP" | "SWAP" | "DROP") {
            return true;
        }

        // [ で始まるならRPNっぽい
        if expr.trim_start().starts_with('[') {
            return true;
        }

        // 大文字のみのワードが含まれるならRPNっぽい
        for word in &words {
            if word.chars().all(|c| c.is_ascii_uppercase() || c == '_') && word.len() > 1 {
                return true;
            }
        }

        false
    }

    fn convert_infix_to_rpn(&mut self, expr: &str) -> Result<String, AjisaiError> {
        // シンプルな中置記法をRPNに変換
        // 例: "a + b" → "a b +"
        // 例: "x * 2" → "x [ 2 ] *"
        // 例: "list.map(f)" → "list 'f' MAP"

        let trimmed = expr.trim();

        // メソッド呼び出し形式: data.method(arg)
        if let Some(dot_pos) = trimmed.find('.') {
            let data = &trimmed[..dot_pos];
            let rest = &trimmed[dot_pos + 1..];

            if let Some(paren_pos) = rest.find('(') {
                let method = &rest[..paren_pos];
                let args = rest[paren_pos + 1..].trim_end_matches(')');

                let method_upper = method.to_uppercase();
                match method_upper.as_str() {
                    "MAP" | "FILTER" | "FOLD" => {
                        let data_rpn = self.convert_infix_to_rpn(data)?;
                        return Ok(format!("{} '{}' {}", data_rpn, args.trim(), method_upper));
                    }
                    "GET" => {
                        let data_rpn = self.convert_infix_to_rpn(data)?;
                        return Ok(format!("{} [ {} ] GET", data_rpn, args.trim()));
                    }
                    "LENGTH" | "REVERSE" | "SUM" => {
                        let data_rpn = self.convert_infix_to_rpn(data)?;
                        return Ok(format!("{} {}", data_rpn, method_upper));
                    }
                    _ => {}
                }
            }
        }

        // 二項演算: a op b
        for op in &[" + ", " - ", " * ", " / ", " = ", " < ", " > ", " <= ", " >= "] {
            if let Some(pos) = trimmed.find(op) {
                let left = &trimmed[..pos];
                let right = &trimmed[pos + op.len()..];

                let left_rpn = self.convert_value_to_rpn(left.trim())?;
                let right_rpn = self.convert_value_to_rpn(right.trim())?;

                return Ok(format!("{} {} {}", left_rpn, right_rpn, op.trim()));
            }
        }

        // 単一の値
        self.convert_value_to_rpn(trimmed)
    }

    fn convert_value_to_rpn(&mut self, value: &str) -> Result<String, AjisaiError> {
        let trimmed = value.trim();

        // 数値
        if trimmed.parse::<i64>().is_ok() || trimmed.parse::<f64>().is_ok() {
            return Ok(format!("[ {} ]", trimmed));
        }

        // 分数
        if trimmed.contains('/') {
            let parts: Vec<&str> = trimmed.split('/').collect();
            if parts.len() == 2
               && parts[0].trim().parse::<i64>().is_ok()
               && parts[1].trim().parse::<i64>().is_ok() {
                return Ok(format!("[ {} ]", trimmed));
            }
        }

        // 文字列（引用符付き）
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
           || (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
            let inner = &trimmed[1..trimmed.len()-1];
            return Ok(format!("[ '{}' ]", inner));
        }

        // ブール値
        if trimmed.eq_ignore_ascii_case("true") {
            return Ok("[ TRUE ]".to_string());
        }
        if trimmed.eq_ignore_ascii_case("false") {
            return Ok("[ FALSE ]".to_string());
        }

        // NIL
        if trimmed.eq_ignore_ascii_case("nil") {
            return Ok("NIL".to_string());
        }

        // 配列リテラル [a, b, c]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len()-1];
            let elements: Result<Vec<String>, _> = inner.split(',')
                .map(|e| self.convert_value_to_rpn(e.trim()))
                .collect();

            let elem_strs: Vec<String> = elements?.iter()
                .map(|e| {
                    // [ x ] の形式を x に戻す
                    if e.starts_with("[ ") && e.ends_with(" ]") {
                        e[2..e.len()-2].to_string()
                    } else {
                        e.clone()
                    }
                })
                .collect();

            return Ok(format!("[ {} ]", elem_strs.join(" ")));
        }

        // 識別子（ワード参照）
        Ok(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parser::parse_markdown;

    #[test]
    fn test_convert_simple_vector() {
        let input = r#"
- 1
- 2
- 3
"#;
        let doc = parse_markdown(input).unwrap().document;
        let result = convert_to_ajisai(&doc).unwrap();

        assert!(result.main_code.is_some());
        let code = result.main_code.unwrap();
        assert!(code.contains("[ 1 2 3 ]"));
    }

    #[test]
    fn test_convert_section_to_definition() {
        let input = r#"
# double

2倍にする

```ajisai
[ 2 ] *
```
"#;
        let doc = parse_markdown(input).unwrap().document;
        let result = convert_to_ajisai(&doc).unwrap();

        assert_eq!(result.definitions.len(), 1);
        assert_eq!(result.definitions[0].name, "double");
        assert_eq!(result.definitions[0].code, "[ 2 ] *");
        assert_eq!(result.definitions[0].description, Some("2倍にする".to_string()));
    }

    #[test]
    fn test_convert_pipeline() {
        let input = r#"
- 1
- 2
- 3

---

```ajisai
[ 2 ] *
```
"#;
        let doc = parse_markdown(input).unwrap().document;
        let result = convert_to_ajisai(&doc).unwrap();

        let code = result.main_code.unwrap();
        assert!(code.contains("[ 1 2 3 ]"));
        assert!(code.contains("[ 2 ] *"));
    }

    #[test]
    fn test_convert_infix_to_rpn() {
        let mut converter = Converter::new();

        // シンプルな演算
        let result = converter.convert_infix_to_rpn("5 + 3").unwrap();
        assert_eq!(result, "[ 5 ] [ 3 ] +");
    }

    #[test]
    fn test_convert_table() {
        let input = r#"
| 1 | 2 |
|---|---|
| 3 | 4 |
"#;
        let doc = parse_markdown(input).unwrap().document;
        let result = convert_to_ajisai(&doc).unwrap();

        let code = result.main_code.unwrap();
        assert!(code.contains("[ [ 3 4 ] ]") || code.contains("[ 3 4 ]"));
    }
}
