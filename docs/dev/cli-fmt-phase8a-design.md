# Phase 8A: source formatter — design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 8A: CLI とプロジェクト基盤（引き継ぎ指示書 §15.1）のうち、`ajisai fmt`（フォーマッタ）を実装する。

8A の候補コマンドは独立してテスト可能な単位に分けて進めており（§4.8）、本 PR はその 2 つ目の単位。
1 つ目の `ajisai repl` に続き、8B に依存しない `fmt` を対象とする。

## `ajisai fmt` の要件（§15.1）と対応

- **固定表層と Sugar を維持する。意味が変わる書き換えをしない。**
  → 有意な空白（トークン間スペース・行頭インデント）だけを整える。改行は増減しない
    （`{ }` 内の改行は文区切り、SPEC §3.5）。文字列・コメントの内側は不変。`;` / `>CF` 等の
    Sugar を展開しない。安全に整形できない入力（未閉じ文字列・文字列内改行）は入力をそのまま返す。
- **GUI の `code-formatter.ts` と同じ期待結果を共有テストで固定する。Rust と TypeScript で
  独立実装する場合、共通 corpus を正本にする。**
  → `tests/formatter-corpus.json` を **正本**とし、Rust の `format_ajisai_source`（`cli/fmt.rs`）と
    TS の `formatAjisaiSource`（`gui/code-formatter.ts`）の両方を、この input→expected 集合で
    pin する（Rust: `matches_shared_corpus` / TS: `formatAjisaiSource shared corpus`）。
    これにより 2 実装が drift しない。
- **formatter を構文正典にしない。**
  → corpus が正本、コードは executor。formatter は表層構文を定義しない（正典は SPEC）。

## アルゴリズム

`cli/fmt.rs` は `gui/code-formatter.ts` の忠実な移植。

1. `scan_lines`: ソースを行×トークン列へ字句分解。文字列（`'...'`）とコメント（`#...`）は
   verbatim な 1 トークン。区切り `[ ] { } | ~ ^` は各々 1 トークン。`' # > = ( )` は
   context 依存のため触らず（`>CF` を誤分割しない）。未閉じ文字列・文字列内改行は `None`。
2. `render_lines`: bracket ネスト深さで行頭をインデント（先頭の閉じ括弧は 1 段戻す）。
   空行の連続は 1 行に畳み、前後の空行は落とす。改行の位置は保存。

`format_ajisai_source` は末尾改行を持たない content レベル結果を返し、CLI がファイル規約
（末尾改行 1 個。空ファイルは空のまま）を付す。

## CLI

`ajisai fmt <file>`:
- 既定: 正準形を stdout へ出力。
- `--write`: 既に正準でなければファイルを in-place で書き換え。
- `--check`: 検証のみ。正準なら exit 0、そうでなければ exit 1（メッセージは stderr）。read 失敗は exit 2。

plain text 出力であり `--json` は非対象。プログラムを実行しない。

## 互換性

- 表層構文: 変更なし。
- CLI: `fmt` サブコマンドと `--write` / `--check` フラグを additive に追加
  （`docs/dev/agent-cli-output-contract.md` §17 / §1）。既存コマンドの出力は不変。
- WASM / GUI / conformance / reference interpreter: 影響なし（fmt は native CLI 専用）。
  GUI formatter は既存のまま、共有 corpus で pin されただけ。

## 必須テスト

- Rust（`cli/fmt.rs`）: `matches_shared_corpus`（全 corpus ケース一致）、`is_idempotent_over_the_corpus`。
- TS（`gui/code-formatter.test.ts`）: `formatAjisaiSource shared corpus`（同一 corpus で pin）＋既存ケース。
- corpus はスペース整形・ネスト括弧分割・インデント・空行畳み・文字列/コメント verbatim・
  Sugar 非展開（`;` / `>CF`）・整形拒否（未閉じ/文字列内改行）を網羅。

## 非対象（8A の後続単位／後続フェーズ）

- `ajisai test`（test manifest / `.expected.json` / コメント directive の host runner）。
- `ajisai new`（プロジェクト scaffold。manifest 形式に依存＝8B と併せて設計）。
- 複数ディレクトリの一括整形・stdin からの整形・diff 表示。現状は 1 ファイル対象。

## 仕様上の未解決点

なし。
