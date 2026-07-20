# Phase 8C: DATA モジュール — design memo

Status: `[実装済み]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。

## 実施フェーズ

Phase 8C（引き継ぎ指示書 §15.3）: 表データ処理を **Module word** として実用化する。Core へ新しい表構文は
追加しない。§15.3 は「一度にすべて追加しない」「推奨順で進める」と定めており、unit ごとに別 PR で進める。

- **unit 1**: CSV 文字列と Record vector の相互変換。
  - `DATA@CSV-PARSE`     text → Record の vector（先頭行 = ヘッダ）。
  - `DATA@CSV-STRINGIFY` Record の vector → CSV text。
- **unit 2**: 列選択・行選択。
  - `DATA@SELECT` `[ table ] [ columns ] SELECT` → 指定列のみに射影。
  - `DATA@WHERE`  `[ table ] 'col' { pred } WHERE` → 指定列のセルに述語を適用し真の行のみ残す。
- **unit 3**: グループ化。
  - `DATA@GROUP` `[ table ] 'col' GROUP` → 指定列の distinct 値ごとに
    `{ 'key' <値> 'rows' <部分表> }` group record の vector（初出順）。
- **unit 4**: 結合。
  - `DATA@JOIN` `[ left ] [ right ] 'key' JOIN` → key 列で left/lookup join。
    match しない left 行（key 不在含む）は追加列を NIL `MissingField` で埋める。

後続 unit（sort-by、chunk）は別 PR。

## 設計

`rust/src/interpreter/data_ops/`（`mod.rs` = 実装、`tests.rs` = テスト）。既存資産（Vector / Record /
RecordShape / 文字列 = codepoint vector / Bubble / Kleene 真理値）だけで実装し、Core 構文・新 primitive を
増やさない。`JSON@PARSE` / `JSON@STRINGIFY`（純粋な text↔構造変換）と `FILTER`（述語 HOF）を範とする。

- **純粋変換**。ファイル読込はしない（既存 IO / Hosted capability の担当）。`Capabilities::PURE`,
  `WordPurity::Pure`, deterministic。
- **不正入力は raise しない**。`JSON@PARSE` と同じく、Bubble/NIL に射影する（SPEC §11.2）。
  CSV は矩形とみなす：ヘッダと列数が異なる行、または未閉じ引用は、黙ってデータを壊さず
  **parse 全体を NIL に射影**する（reason = `InvalidEncoding`）。SELECT / WHERE の非表入力も同様。
- **セルは text**。数値解釈は後続 unit の関心事（本 unit では変換しない）。
- **CSV コアは純関数**（`parse_csv_rows` / `record_vector_to_csv` / `encode_field`）で
  RFC 4180 準拠（`""` エスケープ、`,`/改行を含むフィールドの引用、CRLF/LF 行末）。インタープリタ
  非依存に単体テストできる。

### SELECT / WHERE（unit 2）

ファイル分割: query 語（SELECT / WHERE / GROUP とその補助関数）は `data_ops/query.rs` に置き、
共有ヘルパ（`build_record` / `vector_of` / `extract_stack_value` / `encoding_bubble`）は `mod.rs` に残す
（file-size budget 内に収めるため）。

- **SELECT** は各 Record を指定列へ射影する。列が存在しない場合、その列のセルは
  **NIL（reason = `MissingField`）** になる（結果は矩形を保つ）。列リストが vector でない・行が
  Record でない・入力が表でない場合は NIL に射影。
- **WHERE** は `FILTER` の述語実行機構（`extract_executable_code` / `execute_executable_code`）を再利用し、
  各行の**指定列のセル**を stack に載せて述語を実行する。結果が**確定的に真**の行のみ残す。
  false・**Kleene UNKNOWN**・**NIL（列不在）**はいずれも「真でない」ため行を落とす（SQL の WHERE 準拠）。
  結果は常に表：一致行 0 でも空表（NIL ではなく空 vector）を返し、下流の表操作が続けられる。
  述語実行はスクラッチ stack 上で行い、呼び出し側の stack・operation mode を復元する。

### GROUP（unit 3）

- **GROUP** は指定列のセルの text をキーに行を分割し、**初出順**で
  `{ 'key' <セル> 'rows' <部分表> }` group record の vector を返す。group table 自体が表なので
  SELECT / WHERE で更に処理できる。
- 列が存在しない（または空）行は 1 つの group にまとまり、その group の `key` は
  **NIL（reason = `MissingField`）**（初出セルの reason を保持）。非表入力・非 Record 行は NIL に射影。

### JOIN（unit 4）

- **JOIN** は left/lookup join：各 left 行を、key セルが一致する**最初の** right 行の列で拡張する
  （right は unique key と見なす）。追加列 = right 先頭行の列 − key − left 既存列（merged schema を安定化）。
- match しない left 行、および **key 列が存在しない** left 行は、追加列を
  **NIL（reason = `MissingField`）** で埋める。これが「join key が存在しない」理由（§15.3）であり、
  `ALGO@INDEX-OF` が検索ミスに使う reason と同一。**新しい `NilReason` は追加せず**、SPEC も変更しない
  （SPEC §11.2 が well-formed miss に `missingField` を割り当てる先例に一致）。
- `JOIN` は短縮名が core の文字列結合語 `JOIN` と衝突するため（`JSON@GET` と core `GET` と同様）、
  **修飾名 `DATA@JOIN`** で使う。
- 非表入力・非 Record 行は NIL に射影。left 空 → 空表。right 空 → 追加列なしで left をそのまま返す。

### NIL reason について（§15.3）

§15.3 は「欠損理由を汎用 NIL へ潰さない／新しい理由を追加する場合は仕様上の扱いを確認する」と定める。
本フェーズでは**新しい `NilReason` 変種を追加せず**、既存の意味に合致する変種を使う：

- CSV parse 失敗（ragged / 未閉じ引用）→ `InvalidEncoding`（`JSON@PARSE` 先例）。
- **列が存在しない** → `MissingField`（origin も `MissingField`）。record の key 欠落そのものであり、
  既存意味に一致する。SPEC・Core enum の変更を要さない。

「数値変換不可」「join key 不在」は後続 unit（型付きセル・JOIN）で中心に扱い、必要なら SPEC と協調して
変種を導入する。

## 互換性

- Core: 変更なし。新 Core 構文・新 primitive・新 `NilReason` ゼロ。`DATA` は Module であり、
  `'DATA' IMPORT` で `CSV-PARSE` / `CSV-STRINGIFY` / `SELECT` / `WHERE` / `GROUP` が使える
  （`JOIN` は core と衝突するため `DATA@JOIN`）。
- 語は `Stability::Experimental`（新規 Module。API は後続 unit で発展しうる）。
- 生成物: `docs/word-manifest.json`（unit 4 で +1 = 223 語）、`SKILL.md`、
  `docs/formalization-coverage.json`（`module.data.join` を Sketched で追加、223/223 分類）、
  `docs/primitive-test-map.json` を再生成。
- WASM / GUI / conformance / reference interpreter: 影響なし（conformance case は未追加。
  differential は core カテゴリのみ）。

## 必須テスト

`data_ops/tests.rs`:
- CSV 純コア（unit 1）: 単純表・末尾改行・CRLF・引用フィールド・空 text・未閉じ引用・ragged・
  ヘッダのみ・往復・非表 stringify・空 vector→空 text・`encode_field`。
- 実行経路（unit 1）: CSV 往復、ragged で NIL bubble。
- 実行経路（unit 2）: SELECT が指定列のみを順序通り残す・欠損列が NIL セルになる・非表で NIL、
  WHERE が列述語で行を残す・欠損列で全行 drop（空表）。
- 実行経路（unit 3）: GROUP が distinct 値ごとに初出順で group を作る・group 数・非表で NIL。
- 実行経路（unit 4）: JOIN が left 行を match で拡張・未 match は NIL セル・first-match・非表で NIL。

## 非対象（後続 unit / フェーズ）

- `DATA@SORT-BY` / `DATA@CHUNK`。← unit 5
- per-field NIL reason（数値変換不可）の SPEC 協調導入。多対多 join（現状は first-match の lookup join）。
- 数値・型付きセル解釈、CSV 方言（区切り文字・引用文字の選択）。

## 仕様上の未解決点

- 後続 unit で per-field NIL reason を追加する際、SPEC（`NilReason` / `AbsenceOrigin` の enumeration）への
  反映方針を確定する必要がある。本 unit の範囲外。
