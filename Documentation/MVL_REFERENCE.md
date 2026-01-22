# Markdown Vector Language (MVL) Reference

MVL（Markdown Vector Language）は、Ajisaiプログラミング言語のためのマークダウンベースの構文です。
標準的なマークダウン記法を使用して、データ構造とプログラムロジックを自然に表現できます。

*MVL (Markdown Vector Language) is a markdown-based syntax for the Ajisai programming language.*
*Using standard markdown notation, you can naturally express data structures and program logic.*

---

## 基本概念 / Basic Concepts

### データとコード / Data and Code

MVLでは、マークダウンの要素が自動的にAjisaiのデータ構造に変換されます：

| マークダウン要素 | Ajisai構造 | 説明 |
|:---|:---|:---|
| リスト `-` | Vector | 1次元以上のベクター |
| テーブル `\|` | 2D Vector | 2次元ベクター（行列） |
| コードブロック ` ``` ` | RPN式 | 逆ポーランド記法のコード |
| 見出し `#` | ワード定義 | カスタムワードの定義 |
| 水平線 `---` | パイプライン | 処理の連結 |

---

## Vector（ベクター） / Vectors

### 1次元ベクター / 1D Vector

```markdown
- 1
- 2
- 3
```

これは `[ 1 2 3 ]` に変換されます。

### 2次元ベクター（ネスト） / 2D Vector (Nested)

```markdown
- - 1
  - 2
- - 3
  - 4
```

これは `[ [ 1 2 ] [ 3 4 ] ]` に変換されます。

### テーブル形式 / Table Format

```markdown
| 1 | 2 | 3 |
|---|---|---|
| 4 | 5 | 6 |
| 7 | 8 | 9 |
```

これは `[ [ 4 5 6 ] [ 7 8 9 ] ]` に変換されます（ヘッダー行は除外）。

### 異種データ / Heterogeneous Data

```markdown
- 42
- hello
- TRUE
- - nested
  - data
```

数値、文字列、真偽値、ネストされたベクターを自由に組み合わせることができます。

---

## ワード定義 / Word Definitions

### 基本的なワード定義 / Basic Word Definition

見出しがワード名になり、コードブロックが定義になります：

```markdown
# DOUBLE

値を2倍にする

\`\`\`ajisai
[ 2 ] *
\`\`\`
```

### パラメータ付きワード / Words with Parameters

```markdown
# SQUARE

自乗する

\`\`\`ajisai
DUP *
\`\`\`
```

使用例：
```markdown
- 5

---

\`\`\`ajisai
SQUARE
\`\`\`
```

結果: `[ 25 ]`

---

## パイプライン / Pipelines

水平線 `---` を使用してデータとコードを連結します：

```markdown
- 1
- 2
- 3

---

\`\`\`ajisai
[ 2 ] *
\`\`\`

---

\`\`\`ajisai
REVERSE
\`\`\`
```

この例では：
1. `[ 1 2 3 ]` をスタックにプッシュ
2. 各要素を2倍 → `[ 2 4 6 ]`
3. 逆順に → `[ 6 4 2 ]`

---

## mainセクション / Main Section

`# main` または無名ブロックがプログラムのエントリポイントになります：

```markdown
# DOUBLE

\`\`\`ajisai
[ 2 ] *
\`\`\`

# main

- 5
- 10
- 15

---

\`\`\`ajisai
'DOUBLE' MAP
\`\`\`
```

結果: `[ 10 20 30 ]`

---

## コードブロック / Code Blocks

### RPN式 / RPN Expressions

コードブロックはAjisaiのRPN（逆ポーランド記法）で記述します：

```markdown
\`\`\`ajisai
[ 5 ] [ 3 ] +
\`\`\`
```

### 言語指定 / Language Specification

`ajisai` または `rpn` を指定します：

```markdown
\`\`\`ajisai
[ 1 2 3 ] 'DOUBLE' MAP
\`\`\`
```

---

## 実践的な例 / Practical Examples

### 階乗計算 / Factorial Calculation

```markdown
# FACTORIAL

階乗を計算する（再帰的定義）

\`\`\`ajisai
: DUP [ 1 ] <=
: DROP [ 1 ]
: DUP [ 1 ] - FACTORIAL *
\`\`\`

# main

- 5

---

\`\`\`ajisai
FACTORIAL
\`\`\`
```

結果: `[ 120 ]`

### フィボナッチ数列 / Fibonacci Sequence

```markdown
# FIB

n番目のフィボナッチ数を返す

\`\`\`ajisai
: DUP [ 2 ] <
:
: DUP [ 1 ] - FIB SWAP [ 2 ] - FIB +
\`\`\`

# main

- 10

---

\`\`\`ajisai
FIB
\`\`\`
```

### データ処理パイプライン / Data Processing Pipeline

```markdown
# main

| 名前 | 年齢 | 得点 |
|------|------|------|
| Alice | 25 | 85 |
| Bob | 30 | 92 |
| Carol | 28 | 78 |

---

\`\`\`ajisai
'[ [ 2 ] GET ]' MAP
\`\`\`

---

\`\`\`ajisai
'[ ]' FOLD [ 0 ] +
\`\`\`
```

得点列を抽出し、合計を計算します。

---

## 組み込みワード一覧 / Built-in Words

### 算術演算 / Arithmetic
`+` `-` `*` `/` `MOD` `FLOOR` `CEIL` `ROUND`

### ベクター操作 / Vector Operations
`GET` `INSERT` `REPLACE` `REMOVE` `LENGTH` `TAKE` `SPLIT` `CONCAT` `REVERSE` `RANGE`

### 形状操作 / Shape Manipulation
`SHAPE` `RANK` `RESHAPE` `TRANSPOSE` `FILL`

### 比較・論理 / Comparison & Logic
`=` `<` `<=` `>` `>=` `AND` `OR` `NOT`

### 高階関数 / Higher-Order Functions
`MAP` `FILTER` `FOLD` `UNFOLD`

### 型変換 / Type Conversion
`STR` `NUM` `BOOL` `NIL` `CHARS` `JOIN`

### 制御 / Control
`TIMES` `WAIT`

### 入出力 / I/O
`PRINT`

### その他 / Others
`DEF` `DEL` `?` `RESET` `.` `..` `!`

---

## 内部動作 / Internal Behavior

MVLドキュメントは以下のように処理されます：

1. **パース**: マークダウンを構文解析し、MVL ASTに変換
2. **変換**: MVL ASTをAjisaiコード（RPN形式）に変換
3. **実行**: 変換されたコードをインタープリタで実行

この設計により、マークダウンの読みやすさとAjisaiの表現力を両立しています。

---

## 関連ドキュメント / Related Documentation

- [UNIFIED_FRACTION_ARCHITECTURE.md](UNIFIED_FRACTION_ARCHITECTURE.md) - 統一分数アーキテクチャ
- [DIMENSION_MODEL.md](DIMENSION_MODEL.md) - 次元モデル
- [BROADCASTING.md](BROADCASTING.md) - ブロードキャスティング
