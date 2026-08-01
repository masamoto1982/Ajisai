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

**追記（実施済み）。** 仕様所有者の承認を得て、この修正を正典側で行った。同じ矛盾は
`reduction-consistency-audit-2026-07.md` の D11 としてすでに記録されており、そこでの裁定
（「`LANG.CONTRACT.REGISTRY` の下では `words.json` が purity/effects の権威なので、
kernel prose が誤り」）に従っている。契約レジストリは最初から正しい側にあった——
`DEF` は `effectful`/`dictionaryWrite`、`DEL` は `effectful`/`dictionaryDelete`、
`LOOKUP` は `observational`/`dictionaryRead`、そして高階語 7 語が `conditional` である。
外れ値は prose の一文だけだった。詳細は §4.5。

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

## 4. 実施した改修

§4.1〜§4.4 は最初の改修（PR #1413）、§4.5 以降はその後の追補で順に実施した。

§4.1〜§4.4 は、正典が明示的に規定しており、かつ String 領域の導入を前提としない範囲である。
すべて「正典が何と言っているか」を根拠とし、コードコメントに該当節を引用している。
§4.5 だけは例外で、正典そのものを変更した——本書 §2.3 が「より安い修正は正典側」と判定した
一点について、仕様所有者の承認を得ている。§4.6 は §5 の第 1 項（と、その帰結である第 3 項）
であり、正典は変更していない。

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

### 4.5 LANG.EFFECTS.OUTPUT を二作用に合わせた（正典の変更）

唯一、**正典そのものを変更した**箇所である。仕様所有者の承認を得て実施した。

旧文は「出力が唯一の作用。……**他のすべての Word は純粋**であり、同じスタックと辞書に
対して同じ結果を生み、他に何も変えない」と述べていた。この一文は三重に誤っていた:

- `DEF`/`DEL` は辞書を変更する（LANG.DICTIONARY.MUTATION が規定し、`words.json` が
  `effectful` と宣言している）。
- `LOOKUP` は辞書を読む（`observational`）。変更はしないが、純粋の説明が触れていない。
- 高階語 7 語は `conditional` である。ブロックが `PRINT` や `DEF` を含みうる以上、
  純粋性は供給されたブロックに依存する。LANG.COLLECTIONS.HIGHER 自身が
  「供給されたブロックが出力を発行し、または辞書状態を変更しうる場合、要素訪問順序は
  観測可能である」と、すでにこれを前提にしていた。

新文は作用を二種に分けて述べる。**機械の外に出る作用は出力だけ**（ホストが観測するのは
出力ストリームのみ）で、**機械の内側にもう一つ**辞書変更がある。両者は LANG.MACHINE.ORDER が
トークン評価に対して順序づける。純粋性の記述は `LOOKUP` の読み取りと、ブロックを評価する
Word の条件付き純粋性を含むように直した。

clause ID は `LANG.EFFECTS.OUTPUT` のまま据え置いた。この節は依然として出力の節であり、
誤っていたのは列挙の一文だけである。ID を変えれば `words.json`・`semantic-families.json`・
生成物すべてに波及するが、得るものがない。

実装側のコメント 4 箇所（`host.rs`・`interpreter_core.rs`・`coreword_registry.rs`・
`contract_tests.rs`）も直した。いずれも**ホスト作用**の話としては正しかったが、
"the only effect" という正典の過剰主張をそのまま繰り返していた。`hostedEffect` を持つのは
`PRINT` だけなので、「ホストに届く唯一の作用」と限定すれば正確になる。

あわせて、作用チャネルに対する実行ケースを 4 件追加した。**この改修前、conformance suite は
ホスト作用を一度も発火させていなかった**——suite 自身が「唯一の作用観測対象」と呼ぶ
チャネルが、未固定のまま残っていた。

### 4.6 String を独立した値領域にした（LANG.VALUES.DISJOINT）

§5 の第 1 項。`ValueData::Text(Arc<str>)` を導入し、`Interpretation::Text` を削除した。

**旧表現では、値の領域が値の性質ではなかった。** String は符号位置 Scalar の Vector に
`Interpretation::Text` という表示用ロールを付けたものであり、領域を決めていたのは
表示フィールドだった。帰結は二つ、いずれも言語から観測できた:

- `'A' [ 65 ] EQ` が `TRUE` を返した。符号化が同一で、hint だけが違ったためである。
- `is_string_value` が「文字列らしさ」を**推測**していた。全要素が印字可能な符号位置かを
  走査して判定するので、`[ 65 ]` と `'A'` を区別できない。これは `Interpretation` 自身の
  doc comment が「実行時は描画時に意味を推測しない」と約束している当のことである。

内容を直接持たせると両方が消える。領域はタグであり、要素から推測するものが何もなくなる。

**空 String を解禁した。** `''` は `NilReason::EmptySequence` になっていた。これは「値が
ない」ことを意味する NIL に、文字が 0 個の値を押し込むもので、text 族の各 Word に空の
特例を強い、`TOKENIZE`/`SUBSTITUTE` の下で領域が閉じなくなっていた。
`'ab' 'ab' '' SUBSTITUTE` は今 `''` を返す（以前は NIL）。

**§5 の第 3 項（`Value::eq` から `hint` を外す）も同時に完了した。** 前 PR が `Value::eq` の
コメントに「String が独立領域になればこの関係から hint は外れる」と記録していた依存関係で、
実際に外れた。等価性は data と NIL reason だけを見る。

**collection 族は String を受け付けなくなった。** `LENGTH`/`GET`/`CONCAT`/`REVERSE` は
いずれも family `collection`・`errorWhen: ["nonVector"]` で、要約も「vector」と述べている。
String が Vector でなくなった以上、これらは `nonVector` を上げる。文字列連結は JOIN が担う
——`[ 'ab' 'cd' ] JOIN` であり、これは JOIN の契約（「a vector of strings を単一の string に
join する」）が最初から述べていたことである。計算した値を繋ぐには `COLLECT` で Vector に
してから JOIN する（`'a' 'b' 2 COLLECT JOIN`）。

この判断は**レジストリに厳密に従った**結果である。正典にも Reference にも、collection 族を
String へ拡張する規定はない。代償として corpus の 4 ケースが変わった（うち
`core-str-flattens-container-leaves` は `'65 66 67 68'` → `'AB CD'` と**改善**している——
Text 要素が符号位置へ崩れなくなった）。

**契約レジストリの変更は一件もない。** §4.5 と同じ構図で、`words.json` は最初から正しく、
実装がそこへ収束した。`nonVector`・`nonText`・`invalidName` はいずれも既存の宣言どおりに
発火するようになった。

副次的に見つかったもの: `extract_word_name_from_value` は値を fraction 列へ平坦化して
符号位置として解釈していたので、`[ 73 78 67 ]` を Word 名 `INC` として受理していた。
Word 名は String である。

作用・領域の実行ケースを 10 件追加した（corpus 200 → 214、Core 被覆は 69/69 のまま）。

### 4.7 空 Vector を解禁した（LANG.VALUES.VECTOR）

§5 の第 2 項。`[ ]` が書けるようになり、空の結果は NIL ではなく空 Vector になった。

**正典は空を除外していなかった。** LANG.VALUES.VECTOR は Vector を「ordered finite
collection of values」と定め、「order and length」をその観測可能構造のすべてとする。0 は
有限の長さである。除外は実装側の発明で、その帰結として **NIL に「空の集合」という第二の
職務**を負わせていた。これは LANG.VALUES.NIL——reason は絶対の観測可能内容であり、値の
代用ではない——と衝突する。

**レジストリは今回も正しい側にあった。** `SORT`・`UNIQUE`・`FILTER`・`TAKE`・`MAP`・
`SPLIT`・`REMOVE` はいずれも `projection: {"when":"never"}` を宣言している。しかし実装は
結果が空になる場合に `NIL(EmptySequence)` へ射影していた——つまり宣言は**偽**だった。
空 Vector が値になったことで、これらの宣言が初めて真になる。§4.5・§4.6 と同じ構図で、
これで三度目である。

旧テスト `an_empty_vector_is_inexpressible` は自ら「空 Vector が構築可能になればこの法則は
破れ、両宣言を見直す必要がある」と書いていた。構築可能になり、見直した結果、宣言の側が
正しかった。

観測:

```
[ ]                          => [ ]
[ ] LENGTH                   => [ ] 0/1
[ 1 2 3 ] { 5 GT } FILTER    => [ ]        （旧: NIL）
[ 1 2 3 ] [ 0 ] TAKE         => [ ]        （旧: NIL）
[ 1 2 3 ] [ 0 3 ] SPLIT      => [ ] [ 1/1 2/1 3/1 ]
'' CHARS JOIN                => ''         （旧: 両端がエラー）
[ ] JOIN                     => ''
```

§4.6 で唯一 String 領域が Vector 領域を追い越していた `'' CHARS` も、これで解消した。
`[ ] JOIN` が `''` を返すので、`CHARS`/`JOIN` は空を含めて互いに逆になる。

**`NilReason::EmptySequence` は到達不能になったが、variant は残した。** 退役させた
`LogicallyUnknown` と違い、この reason は逆デコードされる——`value_persist::decode_value` は
知らない reason 文字列を**ハードエラー**にするので、variant を削ると本改修以前に保存された
セッションスナップショットが復元不能になる。degrade ではなく失敗になるため、互換性を優先
した。enum に、もはや生成されないことと残す理由を記録してある。

### 4.8 比較族と単項数学語を LANG.COLLECTIONS.LIFT に合わせた

§5 の第 4 項。

**節の権威範囲を先に確定した。** `semantic-families.json` の `comparison` 族は
LANG.COLLECTIONS.LIFT を参照せず `lifting` キーも持たないので、レジストリだけを見ると
「比較は lift しない」と読める。しかし LANG.CONTRACT.REGISTRY はレジストリの権威範囲を
「arity, consumption, NIL policy, projection reason, error conditions, purity,
documentation」と**明示的に列挙**しており、lifting は含まれない。よって lifting の正典は
LANG.COLLECTIONS.LIFT の側である。

節は「arithmetic **or comparison** Word applies element-wise when given vectors. Two
vectors combine element-wise when their lengths are equal; a scalar combines with every
element of a vector. Any other pairing is ERROR」と述べる。比較族は**その逆**をしていた:

```
[ 3 ] 4 LT      => TRUE     旧。単要素射影——節が禁じる collapse
[ 3 4 ] 4 LT    => ERROR    旧。節が要求する要素ごと適用を拒否
[ -1 2 ] ABS    => ERROR    旧。ABS は LANG.COLLECTIONS.LIFT を宣言済み
```

改修後:

```
[ 3 4 ] 4 LT           => { TRUE FALSE }
3 [ 4 5 ] LT           => { TRUE TRUE }
[ 1 2 ] [ 3 1 ] LT     => { TRUE FALSE }
[ 3 ] 4 LT             => { TRUE }        射影せず shape を保つ
[ 1 2 3 ] [ 1 2 ] LT   => ERROR           長さ不一致 = shapeMismatch
[ 1 NIL ] 2 LT         => { TRUE NIL }    lane が scalar law の NIL を保つ
[ -1 2 ] ABS           => [ 1/1 2/1 ]
```

**`ABS`/`NEG`/`SIGN` はすでに `LANG.COLLECTIONS.LIFT` を clauses に宣言していた。**
宣言どおりに動いていなかっただけである。§4.5・§4.6・§4.7 と同じ構図で、これで四度目。
順序比較 4 語の `errorWhen: ["shapeMismatch"]` も、これまで到達不能だった（Vector を
一律に拒否していたため）が、長さ不一致として初めて意味を持つ。

`LT`/`LTE`/`GT`/`GTE` の clauses に LANG.COLLECTIONS.LIFT を追加した。**族単位では
付けていない**——`EQ`/`NEQ` は同じ `comparison` 族だが構造的関係であり（§4.2）、
lift すると `[ 1 2 ] [ 1 2 ] EQ` が `{ TRUE TRUE }` になって「二つの Vector が同じ値か」
を問う手段が消える。LANG.VALUES.DISJOINT が要求する構造的等価性と両立しない。
**節文の "comparison" は順序比較を指す**と読むべきで、そう明記するのが正典側の改善に
なる（未実施。仕様所有者の判断）。

### 4.9 本改修が露出させた所見（未修正）

**COND のガードが真理値を強制変換している。** `control_cond.rs` は Boolean のほか、
1 要素 Vector に包まれた Boolean と、「legacy numeric guard」（scalar 0 = false,
1 = true）を受理する。どちらも LANG.VALUES.TRUTH が排除する強制変換で、
LANG.VALUES.DISJOINT に二重に反する（単要素 Vector はその要素ではない／scalar は
Boolean ではない）。§4.1 が `AND`/`OR`/`NOT` と `extract_predicate_boolean` から
除いたものが、ここだけ残っていた。

リフティングはこれを**到達可能から常用へ**変えた。旧来 `[ 7 ] [ 5 ] >` は単要素射影で
裸の `TRUE` を返していたが、今は `[ TRUE ]` を返し、wrapper 受理経路に乗る。厳格化を
試みたところ、`[ n ]` でスカラーを包む既存の記法がテスト・例に広く使われているため
多数が壊れた。lifting とは別の規律なので独立項目として残し、コードにも FINDING として
記録した。

**算術の singleton broadcast も節に反する。** `[ 1 2 ] [ 3 ] ADD` が `[ 4/1 5/1 ]` を
返す。節は「長さが等しいとき要素ごと、スカラーは全要素と結合、それ以外は ERROR」と
述べるので、長さ 2 と 1 の対は ERROR であるべきである（監査 D12）。本項目は比較族と
単項語に限定したため未修正。

## 5. 残作業と順序

提案書 §9 の実施順序は採らない。全 Word の再実装を第 9 段、100% conformance を第 10 段に
置いているが、これは**唯一のオラクルを最後に作る**順序である。被覆は先に取る（実施済み）。

正典への収束として残るものを、依存順に置く:

1. ~~**String を独立した値領域にする。**~~ — 実施済み（§4.6）。
2. ~~**空 Vector の解禁。**~~ — 実施済み（§4.7）。
3. ~~**`Value::eq` から `hint` を外す。**~~ — 実施済み（§4.6。1 の直接の帰結）。
4. ~~**比較族のリフティングを LANG.COLLECTIONS.LIFT に合わせる。**~~ — 実施済み（§4.8）。
5. **辞書を二層に戻す。** `resolve_word.rs` の `@` 名前空間・active/owner 辞書・
   三段フォールバックの削除。LANG.DICTIONARY.RESOLUTION は二層のみを規定する。
6. ~~**LANG.EFFECTS.OUTPUT の作用数を LANG.MACHINE.ORDER に合わせる**~~ — 実施済み（§4.5）。
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
