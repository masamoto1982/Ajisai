![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")

# Ajisai

**FORTHにインスパイアされた、スタックベースのプログラミング言語**

*A stack-based programming language inspired by FORTH*

🔗 **デモ / Demo:** [https://masamoto1982.github.io/Ajisai/](https://masamoto1982.github.io/Ajisai/)

---

## 🤖 AI駆動開発について / About AI-Driven Development

> **このプロジェクトの実装の大半はAI（Claude）によって行われています。**
> 設計方針の決定から、Rust/TypeScriptのコード実装、テストケースの作成、ドキュメント整備まで、
> 人間とAIの協働によって開発が進められています。
>
> *The majority of this project's implementation was done by AI (Claude).*
> *From design decisions to Rust/TypeScript code implementation, test case creation, and documentation,*
> *this project is developed through human-AI collaboration.*

---

## 概要 / Overview

Ajisaiは、WebAssembly上で動作するスタックベースのインタープリタと、Webベースの対話的なGUIを提供するプログラミング言語です。

*Ajisai provides a stack-based interpreter running on WebAssembly and an interactive web-based GUI.*

「Ajisai（紫陽花）」という名前は、小さなワードが集まって機能を形成するFORTHの特徴を、小さな花が集まって一つの花房を形作る紫陽花に例えています。（※紫陽花の花びらに見える部分は、実際には萼（がく）です）

*The name "Ajisai" (hydrangea) metaphorically represents FORTH's characteristic of small words coming together to form functionality, like how small flowers come together to form a hydrangea cluster. (Note: What appears to be petals are actually sepals.)*

---

## 特徴 / Features

### 言語設計 / Language Design

- **スタックベース・逆ポーランド記法（RPN）**
  - FORTHスタイルのスタック操作
  - *Stack-based with Reverse Polish Notation, FORTH-style*

- **Vectorベースのフラクタル構造**
  - 全てのコンテナデータはネスト可能なVectorで表現（LISPのリスト構造に通ずる設計思想）
  - 括弧 `[ ]` のネストで多次元を表現し、テンソル的な操作（SHAPE, RESHAPE等）をサポート
  - **異種データ混在可能**: `[ 1 'hello' TRUE [ 2 3 ] ]` のように、数値・文字列・真偽値・Vectorを自由に組み合わせ可能
  - NumPy/APLスタイルのブロードキャスティング
  - *All container data is represented as nestable Vectors (similar to LISP's list structure). Bracket `[ ]` nesting expresses dimensions, with tensor-like operations (SHAPE, RESHAPE, etc.) supported. Heterogeneous data mixing is allowed.*

- **最大4次元までの次元制限**
  - 人間の認知限界に基づき、操作可能な次元を4次元に制限
  - 5次元以上のネストはエラーとなる
  - *Dimension limit of 4 (based on human cognitive limits). Nesting beyond 5 dimensions results in an error.*

| 次元 / Dim | 軸名 / Axis | 構造 / Structure | 例 / Example |
|:---:|:---:|:---|:---|
| 1次元 | col | Vector | `[ 1 2 3 ]` |
| 2次元 | row, col | ネストされたVector | `[ [ 1 2 ] [ 3 4 ] ]` |
| 3次元 | layer, row, col | 2段ネスト | `[ [ [ 1 2 ] [ 3 4 ] ] ]` |
| 4次元 | time, layer, row, col | 3段ネスト（最大） | `[ [ [ [ 1 ] ] ] ]` |

- **完全精度の有理数演算**
  - すべての数値は内部的に分数（Fraction）として扱われ、丸め誤差なし
  - 非常に大きな数値も処理可能
  - *All numbers internally treated as fractions - no rounding errors, capable of handling extremely large numbers*

- **静的型付け（型宣言・型推論不要）**
  - システムが認識するのは：ワード、Vector、真偽値、数値、文字列、Nil
  - *Statically typed: words, vectors, booleans, numbers, strings, and Nil*

- **組み込みワードの保護**
  - 組み込みワードは削除や上書きが不可能
  - *Built-in words cannot be deleted or overwritten*

### 可視化機能 / Visualization

- **深度別ブラケット表示**: `[ ]` → `{ }` → `( )` → `[ ]` ...（3レベルごとに循環）
- *Depth-based bracket styles for visual clarity*

- **リアルタイム状態表示**: スタック、辞書、メモリ使用量をGUIで確認可能
- *Real-time state display: stack, dictionary, memory usage in GUI*

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

### Vector演算 / Vector Operations

```ajisai
# Vectorの作成 / Creating vectors
[ 1 2 3 ]               # 1次元Vector / 1D vector: shape [3]
[ [ 1 2 ] [ 3 4 ] ]     # ネストされたVector（行列的構造） / Nested vector (matrix-like): shape [2, 2]

# 異種データ混在 / Heterogeneous data
[ 1 'hello' TRUE [ 2 3 ] ]   # 数値、文字列、真偽値、Vectorを混在可能

# ブロードキャスティング算術演算 / Broadcasting arithmetic
[ 5 ] [ 1 2 3 ] +       # → [ 6 7 8 ]
[ [ 1 2 3 ] [ 4 5 6 ] ] [ 10 20 30 ] +
# → [ [ 11 22 33 ] [ 14 25 36 ] ]

# 形状操作（テンソル的操作） / Shape manipulation (tensor-like operations)
[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE      # → [ 2 3 ]
[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE    # → [ [ 1 2 3 ] [ 4 5 6 ] ]
[ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE  # → [ [ 1 4 ] [ 2 5 ] [ 3 6 ] ]
```

### カスタムワード定義 / Custom Word Definition

```ajisai
# 2倍にするワードを定義 / Define a word that doubles a value
[ '[ 2 ] *' ] 'DOUBLE' DEF

# 使用例 / Usage
[ 5 ] DOUBLE    # → [ 10 ]

# 高階関数との組み合わせ / Combine with higher-order functions
[ 1 2 3 4 5 ] 'DOUBLE' MAP    # → [ 2 4 6 8 10 ]
```

### 制御構造（ガード） / Control Structure (Guards)

```ajisai
# 条件分岐：偶数ならTRUE、奇数ならFALSE / Conditional: TRUE if even, FALSE if odd
[ '[ 2 ] MOD [ 0 ] =' ] 'EVEN?' DEF

[ 4 ] EVEN?    # → [ TRUE ]
[ 7 ] EVEN?    # → [ FALSE ]
```

---

## 組み込みワード一覧 / Built-in Words

### 算術演算 / Arithmetic
`+` `-` `*` `/` `MOD` `FLOOR` `CEIL` `ROUND`

### 形状操作（テンソル的操作） / Shape Manipulation (Tensor-like Operations)
`SHAPE` `RANK` `RESHAPE` `TRANSPOSE` `FILL`

### Vector操作 / Vector Operations
`GET` `INSERT` `REPLACE` `REMOVE` `LENGTH` `TAKE` `SPLIT` `CONCAT` `REVERSE` `RANGE`

### 比較・論理演算 / Comparison & Logic
`=` `<` `<=` `>` `>=` `AND` `OR` `NOT`

### 高階関数 / Higher-Order Functions
`MAP` `FILTER` `FOLD` `UNFOLD`

### 型変換 / Type Conversion
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

### 入力ヘルパー / Input Helpers
`'` `SCALAR` `VECTOR` `MATRIX` `TENSOR`

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

- [DIMENSION_MODEL.md](Documentation/DIMENSION_MODEL.md) - 次元モデルの詳細
- [BROADCASTING.md](Documentation/BROADCASTING.md) - ブロードキャスティングの仕様
- [TYPE_SYSTEM_OPTIMIZATION.md](Documentation/TYPE_SYSTEM_OPTIMIZATION.md) - 型システムの最適化
- [THREE_VALUED_LOGIC.md](THREE_VALUED_LOGIC.md) - 三値論理について
