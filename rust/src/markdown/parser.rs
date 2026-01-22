// rust/src/markdown/parser.rs
//
// Markdown Vector Language Parser
// pulldown-cmarkを使用してMarkdownを解析し、MVL ASTに変換する

use pulldown_cmark::{Event, Parser, Tag, CodeBlockKind, HeadingLevel, Options};
use super::ast::*;

/// MarkdownテキストをMVL ASTに変換
pub fn parse_markdown(input: &str) -> Result<ParseResult, ParseError> {
    let mut parser_state = ParserState::new();

    // GFM拡張を有効にする（テーブル等）
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(input, options);

    // 行番号追跡用
    let mut current_line = 1;

    for event in parser {
        match event {
            Event::Start(tag) => {
                parser_state.handle_start(tag, current_line)?;
            }
            Event::End(tag) => {
                parser_state.handle_end(tag)?;
            }
            Event::Text(text) => {
                parser_state.handle_text(&text, current_line)?;
                // 改行をカウント
                current_line += text.matches('\n').count();
            }
            Event::Code(code) => {
                parser_state.handle_inline_code(&code, current_line)?;
            }
            Event::SoftBreak | Event::HardBreak => {
                current_line += 1;
            }
            Event::Rule => {
                parser_state.handle_rule(current_line)?;
            }
            _ => {}
        }
    }

    // 最後のセクションを確定
    parser_state.finalize_current_section()?;

    Ok(ParseResult {
        document: parser_state.document,
        warnings: parser_state.warnings,
    })
}

/// パーサーの内部状態
struct ParserState {
    document: MvlDocument,
    warnings: Vec<ParseWarning>,

    // 現在のコンテキスト
    current_section: Option<MvlSection>,
    current_blocks: Vec<MvlBlock>,

    // リスト処理用
    list_stack: Vec<ListContext>,
    in_list_item: bool,
    list_item_text: String,

    // コードブロック処理用
    in_code_block: bool,
    code_block_lang: Option<String>,
    code_block_content: String,
    code_block_line: usize,

    // テーブル処理用
    in_table: bool,
    in_table_head: bool,
    table_rows: Vec<Vec<MvlElement>>,
    current_row: Vec<MvlElement>,
    current_cell: String,
    table_line: usize,

    // 引用ブロック処理用
    in_blockquote: bool,
    blockquote_content: String,
    blockquote_line: usize,

    // パラグラフ処理用
    in_paragraph: bool,
    paragraph_content: String,
    paragraph_line: usize,

    // 見出し処理用
    in_heading: bool,
}

/// リストのコンテキスト
struct ListContext {
    /// 番号付きリストかどうか
    ordered: bool,
    /// 現在のアイテム番号
    #[allow(dead_code)]
    item_number: usize,
    /// ネストレベル
    #[allow(dead_code)]
    depth: usize,
    /// このレベルの要素
    elements: Vec<MvlElement>,
}

impl ParserState {
    fn new() -> Self {
        ParserState {
            document: MvlDocument::new(),
            warnings: Vec::new(),
            current_section: None,
            current_blocks: Vec::new(),
            list_stack: Vec::new(),
            in_list_item: false,
            list_item_text: String::new(),
            in_code_block: false,
            code_block_lang: None,
            code_block_content: String::new(),
            code_block_line: 0,
            in_table: false,
            in_table_head: false,
            table_rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
            table_line: 0,
            in_blockquote: false,
            blockquote_content: String::new(),
            blockquote_line: 0,
            in_paragraph: false,
            paragraph_content: String::new(),
            paragraph_line: 0,
            in_heading: false,
        }
    }

    fn handle_start(&mut self, tag: Tag, line: usize) -> Result<(), ParseError> {
        match tag {
            Tag::Heading(level, _, _) => {
                // 前のセクションを確定
                self.finalize_current_section()?;

                let level_num = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };

                self.current_section = Some(MvlSection {
                    name: String::new(),
                    level: level_num,
                    description: None,
                    blocks: Vec::new(),
                    line,
                });
                self.in_heading = true;
            }

            Tag::List(first_item) => {
                let ordered = first_item.is_some();
                self.list_stack.push(ListContext {
                    ordered,
                    item_number: first_item.unwrap_or(1) as usize,
                    depth: self.list_stack.len(),
                    elements: Vec::new(),
                });
            }

            Tag::Item => {
                self.in_list_item = true;
                self.list_item_text.clear();
            }

            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                self.code_block_line = line;
                self.code_block_content.clear();

                self.code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let lang_str = lang.to_string();
                        if lang_str.is_empty() {
                            None
                        } else {
                            Some(lang_str)
                        }
                    }
                    CodeBlockKind::Indented => None,
                };
            }

            Tag::Table(_alignments) => {
                self.in_table = true;
                self.table_line = line;
                self.table_rows.clear();
            }

            Tag::TableHead => {
                self.in_table_head = true;
                self.current_row.clear();
            }

            Tag::TableRow => {
                self.current_row.clear();
            }

            Tag::TableCell => {
                self.current_cell.clear();
            }

            Tag::BlockQuote => {
                self.in_blockquote = true;
                self.blockquote_line = line;
                self.blockquote_content.clear();
            }

            Tag::Paragraph => {
                if !self.in_blockquote && !self.in_list_item {
                    self.in_paragraph = true;
                    self.paragraph_line = line;
                    self.paragraph_content.clear();
                }
            }

            _ => {}
        }
        Ok(())
    }

    fn handle_end(&mut self, tag: Tag) -> Result<(), ParseError> {
        match tag {
            Tag::Heading(_, _, _) => {
                self.in_heading = false;
            }

            Tag::List(_) => {
                if let Some(ctx) = self.list_stack.pop() {
                    let vector = MvlVector {
                        elements: ctx.elements,
                        line: 0,
                    };

                    if self.list_stack.is_empty() {
                        // トップレベルリスト → ブロックとして追加
                        if ctx.ordered {
                            // 番号付きリスト → ループとして解釈
                            let loop_block = MvlLoop {
                                count: LoopCount::Fixed(vector.elements.len()),
                                body: vector.elements.iter().map(|e| {
                                    match e {
                                        MvlElement::Expr(expr) => MvlBlock::InlineExpr(expr.clone()),
                                        MvlElement::String(s) => MvlBlock::Code(MvlCode {
                                            code: s.clone(),
                                            lang: Some("ajisai".to_string()),
                                            line: 0,
                                        }),
                                        _ => MvlBlock::Comment(format!("{:?}", e)),
                                    }
                                }).collect(),
                                line: vector.line,
                            };
                            self.current_blocks.push(MvlBlock::Loop(loop_block));
                        } else {
                            // 箇条書きリスト → Vector
                            self.current_blocks.push(MvlBlock::Vector(vector));
                        }
                    } else {
                        // ネストしたリスト → 親リストに要素として追加
                        if let Some(parent) = self.list_stack.last_mut() {
                            parent.elements.push(MvlElement::Vector(vector));
                        }
                    }
                }
            }

            Tag::Item => {
                self.in_list_item = false;
                // リストアイテムの内容を処理
                if !self.list_item_text.is_empty() {
                    let text = self.list_item_text.clone();
                    let element = self.parse_cell_content(&text);
                    if let Some(ctx) = self.list_stack.last_mut() {
                        ctx.elements.push(element);
                    }
                }
                self.list_item_text.clear();
            }

            Tag::CodeBlock(_) => {
                self.in_code_block = false;
                let code_block = MvlCode {
                    code: self.code_block_content.trim().to_string(),
                    lang: self.code_block_lang.take(),
                    line: self.code_block_line,
                };
                self.current_blocks.push(MvlBlock::Code(code_block));
            }

            Tag::Table(_) => {
                self.in_table = false;
                let table = MvlTable {
                    rows: std::mem::take(&mut self.table_rows),
                    line: self.table_line,
                };
                self.current_blocks.push(MvlBlock::Table(table));
            }

            Tag::TableHead => {
                self.in_table_head = false;
                // ヘッダー行は無視（MVLではデータのみ使用）
            }

            Tag::TableRow => {
                if !self.in_table_head && !self.current_row.is_empty() {
                    self.table_rows.push(std::mem::take(&mut self.current_row));
                }
            }

            Tag::TableCell => {
                let element = self.parse_cell_content(&self.current_cell.clone());
                self.current_row.push(element);
            }

            Tag::BlockQuote => {
                self.in_blockquote = false;
                let conditional = self.parse_blockquote_as_conditional()?;
                self.current_blocks.push(MvlBlock::Conditional(conditional));
            }

            Tag::Paragraph => {
                if self.in_paragraph {
                    self.in_paragraph = false;
                    let content = self.paragraph_content.trim().to_string();
                    if !content.is_empty() {
                        // セクションの説明として追加するか、コメントブロックとして追加
                        if let Some(ref mut section) = self.current_section {
                            if section.description.is_none() && self.current_blocks.is_empty() {
                                section.description = Some(content);
                            } else {
                                self.current_blocks.push(MvlBlock::Comment(content));
                            }
                        } else {
                            self.current_blocks.push(MvlBlock::Comment(content));
                        }
                    }
                }
            }

            _ => {}
        }
        Ok(())
    }

    fn handle_text(&mut self, text: &str, _line: usize) -> Result<(), ParseError> {
        if self.in_code_block {
            self.code_block_content.push_str(text);
        } else if self.in_table {
            self.current_cell.push_str(text);
        } else if self.in_blockquote {
            self.blockquote_content.push_str(text);
            self.blockquote_content.push('\n');
        } else if self.in_list_item {
            self.list_item_text.push_str(text);
        } else if self.in_paragraph {
            self.paragraph_content.push_str(text);
        } else if self.in_heading {
            if let Some(ref mut section) = self.current_section {
                section.name.push_str(text.trim());
            }
        }
        Ok(())
    }

    fn handle_inline_code(&mut self, code: &str, _line: usize) -> Result<(), ParseError> {
        if self.in_list_item {
            // リスト内のインラインコード → 式として追加
            if let Some(ctx) = self.list_stack.last_mut() {
                ctx.elements.push(MvlElement::Expr(code.to_string()));
            }
        } else if self.in_paragraph {
            // パラグラフ内のインラインコード
            self.paragraph_content.push('`');
            self.paragraph_content.push_str(code);
            self.paragraph_content.push('`');
        } else {
            // 独立したインライン式
            self.current_blocks.push(MvlBlock::InlineExpr(code.to_string()));
        }
        Ok(())
    }

    fn handle_rule(&mut self, _line: usize) -> Result<(), ParseError> {
        // --- はパイプライン区切り
        self.current_blocks.push(MvlBlock::Pipeline);
        Ok(())
    }

    fn finalize_current_section(&mut self) -> Result<(), ParseError> {
        // 現在のブロックを確定
        let blocks = std::mem::take(&mut self.current_blocks);

        if let Some(mut section) = self.current_section.take() {
            section.blocks = blocks;
            let name = section.name.clone();

            if !name.is_empty() {
                self.document.section_order.push(name.clone());
                self.document.sections.insert(name, section);
            }
        } else if !blocks.is_empty() {
            // 無名ブロック（見出しなし）
            self.document.anonymous_blocks.extend(blocks);
        }

        Ok(())
    }

    /// セル内容をMvlElementに変換
    fn parse_cell_content(&self, content: &str) -> MvlElement {
        let trimmed = content.trim();

        // インラインコード
        if trimmed.starts_with('`') && trimmed.ends_with('`') && trimmed.len() > 2 {
            return MvlElement::Expr(trimmed[1..trimmed.len()-1].to_string());
        }

        // 真偽値
        if trimmed.eq_ignore_ascii_case("true") {
            return MvlElement::Boolean(true);
        }
        if trimmed.eq_ignore_ascii_case("false") {
            return MvlElement::Boolean(false);
        }

        // NIL
        if trimmed.eq_ignore_ascii_case("nil") {
            return MvlElement::Nil;
        }

        // 数値（整数、分数、小数）
        if let Some(num) = self.try_parse_number(trimmed) {
            return MvlElement::Number(num);
        }

        // 文字列として扱う
        MvlElement::String(trimmed.to_string())
    }

    /// 数値としてパースを試みる
    fn try_parse_number(&self, s: &str) -> Option<String> {
        // 整数
        if s.parse::<i64>().is_ok() {
            return Some(s.to_string());
        }

        // 分数 (例: 1/3)
        if s.contains('/') {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() == 2 {
                if parts[0].trim().parse::<i64>().is_ok()
                   && parts[1].trim().parse::<i64>().is_ok() {
                    return Some(s.to_string());
                }
            }
        }

        // 小数（内部的には分数に変換される）
        if s.parse::<f64>().is_ok() {
            return Some(s.to_string());
        }

        // 負数
        if s.starts_with('-') {
            return self.try_parse_number(&s[1..]).map(|n| format!("-{}", n));
        }

        None
    }

    /// 引用ブロックを条件分岐として解析
    fn parse_blockquote_as_conditional(&self) -> Result<MvlConditional, ParseError> {
        let mut branches = Vec::new();
        let mut default = None;

        let lines: Vec<&str> = self.blockquote_content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.is_empty() {
                i += 1;
                continue;
            }

            // "それ以外" または "else" または "otherwise"
            if line.eq_ignore_ascii_case("それ以外")
               || line.eq_ignore_ascii_case("else")
               || line.eq_ignore_ascii_case("otherwise") {
                i += 1;
                while i < lines.len() {
                    let action_line = lines[i].trim();
                    if action_line.starts_with('→') || action_line.starts_with("->") {
                        let action = action_line.trim_start_matches('→')
                            .trim_start_matches("->")
                            .trim();
                        default = Some(Box::new(MvlBlock::InlineExpr(action.to_string())));
                        break;
                    }
                    i += 1;
                }
            } else if line.ends_with('?') || !line.starts_with('→') && !line.starts_with("->") {
                // 条件行
                let condition = line.trim_end_matches('?').trim().to_string();

                i += 1;
                while i < lines.len() {
                    let action_line = lines[i].trim();
                    if action_line.starts_with('→') || action_line.starts_with("->") {
                        let action = action_line.trim_start_matches('→')
                            .trim_start_matches("->")
                            .trim();
                        branches.push(MvlBranch {
                            condition,
                            action: Box::new(MvlBlock::InlineExpr(action.to_string())),
                        });
                        break;
                    }
                    i += 1;
                }
            }

            i += 1;
        }

        Ok(MvlConditional {
            branches,
            default,
            line: self.blockquote_line,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_list() {
        let input = r#"
- 1
- 2
- 3
"#;
        let result = parse_markdown(input).unwrap();
        assert_eq!(result.document.anonymous_blocks.len(), 1);

        if let MvlBlock::Vector(v) = &result.document.anonymous_blocks[0] {
            assert_eq!(v.elements.len(), 3);
        } else {
            panic!("Expected Vector block");
        }
    }

    #[test]
    fn test_parse_section_with_list() {
        let input = r#"
# data

- 10
- 20
- 30
"#;
        let result = parse_markdown(input).unwrap();
        assert!(result.document.sections.contains_key("data"));

        let section = result.document.sections.get("data").unwrap();
        assert_eq!(section.blocks.len(), 1);
    }

    #[test]
    fn test_parse_pipeline() {
        let input = r#"
- 1
- 2
- 3

---

```ajisai
* 2
```
"#;
        let result = parse_markdown(input).unwrap();
        assert_eq!(result.document.anonymous_blocks.len(), 3);

        assert!(matches!(result.document.anonymous_blocks[1], MvlBlock::Pipeline));
    }

    #[test]
    fn test_parse_table() {
        let input = r#"
| 1 | 2 | 3 |
|---|---|---|
| 4 | 5 | 6 |
"#;
        let result = parse_markdown(input).unwrap();
        assert_eq!(result.document.anonymous_blocks.len(), 1);

        if let MvlBlock::Table(t) = &result.document.anonymous_blocks[0] {
            assert_eq!(t.rows.len(), 1); // ヘッダーを除く
            assert_eq!(t.rows[0].len(), 3);
        } else {
            panic!("Expected Table block");
        }
    }
}
