# Playground 検証レポート（外部）の妥当性確認と改修（2026-09）

Status: 非正典・`[方針記録]`。本書は Ajisai の意味論も互換性方針も定義しない。
正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。

## 0. 出所

ブラウザ拡張（Claude in Chrome）上の別セッションが、Playground に約40件の
プログラムを流し、MCP 面は README・`spec/words.json`・同じ WASM コアの挙動から
静的に読んだ検証レポート（8 件の指摘）。本書はその**妥当性を独立に確認した
結果**と、そこから実施した改修の記録である。

先行する `mcp-hard-use-findings-work-order-2026-09.md` とは別の検証であり、
所見番号も別体系である。**同名の F-4 / F-5 が両文書にあるが、別の指摘である。**

## 1. 判定

| レポートの指摘 | 判定 | 実際のところ |
| --- | --- | --- |
| 無理数の表示が文書と食い違う | 部分的に妥当（原因が違う） | 表示は3系統ではなく2系統。Playground は正規形を根号で描き、正準表示は平坦な連分数 `[ 1; 2, 2, … ]`。予約記号 `( )` のエラー文は**現行コードと一致**しており、古かったのは ja Reference 本文だけ（入れ子連分数はコード上どこにも無い） |
| `PI` / `PROBE` が未文書 | 無効 | `docs/word-reference.md` に両方とも記載済み（`spec/words.json` から生成）。ただし裏に別の実バグがあった（§2.1） |
| 診断の when / reason が矛盾に見える | 妥当 | 文言が両者を役割で区別していなかった。§2.2 |
| 宣言済み条件が `Unknown (custom)` になる | 妥当（現行の穴） | §2.3。先行文書の F-4（NIL 射影側）修正は別経路であり、raise 側は手つかずだった |
| タイムアウトに診断が付かない / 上限差が見えない | 部分的に妥当 | 診断が付かないのは事実（§2.4）。ただし「上限差が見えない」は誤り——ホストプロファイルのバッジは以前から存在する |
| 実行後に Output へ自動切替 | 妥当 | 挙動としては確認。今回は未着手（§3） |
| Reset がネイティブ confirm / 省略マーカーが英語のまま | 妥当 | マーカーは文書側を実態に合わせた（§2.5）。Reset のモーダル化は未着手 |
| SORT の決定可能性の記述が矛盾 | 無効 | `SPECIFICATION.html` は代数体上の全域性と計算可能実数の UNKNOWN を書き分けており、無限定の「比較は常に TRUE/FALSE」はどこにも無い |
| MCP が npm 未公開・導線が README リンクのみ | 妥当 | 事実。今回は未着手（§3） |

## 2. 実施した改修

### 2.1 `PI` の失敗原因の誤帰属（レポートが症状だけ拾っていたもの）

`PI` 単独実行はインタプリタでは成功する（`ajisai agent compute` で確認）。
Playground だけが失敗して見えたのは、実行の**後段**でワーカーが取る不可逆な
スタックスナップショットが Tier-2 計算可能実数を拒否し、その例外が実行と同じ
`try` に落ちて「`PI` の実行が失敗した」として報告されていたためである。

- ワーカーはスナップショットの失敗を実行の失敗から切り離し、`stackSnapshotError`
  として結果に載せる。
- `syncInterpreterState` はそのとき状態を適用しない。観測形式は復元に使わない
  設計（SPEC §2.3）のため、適用すればスタックが空になり、ユーザーが実行前に
  持っていた値まで失われる。実行前のまま保つのは、失敗した実行と同じ答えである。
- `describeSnapshotRefusal` が「プログラムは動いた。残せないのは結果の方だ」を
  述べる。

### 2.2 診断の when / reason が矛盾に見える文言

`declared_checks` の `checkDeclaredProjection` を、条件（when）と結果側の理由
（reason）を役割名で呼び分け、「対で宣言された別々の項目であり、食い違いではない」
と明示する文言に改めた。実装は元から正しく、直したのは見せ方だけである。

### 2.3 宣言済み条件が診断に出ない（raise 側）

`spec/words.json` は Word ごとに `errorWhen` を宣言しているのに、raise 側は
`AjisaiError::from("MAP: expected return value, got empty stack")` のような
文字列であり、`ErrorCategory::Custom` → `CauseClass::Unknown` に落ちていた。
`why` も `aiDiagnostic.kind` も「読めば分かる」以上のことを言えていなかった。

- `AjisaiError::DeclaredCondition` を追加。raise 側が自分の宣言条件を名指しする。
- `ErrorCategory::Declared` の protocol 表記は**条件名そのもの**であり、
  `word_contract` が公開する語彙と同じ語で答える。
- `cause_class_for_declared_condition` は条件名の語彙（`spec/words.json` の
  errorWhen 31 語）を分類する。Word ごとの手書き表ではないので、同じ条件を
  宣言する新しい Word は書いた日から分類される。
- `declared_condition_vocabulary_is_classified` が、仕様が分類のない条件を
  増やした瞬間に落ちる。

**適用範囲（重要）**: 今回置き換えたのは、レポートが実際に踏んだ系統だけである
——高階ワード（MAP・FILTER・FOLD・ALL・ANY）のブロック契約違反と、コード
オペランドの `notExecutable`（`PROBE` を含む）。`NUM` / `COND` / `RANGE` などの
raise は従来どおり `custom` で、宣言条件の一覧を出す従来の fallback
（`checkDeclaredErrorConditions`）が働く。仕組みは入ったので、残りは順次
置き換えればよい。golden の "a raise the category cannot classify..." は
その fallback 経路を pin しているケースである。

### 2.4 実時間タイムアウトに診断が付かない

5 秒のウォールクロックはインタプリタの予算ではなく、ワーカーを外から
terminate する Playground 固有のガードである。Rust 側は診断を組む前に消える
ので、診断はホスト側で書くしかない。

- `ExecutionTimeoutError`（`src/workers/execution-timeout.ts`）で型として区別。
- `describeTimeoutDiagnosis` が、他のエラーと同じ骨格（when / where / why /
  next）で「どのガードが止めたか」「これは言語の上限ではない」「一括操作へ」
  「`QUANTIZE` で刈る」を述べる。
- ホストプロファイルのバッジに `executionTimeoutMs` を併記した。`host_profile()`
  はインタプリタの上限しか知らないため、このガードだけが一覧から漏れていた。

### 2.5 Reference 日本語版の同期

- 入れ子連分数 `( 1 ( 2 ( … ) ) )` の記述を削除し、実際の2系統（Playground の
  根号表示 / 正準表示としての平坦な連分数）を書いた。
- 予約記号 `( )` の説明から「連分数表示のための表記」を外した。現行の
  tokenizer のエラー文と同じく、括弧は `[ ]` のみである。
- ブレース撤去時の機械置換で残っていた「構造区切り文字（`[ ] [ ]`）」と
  「波括弧 `[ ]`」を修正した。
- 省略マーカーを実際の英語表示（`… N more`）に合わせた。
- `PI` の節を追加した（比較が `undecidable` になりうること、セッションに保存
  できないこと）。「超越関数は語彙にない」という記述と `PI` の存在が本文だけ
  読むと衝突して見えたため。
- Playground の 5 秒ガードと、MCP ホストとの上限差を上限の節に書いた。

英語版（`public/docs/index.html`）の再生成は保留。所有者の指示による。

### 2.6 `NUM` / `COND` / `RANGE` の raise site を宣言条件に置き換え（§2.3 の続き）

§2.3 で見送った「残り」のうち、§3 が名指ししていた3ワードを `AjisaiError::declared`
に置き換えた。

- `NUM`（`cast/cast_conversions.rs`）: `convert_value_to_number` の3箇所すべてを
  `nonText`（`errorWhen` の唯一の宣言）に。
- `RANGE`（`vector_ops/structure.rs`）: `parse_range_bound` / `parse_range_args` /
  `op_range` の7箇所すべてを `invalidRange`（同じく唯一の宣言）に。ステップ0と
  無限方向はどちらも「整形式でない」入力なので通常のエラーのまま
  （§2.3 のコード注釈が既に述べていた区別どおり）。
- `COND`（`control_cond.rs`）: 節の形の不備（block 数が0・奇数・混在スタイル・
  `|` の数や位置）を `invalidClauseShape` に、ガードが TRUE/FALSE を返さない
  5箇所を `nonTruthGuard` に。ただし「body must return a value」は
  `spec/words.json` の `COND.errorWhen`（`invalidClauseShape`, `nonTruthGuard`
  の2つのみ）に対応する条件が無いため `custom` のまま残した——宣言されて
  いない条件を名指しすることは `AjisaiError::declared` の契約自体が想定して
  いない。
- `declared_condition_tests.rs::a_named_condition_is_one_the_word_declares` に
  この3ワード×6ケースを追加し、`why` が `Unknown` に落ちないこと、
  診断が宣言語彙のまま返ることを固定した。
- `error.rs` の `AjisaiError::declared` の doc comment が指していた
  `declared_condition_is_declared_by_its_raising_word` というテストは実在しな
  かった（存在しないテスト名を指す誤記）。コメントを実在するテスト名に直し、
  そのテストが「呼び出し箇所を全数走査するのではなく手で列挙したコード例だけ
  を pin している」ことも明記した。

**まだ残っている**: `BOOL` と `cast_conversions.rs` の文字列版 `NIL`（`op_nil`）
は現行の `spec/words.json` にエントリが無く（`NIL` という名前の別エントリは
定数プッシュ側で別物）、宣言する条件そのものが無いので `custom` のままで
正しい。それ以外にも `AjisaiError::from` の raise site は
`execute_builtin.rs` / `tensor_ops.rs` / `io.rs` など複数ファイルにまだ残って
おり、§2.3 が最初に書いたとおり「仕組みは入ったので、残りは順次置き換えれば
よい」の状態が続く。

## 3. 未着手（レポートで妥当と確認したが今回やらないもの）

| 項目 | 理由 |
| --- | --- |
| 実行後の Input→Output 自動切替、成功時のみエディタを消す非対称 | レイアウト方針の判断が要る。挙動自体は `gui-layout-state.ts` と `execution-controller.ts` で確認済み |
| Reset のネイティブ `confirm()` のモーダル化 | UI 実装の追加。今回の主題（誤帰属・分類・文言）とは別 |
| 省略マーカーの locale 対応 | 文書側を実態に合わせたので不整合は解消済み。GUI の i18n 化は別課題 |
| MCP の npm 公開と Playground 内の接続導線 | リリース判断 |
| `NUM` / `COND` / `RANGE` 以外の残り raise site の条件名指し | §2.6 参照。`BOOL` / 文字列版 `NIL` は宣言語彙が無いため対象外 |
