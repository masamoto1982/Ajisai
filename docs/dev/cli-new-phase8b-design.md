# `ajisai new`: project scaffold — design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 8A の候補コマンド（引き継ぎ指示書 §15.1）のうち最後の `ajisai new` を実装する。scaffold は
Phase 8B の manifest 形式に依存するため、8B（`ajisai.toml` / `ajisai.lock`）の後に置いた。

## 要件と対応

`ajisai new <dir>` は新規プロジェクトの雛形を作る。

- **8B manifest 形式の有効なインスタンスを生成する。**
  → `<dir>/ajisai.toml` を出力（`[project]` name/version/entry、`[capabilities] allow=["effect"]`、
    `[dependencies]` は使用例コメント）。name はパス末尾要素。`manifest.rs` のパーサがそのまま受理する。
- **生成直後に実行できる。**
  → `<dir>/src/main.ajisai` に、宣言 capability の下で成功する最小プログラム
    （`[ 'Hello from <name>!' ] PRINT`）を出力。`ajisai build <dir>` が即座に成功する
    （テストで `parse_manifest` 通過と `cmd_build == 0` を固定）。
- **上書きしない。**
  → 既存 `<dir>` は usage error（exit 2）。

name は directory 名かつ manifest 文字列になるため `[A-Za-z0-9._-]` のみ許可し、空・`.`・`..` を拒否
（manifest パーサは `"` / `\` を別途拒否する）。

## CLI

`ajisai new <dir>`:
- 生成物: `<dir>/ajisai.toml` と `<dir>/src/main.ajisai`。
- 成功時に生成物と次手順（`ajisai build` / `ajisai lock`）を表示。
- exit 0 = 生成成功、2 = usage error（不正 name・既存パス・I/O 失敗）。プログラムは実行しない。
  `--json` 非対象（テンプレート出力のみ）。

## 互換性

- 言語仕様: 変更なし（新規語ゼロ）。生成物はテンプレートテキストのみ。
- CLI: `new` サブコマンドを additive に追加（`agent-cli-output-contract.md` §19 / §1）。既存出力は不変。
- WASM / GUI / conformance / reference interpreter: 影響なし。

## 必須テスト

`new_project.rs`（inline）: scaffold→`parse_manifest` 通過→`cmd_build == 0`（生成物が本当に走る）、
既存ディレクトリ拒否、name バリデーション（空 / `.` / `..` / 空白 / `"` / `/` を拒否、`ok-name_1.2` を許可）。

## 非対象（後続）

- テンプレートの選択肢（lib/app 等の複数雛形）、`.gitignore` や README の生成。現状は最小の 1 雛形。
- `ajisai add`（依存追加。依存元と検証モデル未確定、§15.2）。
- Phase 8C（`DATA` モジュール）。

## 仕様上の未解決点

なし。
