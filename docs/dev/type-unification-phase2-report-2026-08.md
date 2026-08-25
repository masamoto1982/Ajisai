# CodeBlock/Vector 統合（改修Ⅰ）：Phase 2 実装報告（2026-08）

Status: 非正典・`[観察ノート]`。本書は実装の記録であり、意味論の定義ではない。
正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html`、および
`tests/conformance/index.html`（実行可能な適合性コーパス）のみ。

対象：`docs/dev/type-unification-work-order-2026-08.md`（作業指示書）の Phase 2
（本実装）。前提は Phase 0 実測報告（`type-unification-phase0-report-2026-08.md`、
PR #1571）と Phase 1 スパイク報告（`type-unification-phase1-report-2026-08.md`、
PR #1572）。

## 0. 承認と実装方針

ユーザーからの承認：

> 判断事項1、2、3について、正典の書き換えを承認します。言語としての整合性を最優先で進めて下さい。

これは Phase 0 報告 §7 の未回答質問（判断事項1）、Phase 1 報告が発見した `COND`
の構造的な壁への対処方針（判断事項2）、および §5.1 の表示非単射性の解消（判断事項3）
について、正典の書き換えを含む Phase 2 の実装を進める権限移譲である。「言語としての
整合性を最優先」を、本書が記録する複数の設計判断の指針とした。

## 1. 実装した内容

### 1.1 表現の統合

`ValueData::CodeBlock(Vec<Token>)` を `ValueData::Symbol(Arc<str>)` に置き換えた。
Symbol は「まだ実行されていない裸の Word 参照」で、REFLECT の正準ワイヤ形式が
隠していた `'symbol'` タグの概念を、値そのものに昇格したものである。`{ }` と
`[ ]` は `vector_literal.rs` の単一の収集関数を通して同一の値を構築する
（詳細は同ファイルのモジュールdocコメント）。

### 1.2 実行系の橋渡し

`value_as_code.rs`（新規）が `&[Value]` を `Vec<Token>` に戻し、既存のトークン
列ベースの実行ループ（`execute_nested_block`）がそのまま動く。`EXEC`・`PROBE`・
`DEF`・高階語族はすべてこの橋渡しを経由するよう書き換えた。`Tensor` 最適化
（全数値矩形 Vector の暗黙昇格）を橋渡しの対象に含めるため、`as_vector()` では
なく `as_vector_view()`（`Tensor` を認識する `Cow<[Value]>` 版）を使う必要が
あった——Phase 1 報告 §5.1 が発見していた通りの罠である。

### 1.3 `COND` の再設計（判断事項2の実装）

Phase 1 報告 §3.4・§6 は、`COND` の節ブロック発見機構が統合と本質的に相容れない
（実行時の領域判定に代わる手段がなく、部分修正としての字句情報保持もコンパイル
時点でしか有効でない）ことを実測で示し、「`COND` の呼び出し規約自体を作り直す
追加の改修」を選択肢として明示していた。

Phase 2 ではまずこの部分修正――`execution_loop.rs` が字句を先読みして「`COND`
の直前に連続する N 個の括弧グループ」を実行前に発見し側チャネルに退避する方式
――を実装したが、`cargo test` で以下が判明した：

- トップレベルの生プログラム文（`[ 5 ] { ... } { ... } COND`）で、対象値
  `[ 5 ]` 自身が括弧リテラルであるとき、先読みが対象値と節ブロックを区別
  できず、両方まとめて節ブロックとして飲み込んでしまう（節数が偶数/奇数の
  判定を壊す）。
- `DEF` された Word 本体の中の `COND` では、本体全体が一度 Value として構築
  されてから `value_as_code.rs` で字句へ戻るため、元の綴りが `{ }` であって
  も常に `[ ]` として再構成される。ここでも先読みは節ブロックと対象値を
  区別できない。

これは Phase 1 報告が予告した壁そのものであり、いずれも「値がいったん構築
されると、それが元々どちらの綴りだったか分からなくなる」という統合の理念
そのものに起因する。字句先読みという手段では、根本的に解決できない。

**採用した設計**：`COND` の呼び出し規約を、`MAP`/`FILTER`/`FOLD` と同じ
「固定位置の1オペランド」規約に変更した。

```
旧: value { guard | body } { guard | body } ... COND   (可変長)
新: value { { guard | body } { guard | body } ... } COND   (固定2オペランド)
```

節は 1 つの `{ }`（または `[ ]`）にまとめた 1 個の Vector として渡す。`COND`
は常に「対象値」と「節リストの Vector」という 2 つの固定位置オペランドだけを
消費する——`MAP` が「対象データ」と「コードブロック」の 2 オペランドを取るのと
全く同じ規約であり、可変長の実行時発見はもう必要ない。この変更により：

- 動的経路（`op_cond`）は節リスト Vector を1回 pop し、その要素ごとに
  `value_as_code.rs` で字句へ戻すだけになった。字句スキャン
  （`scan_cond_clause_run`）は完全に削除した。
- コンパイル経路（`lower_cond_dispatch`）も、`COND` の直前にある唯一の
  オペランド（`PushCodeBlock` または `PushVectorLiteral`）だけを見ればよく
  なり、「連続する N 個の `PushCodeBlock` を逆順に走査する」ロジットが
  不要になった。
- 綴りの区別に依存する箇所がゼロになった：トップレベルでもDEFされた本体
  でも、`COND` の振る舞いは完全に同一になる。

この変更は `COND` の構文を破壊的に変える（既存の全 `COND` 使用箇所で、節を
もう1段 `{ }` で包む必要がある）。「言語としての整合性を最優先」という指示
のもと、字句上のヒューリスティックや隠れた綴り情報の保持（LANG.VALUES.
DENOTATION が明示的に禁じる「構築履歴を値の一部にする」ことに相当する）で
はなく、他の高階語とまったく同じ規約に揃えることを選んだ。

### 1.4 `DEF` のトークン忠実性（新発見・§3.4 で追加対応）

`COND` の再設計後も、契約推論エンジン（`word_contract.rs` / `word_contract_
widen.rs`）で新たな回帰が見つかった。`{ { PRINT } MAP } 'PRINTALL' DEF` の
ような、`DEF` された本体の中にネストした `{ }` を含む極めて一般的なコードで、
`PRINTALL` の純度推論が `Effectful` から `Pure` に後退していた。

原因は `COND` と同型：`DEF` は演算対象の Value を `value_as_code.rs` で字句へ
戻して保存するため、本体内部のネストした `{ PRINT }` も常に `[ PRINT ]` に
再構成される。`word_contract_widen.rs` の「`[` の中の Symbol は絶対に解決・
呼び出しされないデータ」という判定（`[ 'a' PRINT 'b' ]` が実際に PRINT を
実行しないことを正しく判定するために既存していたゲート）が、この再構成後の
`[ PRINT ]` にも誤って適用され、実際には `MAP` を通して実行される PRINT を
「データ」と誤判定してしまう。

`COND` と異なり、`MAP`/`FILTER`/`FOLD` などの呼び出し規約自体を変える必要は
ない——問題は輪郭ではなく忠実性である。そこで `DEF` に限定した軽量な最適化を
追加した：`execution_loop.rs` が、ブラケットリテラルの直後が
`<name-string> [KEEP] DEF` という固定パターンに一致するかを検査し（`COND`
の旧字句スキャンと異なり、これは曖昧性解消ではなく忠実性の保存であり、
`DEF` の2オペランドという規約自体は最初から固定位置である）、一致すれば
その場で書かれた生トークンを `pending_def_body_tokens` に退避する。`op_def`
はこれを優先し、なければ従来通り橋渡し経由でトークンを導出する。パターンが
一致しない場合（計算された Vector からの `DEF` など）は常に橋渡しにフォール
バックし、動作は変わらない。

### 1.5 `REFLECT` の削除

REFLECT は「CodeBlock トークン列と正準 Vector の間の唯一の可逆な境界」だった
が、その境界自体が統合により消滅した（あらゆる Vector が既に実行可能なので、
渡る境界がない）。ワード定義・関連する契約推論の特例（`GapCode::
OpaqueReflection` とその周辺）・生成ドキュメントパイプラインから完全に削除
した。

### 1.6 表示非単射性の解消（判断事項3の実装）

統合前は `{ TRUE FALSE }`（CodeBlock）と `[ TRUE FALSE ]`（Vector）が異なる
値でありながら同じ文字列で表示され、LANG.VALUES.DENOTATION が要求する
「値の同一性は表示から復元できる」を壊していた（Phase 0 報告 §5.1）。統合後は
この2つが文字通り同じ値になったため、`display.rs` はコード形状の Vector も
含めて常に `[ ]` で一様に表示する。これは恣意的な選択ではなく、値がひとつに
なった帰結である。

## 1.7 互換性方針（Phase 0 報告 §7 項目2）

保存セッションの永続化形式（`value_persist.rs`、`PersistData::Code`→
`PersistData::Symbol`）と `COND` の構文が変わるため、Phase 0 報告が提起した
「0.2.0-beta.1 の中で吸収するか、新しいベータ段階として扱うか」に対し、
ベータチャンネル内での破壊的変更として扱い、`0.2.0-beta.2` へ上げた
（`package.json`・`rust/Cargo.toml`・`src-tauri/Cargo.toml`・
`src-tauri/tauri.conf.json` の4マニフェストと対応するロックファイル。
`npm run check:version-sync` で確認済み）。1.0 未満のベータでは後方互換
保証がそもそもないため、既存の意味論を壊す変更をベータ内で吸収しつつ
段階番号だけ進める――利用者に「beta.1 時点のセッション/プログラムとの
互換は保証されない」と伝える最小限のシグナルとした。

## 2. 検証

`cargo test`（`rust/src/**` の `#[cfg(test)]` と `rust/tests/*.rs` の統合
テストを合わせて全て）、`cargo fmt --check`、`cargo clippy --all-targets --
-D warnings` はすべて緑。適合性コーパス（`tests/conformance/index.html`）は
270 件全て通過——REFLECT 専用の 11 件を、統合後の等価な能力（`{ }`/`[ ]` の
綴りをまたいだ `EQ`、`[ ]`-綴りの Vector を `EXEC` できること、計算された
Vector からの `DEF`）を示す 4 件の新規ケースに置き換え、`core-vector-name-
is-data` など Symbol/Text の表示差に伴う既存 3 件、COND の新規約に伴う
8 件、§5.1 解消に伴う 5 件を更新した（内訳は本コミット群の適合性コーパス
差分を参照）。

`spec/words.json`（`COND` の `stack.inputs` を `"variable"` から固定 `2`
へ、`syntax`/`stackEffect` を新規約に、`MAP`/`FILTER`/`FOLD`/`ANY`/`ALL`/
`EXEC`/`PROBE` の `errorWhen` タグ `nonCodeBlock` を `notExecutable` に
リネーム）と `spec/language-semantics.md`（LANG.VALUES.DISJOINT の6ドメイン
目を CodeBlock から Symbol へ、LANG.VALUES.VECTOR・LANG.SOURCE.CODE を
書き換え、LANG.SOURCE.REFLECTION を削除、語彙数を66→65・Semantic Kernel を
37→36 語に更新）を更新し、全ての生成ドキュメント
（`SPECIFICATION.html`・`SKILL.md`・`docs/word-reference.md`・
`docs/word-manifest.json`・`docs/semantics-table.json` 等）を再生成した。
`docs/formalization-coverage.json` から REFLECT 関連の2エントリ
（`algebra.reflection.involution`・`core.reflect`）を削除し、
`scripts/check-minimal-core.mjs` の期待値とハードコードされた REFLECT 参照
を更新した。`public/docs/index.html` と `public/docs/ja/index.html` の
「Code as data」節・`COND` の例・`[ ]` 内の名前に関する節を、新しい意味論
（REFLECT なし、`COND` の新規約、Symbol 表示）に合わせて書き直した。

`npm run specification:check`・`check:reading-surfaces`・`check:file-size`・
`word-schema:check`・`check:minimal-core`・`check:unreachable-contract`・
`check:version-sync`・`check:agent-cli-contract`・`check:formalization-
coverage`・`check:semantic-firewall`・`check:mcp-assets` は全て緑。
`check:mcp-evaluation` と `npm run check`（`tsc --noEmit`）は
`node_modules` が本セッションの環境に未インストールという Phase 2 と無関係
の事前からの環境上の制約により失敗する（変更を `git stash` した状態でも
同じエラーが再現することを確認済み）。

## 3. まとめ

Phase 0・Phase 1 が見積もった通りの範囲（表現の統合、実行系の橋渡し、
Tensor 最適化・`\|` トークン・`KernelValue` 第二表現への波及）は Phase 1
報告の実測とほぼ一致した。Phase 1 報告が「Phase 2 に進むなら避けて通れない」
と明示していた `COND` の再設計は、可変長オペランドを固定位置の単一オペランド
に変える形で実施し、字句上のヒューリスティックに一切頼らない設計に落ち着いた。
これに加えて Phase 1 報告が予見していなかった `DEF` の契約推論忠実性問題を
実装中に発見し、`DEF` に限定した軽量な字句最適化で対処した。§5.1 の表示
非単射性は、統合の直接の帰結として自然に解消された。
