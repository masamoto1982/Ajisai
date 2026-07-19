# Phase 8A: test runner — design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 8A: CLI とプロジェクト基盤（引き継ぎ指示書 §15.1）のうち、`ajisai test`（テストランナー）を実装する。

8A の候補コマンドは独立してテスト可能な単位に分けて進めており、本 PR はその 3 つ目にして 8A 最後の単位。
`ajisai repl` / `ajisai fmt` に続き、8B（manifest / lockfile）に依存しない `test` を対象とする。

## `ajisai test` の要件（§15.1）と対応

引き継ぎ指示書は「テストは host 側の runner が担い、言語コアに検証語（`ASSERT` 等）を足さない」ことを求める。

- **Core に検証語を追加しない。**
  → 期待値はソース中の `#@` **directive コメント**として書く。`#@` 行はインタープリタにとって
    ただの `#` コメント（SPEC §3.4）であり、実行時には無視される。`@` マーカーを読むのは host runner だけ。
    したがって同じテストファイルは `ajisai run` でもそのまま走り、directive は素通りする。
- **本番と同じ実行経路を使う。**
  → runner は `run` と同一の production Core（`Interpreter::with_host(CliHostEnv)` → `execute`）を駆動する。
    テスト専用の実行モードや特別扱いは無い。
- **harness を言語仕様から分離する。**
  → 期待値の照合（status / stack / output / error 部分一致）はすべて host 側の比較で、
    値モデルや Tier 観測性には一切触れない。Semantic Firewall・Water いずれにも影響しない。

## directive

1 行 1 directive、ファイル中どこでも可：

| directive | 意味 |
|---|---|
| `#@ status ok` \| `#@ status error` | 期待する結果。既定は `ok`（エラーなく走ること）。 |
| `#@ stack <display>` | 期待する最終スタック。空白区切りの display 文字列（`stackDisplay` と同じ描画）。 |
| `#@ output <line>` | 期待する `PRINT` payload。繰り返し可。順序込みで全一致を要求。 |
| `#@ error <substring>` | 実行が失敗し、メッセージが `<substring>` を含むこと。`status error` を含意。 |

未知キーワード・空の `#@`・未知の `status` 値はそのファイルの失敗として報告する
（typo が黙って pass しない）。`@` の付かない素の `#` コメントは directive ではない。

## アルゴリズム

`cli/test_runner.rs`:

1. `parse_directives`: 各行を `trim_start` して `#@` prefix を剥がし、最初の空白で keyword / value に分割。
   keyword ごとに `Expectations`（status / stack / output / error_contains / directive_errors）を組み立てる。
2. `run_test_source`（純関数、FS 非依存でテスト可能）: source を実行し、
   - status（既定 ok）、
   - `error <substring>` の部分一致、
   - `stack_display(&interp).join(" ")` と期待 stack、
   - `print_payloads(&interp)` と期待 output 列、
   を照合。不一致と directive エラーを `failures` に積む。空なら pass。
3. `collect_dir`: ディレクトリを sorted・再帰で walk し `*.ajisai` を収集（決定的順序）。
4. `render_report`: text（`PASS`/`FAIL` ＋サマリ）または `--json`
   （`schemaVersion` / `status` / `total` / `passed` / `failed` / `results`）。

## CLI

`ajisai test <file-or-dir>`:
- ディレクトリ: `*.ajisai` を再帰収集して全実行。ファイル: 拡張子を問わず実行。
- 既定は text レポート、`--json` で単一 JSON ドキュメント（pipe-safe）。
- exit 0 = 全 pass、1 = 1 件以上 fail、2 = usage エラー
  （パス不在・ディレクトリ読取失敗・`.ajisai` が 0 件）。

## 互換性

- 言語仕様: 変更なし（新規語ゼロ）。`word-manifest` / `skill` / formalization coverage は不変。
- CLI: `test` サブコマンドを additive に追加（`agent-cli-output-contract.md` §18 / §1）。既存コマンドの出力は不変。
- WASM / GUI / conformance / reference interpreter: 影響なし（test は native CLI 専用）。

## 必須テスト

`cli/test_runner.rs` の `#[cfg(test)]`：pass（stack＋output）、stack 不一致、output 不一致、
既定 status＝成功、想定外エラーで fail、`status error` で pass、`error` 部分一致、部分一致失敗、
`status error` なのに成功で fail、未知 directive の報告、素の `#` コメントは directive でない。

## 非対象（後続フェーズ）

- `.expected.json` 等の外部期待ファイル形式。現状は directive コメントのみ。
- `ajisai new`（プロジェクト scaffold。manifest 形式に依存＝8B と併せて設計）。
- Phase 8C（`DATA` モジュール）。

## 仕様上の未解決点

なし。
