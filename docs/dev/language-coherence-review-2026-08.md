# 言語整合化改修案の検証と、対案としての正典収束計画

> Status: **Non-canonical / 観察ノート＋方針記録.** 本書は言語意味論を一切定義しない。
> 正典は `spec/` 配下のソースと、そこから生成される `SPECIFICATION.html` のみ。
> 関連: `spec/language-semantics.md`（LANG.* 節）・`docs/dev/spec-impl-drift-tactic.md`
> （乖離裁定戦術）・`docs/dev/concept-reduction-2026-07.md`（十概念への削減）。

## 0. 本書の位置づけ

外部から提出された「Ajisai 言語整合化改修案」（以下 **提案書**）を、リポジトリの実状に
突き合わせて検証した記録である。提案書の観察の多くは正しい。しかし**結論は採らない**。
本書は、なぜ採らないかと、代わりに何をしたかを述べる。

検証はすべて実行によって行った。参照実装 CLI (`rust/target/debug/ajisai run`) に
プログラムを与えた観測結果を根拠とし、コードの読みだけを根拠にした主張は置いていない。

## 1. 結論

提案書の中心的主張は「現在の歪みは複数の言語モデルの併存に由来するので、Ajisai 2 として
意味論を再確定し、旧プログラムは一方向変換器で移行せよ」である。**この結論は誤りである。**

理由は単純で、**再確定すべき意味論はすでに確定している**。提案書が「Ajisai 2 で定めるべき」
とした項目のほぼ全てを、現行の正典 `spec/language-semantics.md` がすでに規定している:

| 提案書が「新たに定めよ」とした項目 | 現行正典の規定 |
| --- | --- |
| 六つの互いに素な値領域（String を含む） | LANG.VALUES.DISJOINT |
| 二値の真理値 | LANG.VALUES.TRUTH |
| reason が NIL の観測可能内容のすべて | LANG.VALUES.NIL |
| Vector は決して実行可能でない | LANG.SOURCE.CODE |
| 二層辞書のみ | LANG.DICTIONARY.RESOLUTION |
| 単一のリフティング規則 | LANG.COLLECTIONS.LIFT |
| 値・欠如・誤用の三分 | LANG.FAILURE.TRICHOTOMY |
| 表示は意味論を変更しない | LANG.STACK.ORDER |

つまり実際の乖離は「仕様どうしの矛盾」ではなく、**正典に対する実装と生成物の乖離**である。
提案書はこれを「仕様が壊れている」と誤診し、その誤診に基づいて仕様の作り直しを処方している。

作り直しの代償は大きい。conformance corpus と Word 契約レジストリは、収束したかどうかを
機械的に判定できる唯一の資産である。「Ajisai 2」はこの二つを同時に捨てる。さらに
`docs/dev/spec-impl-drift-tactic.md` が第一の不変条件として掲げる**第二権威の禁止**に
正面から反する。

**採用方針**: 正典を作り直さず、実装・生成物・corpus を正典へ収束させる。乖離が見つかった
各点で、正典が何と言っているかを引用し、そこへ寄せる。

## 2. 提案書の各節に対する判定

### 2.1 妥当（実行により確認済み）

| 節 | 主張 | 確認した観測 |
| --- | --- | --- |
| 2.1 | `EXEC` が値をソース文字列へ戻して再トークナイズする | `[ 1 2 ADD ] EXEC` → `1 2 [ 65 68 68 ]`。`ADD` が自身の符号位置として戻る |
| 2.2 | String が独立した領域でなく、空文字列が NIL になる | `'' ` → `NIL`（reason `emptySequence`） |
| 2.3 | 空 Vector が禁止されている | `[ ]` → ERROR "Empty vector is not allowed. Use NIL for empty values." |
| 2.4 | 述語が Boolean 以外を受理する | `[ 1 2 3 ] { 1 } FILTER` → `[ 1 2 3 ]`（全件通過） |
| 2.5 | 単要素 Vector が Scalar として比較される | `[ 3 ] 3 EQ` → `TRUE` |
| 2.6 | 値比較が reason を無視する経路がある | `Value::eq` が `absence` を見ていなかった |
| 2.8 | 二層辞書という規定に対し実装が複数辞書を持つ | `resolve_word.rs` に `@` 名前空間・三段フォールバック・`EXAMPLE` |
| 9 | conformance coverage 60/69 (87.0%) | 未被覆語 9 件まで一致 |

### 2.2 誤り

**§2.9「廃止済み構文が字句解析器に残っている」— 前提が事実に反する。**

提案書は `;` `;;` `.` `..` `~` `FLOW` を「廃止済みだがトークナイザに残存」と述べ、削除を
求める。しかしこれらは廃止されていない。`rust/src/surface_forms.rs` が管理する**現役の
surface form** であり、正典 LANG.SOURCE.DESUGAR が「修飾記号と登録された区切り形式は
評価前に正準概念へ降ろされる」と規定する対象そのものである。`.`/`..` は EAT/KEEP 修飾子、
`~` は FLOW の別名として `core_word_aliases` に登録されている。削除すれば、規定された
言語が壊れる。

なお提案書が挙げた参照先 `rust/src/interpreter/tokenizer.rs` と
`rust/src/interpreter/surface_forms.rs` はどちらも存在しない（実際は `rust/src/` 直下）。

**§2.10「VENT が後続ソースに依存している」— 不整合ではなく、規定どおりの挙動。**

LANG.FAILURE.RECOVERY は「非 NIL の top は通過し、**続くソース単位は読み飛ばされる**。
NIL の top は捨てられ、続くソース単位がフォールバックとして評価される」と明示的に規定して
いる。提案書の `value { fallback } VENT` 形は設計変更の提案としては筋が通るが、
「不整合の修正」ではない。両者を混ぜると、正典の変更が修正の名で通ってしまう。

### 2.3 部分的に妥当

**§2.6（NIL メタデータ）は一部が古い。** 提案書は origin / recoverability / diagnosis が
reason と競合すると述べるが、origin はすでに `rust/src/types/value_absence.rs:19` の
`absence_origin_for_reason` 一箇所で reason から導出されており、競合しない（同関数の
doc comment が、以前は競合していた経緯を記録している）。実際に生きていた欠陥は別で、
より重い（§3 参照）。

**§2.7（DEF/DEL と純粋性）は正典側の実在する矛盾を突いている。** LANG.EFFECTS.OUTPUT は
「出力が唯一の作用」と述べる一方、LANG.DICTIONARY.MUTATION は DEF/DEL による辞書変更を
規定し、LANG.MACHINE.ORDER は「トークン評価・出力・**辞書変更**は観測順序を保つ」と
辞書変更を作用として数えている。矛盾は本物である。

ただし提案書の処方（DEF/DEL を実行時 Word から削除し、ソースレベル宣言へ移す）は代償が
大きい。REPL の対話モデルと、LANG.DICTIONARY.MUTATION が規定する content identity の
両方を作り直すことになる。**より安い修正は正典側**で、LANG.EFFECTS.OUTPUT の一文を、
LANG.MACHINE.ORDER がすでに前提にしている二作用（出力・辞書変更）に合わせることである。
これは正典の変更なので、本 PR では実施せず提起にとどめる。

## 3. 提案書が見落としていた不整合

検証の過程で、提案書が挙げていない乖離が見つかった。いくつかは提案書が挙げたものより重い。

### 3.1 契約レジストリが、自ら引用する正典節と矛盾していた

`spec/words.json` の `EXEC` は `documentation.summary` を "Execute a vector as Ajisai code."、
`syntax` を `[ 1 2 + ] EXEC`、`errorWhen` を `nonCodeVector` としていた。一方その
`clauses` は `LANG.SOURCE.CODE` を挙げており、当の節は「Vector はデータであり決して
実行可能でない」と規定する。**契約が、自分が準拠を宣言した節の反対を述べていた。**

これは提案書 §9 の指摘（「緑のチェックは生成物どうしが同じ情報を共有していることを示すだけ」）
の具体例である。実際 CI は全て緑だった。生成物は `words.json` から導出されるので、
`words.json` が間違っていれば全生成物が揃って間違い、整合チェックは通る。

### 3.2 conformance corpus が正典の反対を主張していた

`tests/conformance/index.html` に、こういうケースが存在した:

```
id:     core-eq-structural-nil-uniform
title:  Structural equality treats all NIL elements uniformly, ignoring reasons
source: 0 0 DIV 1 COLLECT NIL 1 COLLECT EQ
expect: TRUE
```

LANG.VALUES.NIL は「reason が NIL の観測可能内容のすべてであり、同じ reason を持つ二つの
NIL は同じ値である」と規定する。`divisionByZero` と `literal` は同じ reason ではない。
このケースは正典の否定を、正典適合性の判定器の中に書き込んでいた。

十概念の第 10 項は corpus を「実装が Ajisai かどうかを決める」ものと位置づける。その corpus が
正典と食い違うのは、単なるテストの誤りより重い。**「実装を正典へ寄せる」作業は corpus の
検算を含まねばならない**、というのが本件の教訓である。

### 3.3 真理値の主たる強制変換部位は高階述語ではなく AND/OR/NOT だった

提案書は truthy 変換を高階述語と COND に見ていたが、実際の主要因は論理語そのものだった。
`AND`/`OR`/`NOT` は演算子の形によって Boolean 経路と数値 broadcast 経路を動的に選び分けて
おり、`0`/`1` が真理値として振る舞っていた。

さらに悪いことに、数値経路の結果は Scalar であるのに `TRUE` と表示される:

```
1 1 AND            => 表示は TRUE
1 1 AND TRUE EQ    => FALSE      （つまり TRUE ではない）
1 1 AND 1 ADD      => 2/1        （つまり Scalar 1 である）
```

LANG.STACK.ORDER は「表示変換はスタック値を並べ替え・強制変換・削除・**捏造**できない」と
規定する。ここでは表示が Boolean を捏造していた。提案書の原則「表示情報は意味論を変更しない」
は正しいが、違反箇所の特定を外していた。

契約側は最初から正しかった。`spec/semantic-families.json` の `booleanLogic` は
`truth: twoValued` であり、`lifting` キーを持たない。`AND`/`OR`/`NOT` の `errorWhen` は
`nonTruthValue` である。つまり executor が契約を精緻化していなかった（LANG.MACHINE.WORD 違反）。

### 3.4 「lossless」を謳う永続化コーデックが NIL reason を落としていた

`rust/src/types/value_persist.rs` は doc comment で「永続化は lossless でなければならない」
「`decode(encode(v)) == v` を保証する」と述べつつ、同じ comment で absence は識別に含まれない
（"provenance, not identity"）として round-trip の対象外としていた。`Value::eq` が
`absence` を無視していたので、この主張は自己無矛盾に見えていた。

LANG.VALUES.NIL の下では成立しない。reason は識別そのものである。同じ欠落が
`rust/src/types/arena.rs` にもあり、こちらは WASM の `restore_stack_snapshot` 経路に乗る。
**セッションを保存して再開すると NIL の reason が消えていた。**

### 3.5 順序比較のリフティングが正典と逆になっていた

LANG.COLLECTIONS.LIFT は「算術**または比較**の Word は Vector を与えられたとき要素ごとに
適用される」と規定する。実際は逆だった:

```
[ 3 ] 4 LT      => TRUE        （単要素を射影。正典は射影を禁じる）
[ 3 4 ] 4 LT    => ERROR       （正典は [ TRUE FALSE ] へリフトせよと言う）
[ 3 4 ] 4 ADD   => [ 7/1 8/1 ] （算術は正しくリフトする）
```

比較族は、正典が禁じる射影を行い、正典が要求するリフティングを拒否していた。同じ倒錯が
`ABS` にもある（`[ -1 2 ] ABS` は ERROR）。

なお提案書 §2.5 は順序比較を Scalar 限定にせよと述べるが、これは LANG.COLLECTIONS.LIFT から
**遠ざかる**方向である。§2.10 と同じく、正典の変更を修正として提示している。

## 4. 本 PR で実施した改修

正典が明示的に規定しており、かつ String 領域の導入を前提としない範囲を実施した。
すべて「正典が何と言っているか」を根拠とし、コードコメントに該当節を引用している。

### 4.1 真理値を単一モデルに戻した（LANG.VALUES.TRUTH / LANG.VALUES.DISJOINT）

- `rust/src/interpreter/logic.rs`: `AND`/`OR`/`NOT` から数値 broadcast 経路を削除。
  Boolean と NIL passthrough のみを受け、それ以外は `nonTruthValue` ERROR。
  二経路の動的選択（`forces_boolean_path`）とスカラー強制変換を削除。
- `rust/src/interpreter/higher_order/common.rs`: `extract_predicate_boolean` を
  Boolean のみ受理に変更。非ゼロ Scalar・単要素 Vector・NIL の受理を削除。
  未使用になった `is_truthy_boolean` を削除。

観測:

```
1 1 AND                   => ERROR (expected truth value)
0 NOT                     => ERROR (expected truth value)
TRUE FALSE AND            => FALSE
[ 1 2 3 ] { 1 } FILTER    => ERROR (expected boolean result from predicate block)
```

### 4.2 等価性を単一の構造的関係に戻した（LANG.VALUES.DISJOINT / LANG.VALUES.NIL）

- `rust/src/interpreter/comparison.rs`: `pairwise_eq` から単要素射影の 4 アームを削除。
  NIL 対を reason で判定するアームを先頭に追加。
- `rust/src/types/mod.rs`: `Value::eq` に `nil_reason()` の比較を追加。EQ Word が Vector に
  到達した後の要素判定はこの関係が担うため、両者が食い違うと言語から観測できてしまう。

```
[ 3 ] 3 EQ                                  => FALSE
0 0 DIV 1 COLLECT NIL 1 COLLECT EQ          => FALSE   （reason が異なる）
0 0 DIV 1 COLLECT 0 0 DIV 1 COLLECT EQ      => TRUE    （reason が一致）
```

`Value::eq` の修正は、§3.4 の二つの reason 欠落を露呈させた。両方を修正した:

- `rust/src/error.rs`: `NilReason::ALL` と `from_protocol_str` を追加。綴りの権威は
  `as_protocol_str` のままとし、逆写像は探索で導出する（表を二つにしない）。
- `rust/src/types/value_persist.rs`: wire 形式の `Nil` に reason を持たせた
  （`#[serde(default)]` なので旧ペイロードは読める）。
- `rust/src/types/arena.rs`: `NodeKind::Nil` に reason を持たせた。

### 4.3 EXEC を CodeBlock 専用にした（LANG.SOURCE.CODE）

- `rust/src/interpreter/control.rs`: CodeBlock のみ受理。他は ERROR。CodeBlock は
  すでにトークンを保持しているので、実行に round-trip は不要になった。
- `rust/src/interpreter/vector_exec.rs`: **削除**（229 行）。値→ソース文字列→
  再トークナイズの経路そのものが消えた。
- `spec/words.json`: `EXEC` の契約を実際の正典節に合わせた。`errorWhen` を
  `nonCodeVector` から既存語彙の `nonCodeBlock` へ（`MAP`/`FILTER`/`FOLD`/`ANY`/`ALL` と同じ）。
  documentation を全面的に更新し、全生成物を再生成した。

```
{ 1 2 ADD } EXEC              => 3/1
[ 1 2 ADD ] EXEC              => ERROR (expected code block)
5 EXEC                        => ERROR   （従来は暗黙の no-op）
{ { 2 3 MUL } EXEC 1 ADD } EXEC => 7/1
```

### 4.4 corpus の誤りを正し、被覆を 100% にした

- `core-eq-structural-nil-uniform`（§3.2）を正典に沿う二ケースへ置換。
- 上記三つの契約に対する実行ケースを追加（EXEC の型拒否、述語の型拒否、論理語の型拒否、
  単要素 Vector の非同一性）。
- 提案書 §9 が挙げた未被覆 9 語すべてにケースを追加:
  `ABS` `CONTAINS` `INDEX-OF` `MAX` `MIN` `NEG` `SIGN` `SQRT` `UNIQUE`。

```
conformance suite: 196 case sources   (改修前 175)
covered: 69 / 69 (100.0%)             (改修前 60 / 69, 87.0%)
```

### 4.5 実施しなかったもの

**空 Vector・空 String（提案書 §2.3）と String 領域（§2.2）は実施していない。** 両者は
同じ一つの作業である。String が符号位置 Vector として符号化されている限り、空 String は
空 Vector と同一物であり、`Interpretation::Text` だけが両者を隔てている。表示用フィールドに
領域の判定をさせたまま空値を解禁すると、意味論が表示に依存する度合いが増す。正しい順序は
**String を独立 variant にするのが先**である。`Value::eq` のコメントにこの依存関係を記録した。

同じ理由で `'A' [ 65 ] EQ` は現在も `TRUE` を返す。LANG.VALUES.DISJOINT 違反だが、
String 領域の導入なしには正しく直せない。`hint` を等価性から外す小手先の修正は、
String と符号位置 Vector をさらに多くの場所で同一視するので、方向が逆である。

順序比較のリフティング（§3.5）も未実施。要素ごとに Boolean を返す broadcast ヘルパが
必要で、既存の `apply_binary_broadcast_with_metrics` は Fraction を返すため流用できない。

## 5. 残作業と順序

提案書 §9 の実施順序は採らない。全 Word の再実装を第 9 段、100% conformance を第 10 段に
置いているが、これは**唯一のオラクルを最後に作る**順序である。被覆は先に取る（本 PR で
実施済み）。

正典への収束として残るものを、依存順に置く:

1. **String を独立した値領域にする。** `Interpretation::Text` の除去、空 String の解禁、
   `CHARS`/`JOIN` の確定。これが解けないと 2 も 3 も解けない。
2. **空 Vector の解禁。** 1 に依存。各 Word に散在する空入力の特例が消える。
3. **`Value::eq` から `hint` を外す。** 1 に依存。表示フィールドが意味論から離れる。
4. **比較族のリフティングを LANG.COLLECTIONS.LIFT に合わせる。** 単要素射影を削除し、
   要素ごと適用を実装する。`ABS` など単項数学語も同じエンジンへ。
5. **辞書を二層に戻す。** `resolve_word.rs` の `@` 名前空間・active/owner 辞書・
   三段フォールバックの削除。LANG.DICTIONARY.RESOLUTION は二層のみを規定する。
6. **LANG.EFFECTS.OUTPUT の作用数を LANG.MACHINE.ORDER に合わせる**（正典の変更。§2.3 参照）。
7. **Host Protocol の一本化。** V2 を正式採用するか V3 を定義し、V1 を adapter へ隔離。

各段の完了条件は「実行ケースが corpus にあり、通ること」とする。契約・実装・corpus の
三点が揃って初めて、その節は収束したと言える。

## 6. 提案書から採るべきだった原則

提案書の最終節が挙げる不可侵原則は、ほぼそのまま正典の言い換えである。以下は正典に
対応節を持つ:

| 原則 | 対応節 |
| --- | --- |
| 一つの値は一つの型だけを持つ | LANG.VALUES.DISJOINT |
| データは暗黙にはコードにならない | LANG.SOURCE.CODE |
| NIL は真理値ではない | LANG.VALUES.TRUTH |
| Scalar は暗黙には Boolean にならない | LANG.VALUES.DISJOINT |
| 単要素 Vector は Scalar ではない | LANG.VALUES.DISJOINT |
| 表示情報は意味論を変更しない | LANG.STACK.ORDER |
| すべての Word は同じ契約実行機構を通る | LANG.MACHINE.WORD |

対応節を持たないのは「空は失敗ではない」（LANG.VALUES.VECTOR が空を排除していないので
含意ではあるが明示規定はない）と「定義はプログラム実行中に変化しない」（LANG.DICTIONARY.MUTATION
が明示的に否定している）だけである。

これは提案書の主張が新しくないという意味ではなく、**正典を読めば処方箋はすでにそこにある**
という意味である。必要なのは新しい言語核ではなく、既存の言語核に対する適合作業である。
