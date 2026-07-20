# Phase 8C: DATA モジュール — design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 8C（引き継ぎ指示書 §15.3）: 表データ処理を **Module word** として実用化する。Core へ新しい表構文は
追加しない。§15.3 は「一度にすべて追加しない」「推奨順で進める」と定めており、本 PR はその **unit 1**：
CSV 文字列と Record vector の相互変換のみを対象とする。

- `DATA@CSV-PARSE`     text → Record の vector（先頭行 = ヘッダ）。
- `DATA@CSV-STRINGIFY` Record の vector → CSV text。

後続 unit（列/行選択、group、join、chunk）は別 PR。

## 設計

`rust/src/interpreter/data_ops.rs`。既存資産（Vector / Record / RecordShape / 文字列 = codepoint
vector / Bubble）だけで実装し、Core 構文・新 primitive を増やさない。`JSON@PARSE` / `JSON@STRINGIFY`
（純粋な text↔構造変換）を範とする。

- **純粋変換**。ファイル読込はしない（既存 IO / Hosted capability の担当）。`Capabilities::PURE`,
  `WordPurity::Pure`, deterministic。
- **不正入力は raise しない**。`JSON@PARSE` と同じく、Bubble/NIL に射影する（SPEC §11.2）。
  CSV は矩形とみなす：ヘッダと列数が異なる行、または未閉じ引用は、黙ってデータを壊さず
  **parse 全体を NIL に射影**する（reason = `InvalidEncoding`）。
- **セルは text**。数値解釈は後続 unit の関心事（本 unit では変換しない）。
- **CSV コアは純関数**（`parse_csv_rows` / `record_vector_to_csv` / `encode_field`）で
  RFC 4180 準拠（`""` エスケープ、`,`/改行を含むフィールドの引用、CRLF/LF 行末）。インタープリタ
  非依存に単体テストできる。

### NIL reason について（§15.3）

§15.3 は「欠損理由を汎用 NIL へ潰さない／新しい理由を追加する場合は仕様上の扱いを確認する」と定める。
本 unit では、DATA 固有の per-field reason（列が存在しない・数値変換不可・join key 不在）は登場しない。
これらは SELECT / WHERE / JOIN の unit が中心に扱うため、そこで **SPEC と協調して**必要な `NilReason`
変種を導入する。CSV parse の失敗（ragged / 未閉じ引用）は「text が矩形 CSV として読めない」ため既存の
`InvalidEncoding` に射影する（`JSON@PARSE` の先例と一致し、Core enum・SPEC 変更を要さない）。

## 互換性

- Core: 変更なし。新 Core 構文・新 primitive ゼロ。`DATA` は Module であり、`'DATA' IMPORT` で
  `CSV-PARSE` / `CSV-STRINGIFY` が使える（未 import の `DATA@CSV-PARSE` は他 Module と同様 Unknown）。
- 語は `Stability::Experimental`（新規 Module。API は後続 unit で発展しうる）。
- 生成物: `docs/word-manifest.json`（+2 語 = 219）、`SKILL.md`、`docs/formalization-coverage.json`
  （`module.data.csv-parse` / `csv-stringify` を Sketched で追加、219/219 分類）、
  `docs/primitive-test-map.json`（data_ops.rs が既存 primitive を新たに exercise）を再生成。
- WASM / GUI / conformance / reference interpreter: 影響なし（conformance case は未追加。
  differential は core カテゴリのみ）。

## 必須テスト

`data_ops.rs`（inline）:
- 純コア: 単純表・末尾改行・CRLF・引用フィールド（`,`/`"`/改行）・空 text・未閉じ引用（None）・
  ragged 行（None）・ヘッダのみ（空表）・往復（quote 要フィールド込み）・非表の stringify（None）・
  空 vector→空 text・`encode_field` の引用判定。
- 実行経路: `'DATA' IMPORT CSV-PARSE CSV-STRINGIFY` の往復、ragged 入力で NIL bubble。

## 非対象（後続 unit / フェーズ）

- `DATA@SELECT` / `DATA@WHERE`（列選択・行選択）。← unit 2
- `DATA@GROUP` / `DATA@JOIN` / `DATA@SORT-BY` / `DATA@CHUNK`。← unit 3–5
- per-field NIL reason（列不在・数値変換不可・join key 不在）の SPEC 協調導入。
- 数値・型付きセル解釈、CSV 方言（区切り文字・引用文字の選択）。

## 仕様上の未解決点

- 後続 unit で per-field NIL reason を追加する際、SPEC（`NilReason` / `AbsenceOrigin` の enumeration）への
  反映方針を確定する必要がある。本 unit の範囲外。
