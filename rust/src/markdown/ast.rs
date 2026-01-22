// rust/src/markdown/ast.rs
//
// Markdown Vector Language (MVL) Abstract Syntax Tree
// Markdownの構造をプログラムとして表現するAST

use std::collections::HashMap;

/// MVLドキュメント全体
#[derive(Debug, Clone)]
pub struct MvlDocument {
    /// 名前付きセクション（見出しによる定義）
    pub sections: HashMap<String, MvlSection>,
    /// セクションの定義順序（実行順序の決定に使用）
    pub section_order: Vec<String>,
    /// トップレベルの無名ブロック（見出しなしの内容）
    pub anonymous_blocks: Vec<MvlBlock>,
}

impl MvlDocument {
    pub fn new() -> Self {
        MvlDocument {
            sections: HashMap::new(),
            section_order: Vec::new(),
            anonymous_blocks: Vec::new(),
        }
    }
}

impl Default for MvlDocument {
    fn default() -> Self {
        Self::new()
    }
}

/// 名前付きセクション（# 見出し）
#[derive(Debug, Clone)]
pub struct MvlSection {
    /// セクション名（見出しテキスト）
    pub name: String,
    /// 見出しレベル（1-6）
    pub level: u8,
    /// 説明文（パラグラフ）
    pub description: Option<String>,
    /// セクション内のブロック列
    pub blocks: Vec<MvlBlock>,
    /// ソース位置（行番号）
    pub line: usize,
}

/// MVLブロック - プログラムの基本構成要素
#[derive(Debug, Clone)]
pub enum MvlBlock {
    /// Vectorリテラル（Markdownリストから）
    Vector(MvlVector),

    /// テーブル（2D Vector）
    Table(MvlTable),

    /// コードブロック（Ajisai式）
    Code(MvlCode),

    /// パイプライン区切り（---）
    Pipeline,

    /// 条件分岐（引用ブロック）
    Conditional(MvlConditional),

    /// 繰り返し（番号付きリスト）
    Loop(MvlLoop),

    /// インライン式（`式`）
    InlineExpr(String),

    /// 参照（他のセクション名）
    Reference(String),

    /// コメント/説明（パラグラフ）
    Comment(String),
}

/// Vectorリテラル（Markdownリストから変換）
#[derive(Debug, Clone)]
pub struct MvlVector {
    /// Vector要素
    pub elements: Vec<MvlElement>,
    /// ソース位置
    pub line: usize,
}

/// Vector要素
#[derive(Debug, Clone)]
pub enum MvlElement {
    /// 数値
    Number(String),
    /// 文字列
    String(String),
    /// ネストしたVector
    Vector(MvlVector),
    /// インライン式（`式`）
    Expr(String),
    /// 真偽値
    Boolean(bool),
    /// NIL
    Nil,
}

/// テーブル（2D Vector）
#[derive(Debug, Clone)]
pub struct MvlTable {
    /// 行データ
    pub rows: Vec<Vec<MvlElement>>,
    /// ソース位置
    pub line: usize,
}

/// コードブロック
#[derive(Debug, Clone)]
pub struct MvlCode {
    /// コード内容（Ajisai RPN式）
    pub code: String,
    /// 言語指定（ajisai, ajisai:run など）
    pub lang: Option<String>,
    /// ソース位置
    pub line: usize,
}

/// 条件分岐（引用ブロックから）
#[derive(Debug, Clone)]
pub struct MvlConditional {
    /// 条件と分岐のペア
    pub branches: Vec<MvlBranch>,
    /// デフォルト分岐
    pub default: Option<Box<MvlBlock>>,
    /// ソース位置
    pub line: usize,
}

/// 条件分岐の1分岐
#[derive(Debug, Clone)]
pub struct MvlBranch {
    /// 条件式
    pub condition: String,
    /// 実行ブロック
    pub action: Box<MvlBlock>,
}

/// 繰り返し（番号付きリストから）
#[derive(Debug, Clone)]
pub struct MvlLoop {
    /// 繰り返し回数または条件
    pub count: LoopCount,
    /// 実行ブロック
    pub body: Vec<MvlBlock>,
    /// ソース位置
    pub line: usize,
}

/// ループ回数の指定方法
#[derive(Debug, Clone)]
pub enum LoopCount {
    /// 固定回数
    Fixed(usize),
    /// 式による指定
    Expr(String),
}

/// パース結果
#[derive(Debug)]
pub struct ParseResult {
    pub document: MvlDocument,
    pub warnings: Vec<ParseWarning>,
}

/// パース警告
#[derive(Debug)]
pub struct ParseWarning {
    pub message: String,
    pub line: usize,
}

/// パースエラー
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}
