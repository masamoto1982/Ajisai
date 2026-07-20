# Phase 8B: manifest と lockfile — design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 8B: Manifest と lockfile（引き継ぎ指示書 §15.2）。`ajisai.toml`（宣言）と `ajisai.lock`
（実現された恒等性の記録）を導入し、単一ファイルでないプロジェクトを再現可能に実行できるようにする。
CLI には `ajisai build`（プロジェクト実行）と `ajisai lock`（lockfile 生成/検証）を追加する。

## 役割分離：manifest（意図）と lockfile（事実）

- **`ajisai.toml`** はプロジェクトの*意図*を宣言する。name / version / entry / specification、
  許可する capability（`[capabilities] allow`）、ローカル path 依存（`[dependencies]`）。
- **`ajisai.lock`** は実行から得た*事実*を記録する。各ソースの content identity、公開語の
  content identity、実際に必要とした capability、対象仕様バージョン、manifest schema version。

この分離により、(1) 複数ファイルのプロジェクトを再現可能に実行でき、(2) パッケージ identity が
名前と version だけでなく content に依存する（§15.4 の受け入れ条件）。

## manifest 形式（`manifest.rs`）

固定された小さな TOML サブセットを手書きパーサで読む（`toml` crate を足さず Core を軽量に保つ）。
受理する形は §19 の contract doc に記載。パーサは strict：未知セクション・未知キー・不正な値・
重複依存は全てエラーにし、typo が黙って「許可 capability」を変えないようにする。

capability 名は receipt の capability 語彙（`HostCapability::as_protocol_str`：`clock` /
`secureRandom` / `serial` / `audio` / `jsonExport` / `config` / `effect`）と同一。
これにより allow-list・ランタイムゲート・receipt の `requiredCapabilities` が同じ名前で話す。
`["io.output"]` 等の抽象名ではなく実際にランタイムがゲートする名前を使うことで、「capability を
プロジェクト単位で確認できる」が実質的な意味を持つ。

## 実行モデル（`project.rs`）

- 依存 `path` は manifest ディレクトリ相対の Ajisai *ソースファイル*を指す（§15.2「ローカル path
  dependency から始めてよい」）。依存ソースを宣言順に、その後 entry を、**同一辞書**へ実行する
  （フラットな直接依存名前空間）。推移的依存・ディレクトリ/サブ manifest 依存・リモート registry は
  本フェーズ非対象。
- capability 拘束は既存のゲートを再利用する。`ProjectHostEnv::has_capability` は
  `allowed.contains(cap) && CliHostEnv.has_capability(cap)`。manifest は capability を*絞る*ことは
  できるが、端末に無いデバイスを*生む*ことはできない。許可されない Hosted 語は通常の構造化
  missing-capability 経路（§2.5, `why: environment`）で失敗する。Core 意味論は不変。
- content identity と required capability は Phase 6 の receipt recorder で*観測*する（実行結果は
  変えない）。公開語は `word_identities`（FQN → content identity）を正本に列挙する。

## `ajisai build` / `ajisai lock`

- `build <dir>`: manifest 解決 → プロジェクト実行（拘束下）→ `run` と同一の envelope で描画。
  `ajisai.lock` があれば成功実行を照合し、drift なら拒否（再現性の担保）。
- `lock <dir> [--check]`: プロジェクトを実行し `ajisai.lock` を書く（正準 JSON。入力不変なら
  byte 安定 → 照合で drift 検出可能）。`--check` は書かずに最新性を検証。
- `--check` フラグは `fmt` と共有（`Opts::fmt_check`）。

exit code は contract doc §19 の表を正本とする。

## 互換性

- 言語仕様: 変更なし（新規語ゼロ）。`word-manifest` / `skill` / formalization coverage は不変。
- CLI: `build` / `lock` サブコマンドと `ajisai.toml` / `ajisai.lock` 形式を additive に追加
  （`agent-cli-output-contract.md` §19 / §1）。既存コマンドの出力は不変。`cmd_run` の結果描画は
  `render_completed_run` に切り出し、`run` と `build` で共有（`run` の出力は byte 不変）。
- WASM / GUI / conformance / reference interpreter: 影響なし（native CLI 専用）。

## 必須テスト

- `manifest_tests.rs`: full/minimal 解析、capability 重複排除、文字列内 `#`、必須キー欠落、
  未知セクション/キー、セクション前キー、依存 path 欠落、依存重複、非引用値。
- `lockfile.rs`（inline）: render 決定性＋末尾改行、必須フィールド、content-addressed source identity。
- `project_tests.rs`: lock 書込、`--check` の一致/不一致/未存在、capability 許可で実行成功／不許可で
  失敗、依存合成（依存語を entry が使用）、build の lock 照合と drift 検出、lock 無しでも実行、
  manifest 欠落＝usage error、未知 capability＝usage error、entry の言語エラー。

## 既知の限界

- 許可されない capability の missing-capability 診断は既存の汎用文言（「この実行環境は capability を
  提供していない」）を再利用する。厳密には「manifest が許可していない」だが、これを区別するには
  Core の診断文言変更が必要なため本フェーズでは行わない（Core 意味論不変を優先）。

## 非対象（後続）

- `ajisai new`（プロジェクト scaffold。本フェーズの manifest 形式に依存）。
- `ajisai add`（依存元と検証モデルが決まるまで導入しない／ローカル path 限定、§15.2）。
- 推移的依存・sub-manifest 依存・中央 registry・署名・自動アップロード（§15.2 の初期スコープ外）。
- Phase 8C（`DATA` モジュール）。

## 仕様上の未解決点

なし。
