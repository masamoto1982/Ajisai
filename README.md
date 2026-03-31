![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")

# Ajisai

**Ajisai** は、Rust + WebAssembly で実装された **Fractional Dataflow** ベースのスタック指向言語です。  
構文は FORTH 由来の RPN（逆ポーランド記法）を採用し、値を「消費しながら残余を次へ流す」実行モデルを持ちます。

- Playground: https://masamoto1982.github.io/Ajisai/
- 正準仕様: `SPECIFICATION.md`

---

## この README の位置づけ

この README は **実装済み機能の概要** に限定し、詳細仕様は `SPECIFICATION.md` を参照します。

- 言語意味論・用語定義: `SPECIFICATION.md`
- 実行系実装: `rust/src/interpreter/`
- 組み込みワード定義: `rust/src/builtins/builtin-word-definitions.rs`
- Web UI 実装: `js/gui/`

---

## 現在の実装モデル（概要）

- 計算値の中心は `Fraction`（有理数）
- 実行は consumed / remainder の連鎖で進行
- 既定は消費モード（`,`）、分流は `,,`
- エラー抑止は `~`、Nil Coalescing は `=>`
- パイプライン可読化マーカーとして `==`

> 注: 詳密なアーキテクチャ（Data Plane / Semantic Plane、FlowToken 等）は `SPECIFICATION.md` を正としてください。

---

## 実装済みの主な機能

### 言語コア

- RPN（後置記法）
- カスタムワード定義（`DEF` / `DEL` / `?`）
- コードブロック（`:` / `;`）
- 高階関数（`MAP`, `FILTER`, `FOLD`）
- フロー経路制御（`ROUTE` — 分岐・反復の統一構造）

### データ・演算

- 位置操作: `GET`, `INSERT`, `REPLACE`, `REMOVE`
- 量/構造操作: `LENGTH`, `TAKE`, `SPLIT`, `CONCAT`, `REVERSE`, `RANGE`, `REORDER`, `COLLECT`, `SORT`
- 算術: `+`, `-`, `*`, `/`, `MOD`, `FLOOR`, `CEIL`, `ROUND`
- 比較/論理: `=`, `<`, `<=`, `AND`, `OR`, `NOT`
- 変換: `NUM`, `STR`, `BOOL`, `CHR`, `CHARS`, `JOIN`
- テンソル系: `SHAPE`, `RANK`, `RESHAPE`, `TRANSPOSE`, `FILL`

### 補助・モジュール

- 定数: `TRUE`, `FALSE`, `NIL`
- 実行補助: `EXEC`, `EVAL`, `WAIT`, `PRINT`
- 乱数/ハッシュ: `CSPRNG`, `HASH`
- 日時: `NOW`, `DATETIME`, `TIMESTAMP`
- モジュール読込: `IMPORT`（例: `music`, `json`, `io`）

---

## ミニサンプル

```ajisai
# カスタムワード
: [ 2 ] * ; 'DOUBLE' DEF
[ 1 2 3 4 ] 'DOUBLE' MAP   # -> [ 2 4 6 8 ]
```

```ajisai
# Safe mode + Nil coalescing
[ 1 2 3 ] [ 10 ] ~ GET => [ 0 ]
```

```ajisai
# パイプライン可読化
[ 1 2 3 4 5 ]
  == : [ 2 ] * ; MAP
  == : [ 5 ] < NOT ; FILTER
  == [ 0 ] : + ; FOLD
```

---

## ローカル開発

### 前提

- Rust（stable）
- `wasm-pack`
- Node.js 20+

### セットアップ

```bash
git clone https://github.com/masamoto1982/Ajisai.git
cd Ajisai
npm install
```

### WASM ビルド

```bash
cd rust
wasm-pack build --target web --out-dir ../js/pkg
cd ..
```

### Web 開発

```bash
npm run dev
```

### 型チェック / ビルド

```bash
npm run check
npm run build
```

### Rust テスト

```bash
cd rust
cargo test
```

---

## ドキュメント

- 言語仕様（正準）: `SPECIFICATION.md`
- 命名規約: `docs/guide-file-naming-convention.md`
- 移行メモ: `docs/migration-file-renaming-inventory.md`

---

## License

MIT（`LICENSE`）
