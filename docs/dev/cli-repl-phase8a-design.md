# Phase 8A: interactive REPL — design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 8A: CLI とプロジェクト基盤（引き継ぎ指示書 §15.1）のうち、`ajisai repl` を実装する。

Phase 8 は一つの巨大 PR にせず 8A〜8C に分割する方針であり（§15）、8A 内の候補コマンド
（`repl` / `fmt` / `test` / `new`）も、プロセス規則（§4.8「フェーズ内で複数の PR 相当単位へ
分割してよい」）に従い独立してテスト可能な単位に分ける。本 PR はその最初の単位として、
8B（manifest / lockfile）に依存しない `repl` を対象とする。

## `ajisai repl` の要件（§15.1）と対応

- **Rust CLI で実装する。Python REPL をそのまま正典扱いしない。**
  → 本番 Core（`Interpreter::with_host(CliHostEnv)`）を駆動する。Python 参照は使わない。
- **ユーザー辞書・imports・stack をセッション内で保持する。**
  → 1 セッション = 1 個の永続 `Interpreter`。行ごとに `execute` を呼び、状態を持ち越す。
- **構造化診断を表示できる。**
  → エラー時は `message` を返し、`--json` では `{ status, stackDisplay, output, message }` を行ごとに出す。
- **非対話環境でもテスト可能な入出力分離を行う。**
  → 評価コアは純粋な `(session, line) -> ReplResponse`（I/O なし）。端末ドライバ
    `run_repl<R: BufRead, W: Write, E: Write>` はその薄いシェル。banner / prompt / help /
    `:reset` 通知は **stderr** へ出し、stdout は結果のみ（`run --json` と同じ pipe-safe 保証）。

## 構造

`rust/src/cli/repl.rs`。

- `ReplSession`: 永続 `Interpreter` を保持。`eval(line) -> ReplResponse` は行実行後の
  スタック表示と、その行が出した `PRINT` payload（累積ではなく行単位）を返す。
  行間で host effect が累積するため、実行前の長さを記録し差分だけを取る。
- meta-command: 先頭が `:` の行は host が処理し、言語表層とは厳密に分離する（言語語ではない）。
  `:help` / `:reset`（全 reset）/ `:quit`（EOF でも離脱）。他の `:x` は「unknown」通知のみ。
- `--json`: 評価行ごとに 1 JSON ドキュメント（`status` / `stackDisplay` / `output` / `message`）。
  text mode: output payloads → （エラー時）`error: <msg>` → スタック 1 行（空なら `(empty stack)`）。
- エラーはセッションを壊さない（失敗語の後も評価継続）。

## 言語意味論への非介入

REPL は host driver であり、Core 語彙も表層構文も一切追加・変更しない。`:` directive は
言語トークンではなく host 側の meta-command として分離される（§15.1 の「directive を採用しても
通常言語意味論と分離する」方針に沿う）。したがって表層構文・CLI JSON 契約・WASM は不変。

## 互換性

- 表層構文: 変更なし。
- CLI: `repl` サブコマンドと `--json` 出力形状を additive に追加（`docs/dev/agent-cli-output-contract.md` §16）。
  既存コマンドの出力は不変。
- WASM / GUI / conformance / reference interpreter: 影響なし。

## 必須テスト（`rust/src/cli/repl.rs` の `#[cfg(test)]`）

- スタックが行を跨いで保持される。
- 定義（DEF）が行を跨いで保持される。
- output が行単位（累積しない）。
- エラーが message を返し、セッションが使用可能なまま継続する。
- `:reset` がスタックを消去する。
- ドライバが pipe-safe（banner は stderr、stdout は 1 行 1 JSON）。

## 非対象（8A の後続単位／後続フェーズ）

- `ajisai fmt`（GUI `code-formatter.ts` と共通 corpus を正本にする formatter）。
- `ajisai test`（test manifest / `.expected.json` / コメント directive を用いた host runner）。
- `ajisai new`（プロジェクト scaffold。manifest 形式に依存するため 8B と併せて設計）。
- REPL の複数行入力継続（未閉じブロックの継続行）や履歴。現状は 1 行 = 1 評価。

## 仕様上の未解決点

なし。
