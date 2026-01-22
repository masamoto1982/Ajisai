![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")

# Ajisai

**マークダウンベースのスタック指向プログラミング言語**

*A markdown-based stack-oriented programming language*

**Demo:** [https://masamoto1982.github.io/Ajisai/](https://masamoto1982.github.io/Ajisai/)

---

## AI駆動開発について / About AI-Driven Development

> **このプロジェクトの実装の大半はAI（Claude）によって行われています。**
> 設計方針の決定から、Rust/TypeScriptのコード実装、テストケースの作成、ドキュメント整備まで、
> 人間とAIの協働によって開発が進められています。
>
> *The majority of this project's implementation was done by AI (Claude).*
> *From design decisions to Rust/TypeScript code implementation, test case creation, and documentation,*
> *this project is developed through human-AI collaboration.*

---

## 概要 / Overview

Ajisaiは、**Markdown Vector Language (MVL)** を採用したマークダウンベースのプログラミング言語です。
標準的なマークダウン記法を使用して、データ構造とプログラムロジックを自然に表現できます。
WebAssembly上で動作するインタープリタと、Webベースの対話的なGUIを提供します。

*Ajisai is a markdown-based programming language that uses **Markdown Vector Language (MVL)**.*
*Using standard markdown notation, you can naturally express data structures and program logic.*
*It provides an interpreter running on WebAssembly and an interactive web-based GUI.*

「Ajisai（紫陽花）」という名前は、小さな要素が集まって構造を形成する特徴を、小さな花が集まって一つの花房を形作る紫陽花に例えています。

*The name "Ajisai" (hydrangea) metaphorically represents how small elements come together to form a structure, like how small flowers form a hydrangea cluster.*

---

## 特徴 / Features

### MVL（Markdown Vector Language）

マークダウンの記法がそのままプログラミング言語の構文になります：

| マークダウン | Ajisai構造 | 説明 |
|:---|:---|:---|
| リスト `-` | Vector | 1次元以上のベクター |
| テーブル `\|` | 2D Vector | 2次元ベクター（行列） |
| コードブロック ` ``` ` | RPN式 | 逆ポーランド記法のコード |
| 見出し `#` | ワード定義 | カスタムワードの定義 |
| 水平線 `---` | パイプライン | 処理の連結 |

### 言語設計 / Language Design

- **マークダウンネイティブ**
  - 標準的なマークダウン記法をそのまま使用
  - ドキュメントとコードが一体化
  - *Native markdown syntax - documentation and code unified*

- **スタックベース・逆ポーランド記法（RPN）**
  - コードブロック内はRPN形式
  - *Stack-based with Reverse Polish Notation in code blocks*

- **Vectorベースのフラクタル構造**
  - 全てのコンテナデータはネスト可能なVectorで表現
  - NumPy/APLスタイルのブロードキャスティング
  - **異種データ混在可能**: 数値・文字列・真偽値・Vectorを自由に組み合わせ可能
  - *All container data is nestable Vectors with broadcasting support*

- **完全精度の有理数演算**
  - すべての数値は内部的に分数として扱われ、丸め誤差なし
  - *All numbers internally treated as fractions - no rounding errors*

### 次元モデル / Dimension Model

| 次元 / Dim | 括弧 / Bracket | 構造 / Structure |
|:---:|:---:|:---|
| 0次元 | — | スタック（暗黙の最外殻） |
| 1次元 | `{ }` | `{ 1 2 3 }` |
| 2次元 | `( )` | `{ ( 1 2 ) ( 3 4 ) }` |
| 3次元 | `[ ]` | `{ ( [ 1 ] [ 2 ] ) }` |

### テクノロジースタック / Technology Stack

| コンポーネント / Component | 技術 / Technology |
|:---|:---|
| コアインタープリタ / Core Interpreter | Rust |
| ランタイム / Runtime | WebAssembly |
| フロントエンド / Frontend | TypeScript |
| ビルドツール / Build Tool | Vite |
| CI/CD | GitHub Actions |

---

## コード例 / Code Examples

### 基本的なVector / Basic Vector

```markdown
- 1
- 2
- 3
```

これは `{ 1 2 3 }` に変換されます。

### 2次元Vector（テーブル） / 2D Vector (Table)

```markdown
| 1 | 2 | 3 |
|---|---|---|
| 4 | 5 | 6 |
| 7 | 8 | 9 |
```

### ワード定義 / Word Definition

```markdown
# DOUBLE

値を2倍にする

\`\`\`ajisai
[ 2 ] *
\`\`\`
```

### データ処理パイプライン / Data Processing Pipeline

```markdown
- 1
- 2
- 3
- 4
- 5

---

\`\`\`ajisai
'[ 2 ] *' MAP
\`\`\`

---

\`\`\`ajisai
REVERSE
\`\`\`
```

結果: `{ 10 8 6 4 2 }`

### 完全なプログラム例 / Complete Program Example

```markdown
# SQUARE

自乗する

\`\`\`ajisai
DUP *
\`\`\`

# main

- 1
- 2
- 3
- 4
- 5

---

\`\`\`ajisai
'SQUARE' MAP
\`\`\`
```

結果: `{ 1 4 9 16 25 }`

---

## 組み込みワード一覧 / Built-in Words

### 算術演算 / Arithmetic
`+` `-` `*` `/` `MOD` `FLOOR` `CEIL` `ROUND`

### 形状操作 / Shape Manipulation
`SHAPE` `RANK` `RESHAPE` `TRANSPOSE` `FILL`

### Vector操作 / Vector Operations
`GET` `INSERT` `REPLACE` `REMOVE` `LENGTH` `TAKE` `SPLIT` `CONCAT` `REVERSE` `RANGE`

### 比較・論理演算 / Comparison & Logic
`=` `<` `<=` `>` `>=` `AND` `OR` `NOT`

### 高階関数 / Higher-Order Functions
`MAP` `FILTER` `FOLD` `UNFOLD`

### 形式変換 / Format Conversion
`STR` `NUM` `BOOL` `NIL` `CHARS` `JOIN`

### 日時操作 / DateTime
`NOW` `DATETIME` `TIMESTAMP`

### ワード管理 / Word Management
`DEF` `DEL` `?`

### 制御フロー / Control Flow
`TIMES` `WAIT` `:` `!`

### 入出力 / I/O
`PRINT`

### 操作対象指定 / Target Specification
`.` `..`

---

## ローカル開発 / Local Development

### 必要条件 / Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/) (v20以上推奨 / v20+ recommended)

### セットアップ / Setup

```bash
# リポジトリのクローン / Clone the repository
git clone https://github.com/masamoto1982/Ajisai.git
cd Ajisai

# 依存関係のインストール / Install dependencies
npm install

# WASMビルド / Build WASM
cd rust
wasm-pack build --target web --out-dir ../js/pkg
cd ..

# TypeScriptビルド / Build TypeScript
npm run build

# 開発サーバー起動 / Start development server
npx vite
```

### ビルド / Build

```bash
# プロダクションビルド / Production build
npx vite build
```

---

## ライセンス / License

[MIT License](LICENSE)

---

## 関連ドキュメント / Related Documentation

- [MVL_REFERENCE.md](Documentation/MVL_REFERENCE.md) - Markdown Vector Language リファレンス
- [UNIFIED_FRACTION_ARCHITECTURE.md](Documentation/UNIFIED_FRACTION_ARCHITECTURE.md) - 統一分数アーキテクチャの設計
- [DIMENSION_MODEL.md](Documentation/DIMENSION_MODEL.md) - 次元モデルの詳細
- [BROADCASTING.md](Documentation/BROADCASTING.md) - ブロードキャスティングの仕様
- [THREE_VALUED_LOGIC.md](THREE_VALUED_LOGIC.md) - 三値論理について
