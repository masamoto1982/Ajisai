# MCP 実使用で判明した欠陥の改修指示書（2026-09）

Status: 非正典・`[設計根拠]`。本書は Ajisai の意味論も互換性方針も定義しない。
正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。

前提文書：`docs/dev/spec-impl-alignment-methodology.md`（4フェーズ手順と
スイート裁定規則。本書の所見はすべてその Phase 3 の対象である）、
`tools/mcp-server/README.md`（MCP 面の約束）。

---

## 0. この文書の読み方（実装者向け・最初に必ず読む）

### 0.1 出所

PR #1614・#1615 で MCP サーバーを接続・整備した直後、**Claude (Opus 5) が
MCP クライアントとして実タスクを流した**セッションの所見である。机上のレビュー
ではなく、`compute` / `check` / `infer_contracts` / `word_contract` を実際に
呼んで出た結果だけを根拠にしている。

検証環境（全所見で同一）:

| 項目 | 値 |
| --- | --- |
| `mcp.serverVersion` | 0.3.0 |
| `mcp.engineVersion` | 0.2.0-alpha.1 |
| `mcp.registryDigest` | `a41f6dbb7e8716f0567be684f8b9d91a01a7e46fb38feffba2171d76c5584c0a` |
| backend | `nativeCli` と `wasmWorker` の**両方**で再現確認 |

### 0.2 記述規律

- **すべての再現手順は実行済み**である。実行していない推測には「未確認」と明記した。
- 実装バグ（仕様違反）・仕様内の設計不整合（所有者判断が要る）・文書/表面の問題を
  §1 の表で明確に分けた。**混ぜて直さないこと**。F-3 は判断待ちであり、
  勝手にどちらかへ寄せてはならない。

### 0.3 禁止事項

- `SPECIFICATION.html` を手で編集しない（`spec/` から生成する）。
- テストの skip・無効化・quarantine で緑にしない。
- **F-1 を `Fraction::new` の panic 抑止だけで閉じない。** panic を握り潰すと
  「NIL レーンが 0/1 として読める」別の静かな誤りに化ける。本題は表現の一本化である。
- 実測していない効果をコメント・コミットメッセージに書かない。

### 0.4 停止条件（詰まったら黙って続行せず報告する）

- F-1 の表現一本化で、`valid_mask` と分母 0 センチネルのどちらを残すかが
  性能・永続化フォーマットに影響すると判明した場合（`value_persist.rs` が
  絡む）。所有者判断へ差し戻す。
- F-2 の修正が `LANG.COLLECTIONS.LIFT` の「Vectorization cannot turn an ERROR
  lane into NIL」に抵触する形になった場合。

---

## 1. 所見一覧

| ID | 重大度 | 種別 | 一行要約 | 裁定 |
| --- | --- | --- | --- | --- |
| F-1 | **重大** | 実装バグ | NIL レーンを含む Vector への算術がプロセスを落とす（exit 101） | impl → spec |
| F-2 | 高 | 実装バグ | 除算の NIL レーンが Vector 全体を潰す（LIFT 違反） | impl → spec |
| F-3 | 中 | 仕様内不整合 | ゼロ除数が DIV では NIL、MOD では ERROR | **所有者判断待ち** |
| F-4 | 中 | 実装（診断品質） | registry が宣言済みの条件が診断に出ず `custom`/`unknown` になる | 改善 |
| F-5 | 中 | 文書 vs 実装 | README が約束する `diagnosis.resourceLimit` が載らない | impl or doc |
| F-6 | 低 | 実装（文言） | COND の NIL 主体が "expected a value, got NIL" で落ちる | 文言 + 判断 |
| F-7 | 低 | 文書 | `[ 42 ]` を idiomatic と教える §2 と PUT の要素意味論が衝突し、誤りが黙って通る | doc |
| F-8 | 低 | 文書 | `infer_contracts` が `nil-propagating` と `nil-free` を同一応答内に併記 | doc |
| F-9 | 低 | 文書 | cost クラスが桁を隠す（同じ `const` で 5 と 2048） | doc |

---

## 2. 所見の詳細

### F-1（重大）NIL レーンを含む Vector への算術がバックエンドを落とす

**再現**（3 形すべて確認済み。`compute` 経由、両バックエンド）:

```
[ 1 NIL 3 ] [ 2 ] *          => hostError backendFailure
[ 1 NIL 3 ] [ 1 1 1 ] +      => hostError backendFailure
[ 4 -1 ] SQRT [ 2 ] *        => hostError backendFailure
[ 1 2 3 ] [ [ 0 ] / ] MAP [ 1 ] +  => hostError backendFailure
```

native CLI を直接叩くと **panic / exit 101**:

```
thread 'main' panicked at src/types/fraction.rs:189:13: Division by zero
  2: Fraction::new                      (rust/src/types/fraction.rs:189)
  3: DenseTensor::get_small_fraction    (rust/src/types/tensor_storage.rs:129)
  4: DenseTensor::fraction_or_nil       (rust/src/types/tensor_storage.rs:136)
  5: tensor_to_nested_values::build     (rust/src/types/value_tensor.rs:179)
 18: Value::as_vector_view              (rust/src/types/value_semantics.rs:380)
 21: apply_word_hint_override           (rust/src/interpreter/execution_loop.rs:108)
```

**根本原因（確認済み）— NIL が二つの表現で符号化されている。**

1. `Fraction::nil()` は `FractionRepr::Small(0, 0)`、すなわち**分母 0 のセンチネル**
   （`rust/src/types/fraction.rs:168-176`）。本番の書き込み経路
   `value_children.rs:179`（`ValueData::Nil => buf.push(Fraction::nil())`）は
   これを積む。
2. `DenseTensor` はもう一つ別に `valid_mask` を持つ。しかし
   `tensor_storage.rs:48-50` と `:74-76` はマスクを **`u64::MAX`（全ビット有効）で
   初期化**し、`clear_valid` の呼び出しは**テストにしか存在しない**
   （`value_tensor.rs:290`、`tensor_storage.rs:334-335`、`value_persist_tests.rs:105`）。
3. 読み出し側 `get_small_fraction`（`tensor_storage.rs:125-133`）は
   **マスクだけを見て** `Fraction::new(numerator, denominator)` を呼ぶ。
   NIL レーンは「マスク上は有効・分母 0」なので `Fraction::new` の
   `panic!("Division by zero")` に直行する。

つまり本番では **`valid_mask` は不在マーカーとして一度も使われておらず**、
不在はすべて分母 0 で表されているのに、読み出しはマスクだけを信用している。
これは本 repo が Phase 2 で繰り返し扱ってきた「同じ事実が二か所にあり、
クロスチェックされていない」型そのものである。

**仕様上の裁定（impl → spec）。** `LANG.COLLECTIONS.LIFT`
（`spec/language-semantics.md`）は明言している:

> Each lane preserves the exactness, truth, NIL, and ERROR distinctions of the scalar law.

スカラー法則は `NIL 1 +` → `NIL`（実行確認済み）。したがって
`[ 1 NIL 3 ] [ 2 ] *` の正解は `[ 2/1 NIL 6/1 ]` である。**この期待値は推測ではない**
——同じ計算を MAP 経由で書いた `[ 1 NIL 3 ] [ 2 MUL ] MAP` が現在すでに
`[ 2/1 NIL 6/1 ]` を返している。

**conformance corpus はクラッシュの両隣を pin しているが、その間を pin していない。**

| 既存ケース | 期待 | 状態 |
| --- | --- | --- |
| "SQRT applies element-wise, and a negative lane bubbles alone" — `[ 4 -1 ] SQRT` | `[ 2/1 NIL ]` | 通る |
| "MAP over a vector containing NIL passes the NIL element through the block" — `[ 1 NIL 3 ] [ 2 MUL ] MAP` | `[ 2/1 NIL 6/1 ]` | 通る |
| **（欠落）NIL レーンを含む Vector に算術 Word を直接適用** | `[ 2/1 NIL 6/1 ]` | **プロセス abort** |

スイートは沈黙し、挙動は本文に違反する。裁定規則どおり **実装を仕様へ寄せ、
矯正後の挙動を pin する conformance ケースを同一 PR で追加する**。

**修正方針。**

- 暫定（読み出しの nil 認識）: `get_small_fraction` が分母 0 を「不在」として
  `None` を返す。`fraction_or_nil` は既に `Fraction::nil()` を返す形なので、
  `iter()` / `to_fractions()` も同時に直る。
- 本題（表現の一本化）: `valid_mask` と分母 0 センチネルのどちらを正とするか
  決め、他方を削除する。**証拠は「センチネルが生きている側」を示している**
  （本番の書き込みはすべてセンチネル、`clear_valid` は本番呼び出し 0 件、
  マスクは常に全 1）。ただし `valid_mask` は `PartialEq`（`tensor_storage.rs:21`）と
  永続化に絡むため、削除の可否は §0.4 の停止条件に該当しうる。
- `Fraction::new` の panic 自体は**残す**。不正な分母 0 の構築は本来バグであり、
  ここを `Option` 化して黙らせると F-1 が静かな誤値に化ける。

**受け入れ条件。**

1. 上記 4 本の再現がすべて `status: ok` で正しいレーン値を返す。
2. conformance に「NIL レーンを含む Vector への算術は各レーンを保つ」ケースを追加し、
   `[ 1 NIL 3 ] [ 2 ] *` → `[ 2/1 NIL 6/1 ]` を pin する。
3. `npm run test:mcp-backends`（native/WASM パリティ）が両backendで緑。
4. `cargo test --all-targets` 緑。

### F-2（高）除算の NIL レーンが Vector 全体を潰す

**再現**:

```
[ 6 6 6 ] [ 1 2 0 ] /   => NIL        （期待: [ 6/1 3/1 NIL ]）
[ 6 ] [ 1 2 0 ] /       => NIL        （期待: [ 6/1 3/1 NIL ]）
```

成功したはずの 6/1 と 3/1 が消える。同じ射影クラス（`partiality: projecting`）の
SQRT は正しくレーンを保つ:

```
[ 4 -1 ] SQRT           => [ 2/1 NIL ]
[ 4 -9 ] SQRT [ 1 1 ] < => [ FALSE NIL ]      比較も正しく持ち上がる
[ 1 2 3 ] [ [ 0 ] / ] MAP => [ NIL NIL NIL ]  MAP 経由の DIV は正しい
```

**同じ DIV が、経路（LIFT か MAP か）で異なる意味を持ってしまっている。**
`word_contract DIV` は `clauses: [..., "LANG.COLLECTIONS.LIFT", ...]` を宣言し、
`projection: { when: "divisorEqualsZero", reason: "divisionByZero" }` としている。
F-1 と同じ本文（レーン保存）に違反しており、裁定も同じ **impl → spec** である。

**受け入れ条件。** `[ 6 6 6 ] [ 1 2 0 ] /` → `[ 6/1 3/1 NIL ]`、
`[ 6 ] [ 1 2 0 ] /` → `[ 6/1 3/1 NIL ]`。conformance に両方を追加。
F-1 と原因を共有する可能性が高いが、**別 PR に分けること**（F-1 は表現の一本化、
F-2 はレーン射影の合成）。

### F-3（中・**所有者判断待ち**）ゼロ除数が DIV では NIL、MOD では ERROR

```
[ 6 ] [ 1 2 0 ] /   => status ok,    NIL(divisionByZero)
[ 6 ] [ 1 2 0 ] %   => status error, "Modulo by zero"
```

実装は registry のとおりである（`word_contract MOD` は
`errorWhen: ["nonNumeric", "shapeMismatch", "divisorEqualsZero"]`、
`projection.when` は無関係な `integerProjectionUndecidable`）。つまり
**実装バグではなく、仕様が両者を非対称に定めている。**

問題は、両者が同じ family（`exactArithmetic`）・同じ `partiality: projecting`・
同じ `LANG.FAILURE.TRICHOTOMY` を宣言していることである。三分法は
「値を作れない演算は NIL、壊れた演算は ERROR」と述べており、
0 による剰余は前者に読める。実使用でも「除算のゼロは NIL」と学んだ直後に
剰余で ERROR に落ちるのは学習則の裏切りになる。

**指示**: どちらへ寄せるかは所有者判断。決めてから実装すること。判断に要る材料:

- MOD を DIV に合わせる → 三分法が語をまたいで一貫。既存 `errorWhen` の削除と
  `projection` の追加、conformance 追加が要る。
- 現状維持 → `spec/words.json` の MOD の `documentation` に非対称の理由を明記し、
  SKILL.md §4 に「ゼロ剰余だけは ERROR」を書く。**黙って残すのは不可**。

### F-4（中）registry が宣言済みの条件が診断に出ない

宣言されている失敗条件が、診断では `category=custom` / `why=unknown` /
nextChecks は「メッセージを直接読め」の 1 件に潰れる。

| 実行 | registry の宣言 | 実際の診断 |
| --- | --- | --- |
| `4 NEG SQRT` | `projection: { when: "negativeScalar", reason: "domainMiss" }` | `why: "unknown"`, `kind: "custom"`, nextCheck=`checkErrorMessage` |
| `[ 6 ] [ 1 2 0 ] %` | `errorWhen: [... "divisorEqualsZero"]` | `why: "unknown"`, `kind: "custom"`, nextCheck=`checkErrorMessage` |
| `[ 0 100001 ] RANGE` | 限界 `materializedElements` | 同上（F-5 も参照） |

対照的に DIV は `checkDivisor` / `checkZeroIsExpected` / `checkDivisorOrigin` という
手書きの良い nextChecks を持つ。**手書きの表がある語だけが良い診断を得ている。**

さらに arity 不足が `"Stack underflow"` の一言で返る。実使用では
`'hello world' TOKENIZE` と `[ [ 1 2 ] [ 3 ] ] CONCAT` の 2 回、これで
ターンを浪費した（正解は `'a,b,c' ',' TOKENIZE` と `[ 1 2 ] [ 3 ] CONCAT`）。
registry は `stack: { inputs, outputs }` と `documentation.syntax` を持っているのに、
診断はそれを引いていない。

**指示**: 診断の `why` / `nextChecks` を、手書き表ではなく registry の
`projection.when` / `errorWhen` / `stack.inputs` から導出する。
最低限、(a) 宣言済み条件は `custom`/`unknown` にしない、(b) stack underflow は
語名・宣言 arity・`documentation.syntax` を返す。

**受け入れ条件**: 上表 3 行が宣言由来の `why` を返し、`TOKENIZE` の underflow が
`( 2 -- 1 )` 相当と `syntax` を含む。

### F-5（中）README が約束する `diagnosis.resourceLimit` が載らない

`tools/mcp-server/README.md` は次を約束している:

> A resource-limit failure carries `diagnosis.resourceLimit` (`{ resource, limit, observed }`),
> where `resource` is the name of the very entry in `mcp.limits` that fired.

しかし README 自身が例示する式で載らない:

```
[ 0 100001 ] RANGE
  => status ok, stack ["NIL"]
     stack[0].semantics.absence = { origin: "spaceBudget", reason: "spaceExhausted", ... }
     diagnosis.resourceLimit    = null      ← 約束と食い違う
```

限界（`materializedElements: 100000`）が発火したことは `absence.reason` から
分かるが、**どの限界が・いくつで・どこまでなら通るのか**が機械可読で返らない。
README が「repair 率が 0.750 → 1.000 に動いた根拠」として挙げている `progress`
（`{ completed, total, unit }`）も同じ経路であり、ここでは観測できなかった。

**指示**: 実装を README に合わせる（`resourceLimit` を載せる）か、README を実態に
合わせる（NIL 射影には載らないと明記する）かを決め、**どちらかに一本化する**。
前者を推奨する。理由は README 自身が書いている——限界に当たった呼び出し側が
次に必要なのは「どこまでなら入るか」であり、`observed` だけでは足りないと
自ら結論している。

### F-6（低）COND の NIL 主体が "expected a value, got NIL" で落ちる

```
NIL [ [ TRUE ] [ 'yes' PRINT ] ] COND
  => status error, "Structure error: expected a value, got NIL"
```

SKILL.md §4 の見出しは「NIL — absence is a value, not an exception」であり、
§5 は「NIL 被演算子は真理値位置で UNKNOWN として読まれる」と述べる。
実際 `NIL TRUE AND` → `NIL`、`NIL TRUE OR` → `TRUE`、`NIL NOT` → `NIL` は
三値論理として正しく動く（下の §4 参照）。COND だけが NIL を「値ではない」と
拒否し、しかもその文言が本文と正面衝突している。

**指示**: 最低限、文言を直す（「値ではない」ではなく「COND の主体に NIL は取れない」）。
そのうえで、NIL 主体を UNKNOWN として扱い全ガードを false にするのが正しいか
（＝ else 節に落ちるか）は仕様判断であり、F-3 と同じく所有者に差し戻す。

### F-7（低・文書）`[ 42 ]` idiom と PUT の要素意味論が衝突し、誤りが黙って通る

```
[ 1 2 3 ] [ 1 ] [ 9 ] PUT  => [ 1/1 [ 9/1 ] 3/1 ]    ← 入れ子。エラーにならない
[ 1 2 3 ] [ 1 ] 9 PUT      => [ 1/1 9/1 3/1 ]        ← 正しい
```

PUT は仕様どおり（`stackEffect: "[ vec ] [ idx ] [ x ] -> [ vec ]"`、
registry の `syntax` も `[ 1 2 3 ] 1 9 PUT`）であり、**実装バグではない**。
問題は SKILL.md §2 が「A lone number like `42` is allowed but `[ 42 ]` is the
idiomatic scalar」と教えていることで、その idiom に従うと PUT では黙って
入れ子データができる。エラーが出ないため、AI は誤りに気づかない。

**指示**: SKILL.md（=`scripts/generate-skill-md.mjs`）の当該記述に、要素を
受け取る語（PUT・GET・INDEX-OF など）では 1 要素 Vector が「その Vector 自身」を
意味する旨の例外を書く。§7「Common errors」に実行検証付きで 1 件足すのが
この repo の作法に合う（PR #1615 で定めた「バッククォートのコード片は
実行検証済みのものに限る」規律に従うこと）。

### F-8（低・文書）`infer_contracts` が二つの語彙を説明なしに併記する

```
[ [ 2 ] * [ 1 ] + ] 'F' DEF
  => contracts[0].nil        = "nil-propagating"
     contracts[0].suggested  = "#:contract F ( 1 -- 1 ) pure nil-free cost ..."
```

**実装は正しい。** 宣言語彙は `nil-free` / `may-nil` の 2 値しかなく
（`rust/src/agent/contract_decl.rs:197-217`）、`nil-free` は「自ら不在を製造しない」
の意で、伝播は含む（同 `:463-465` のコメント、"ADD1 is nil-free yet propagates"）。
`contract_report.rs:57-58` と `:94-95` がその 2 語彙を意図的に作り分けている。

問題は**応答にその説明が無い**ことである。読む側には同一オブジェクト内の
矛盾に見え、`suggested` を貼るべきか `nil` を信じるべきか判断できない。

**指示**: `result.schema.json` の当該フィールド description に、2 語彙の別と
「`nil-free` は非製造であって非伝播ではない」を明記する。

### F-9（低・文書）cost クラスが桁を隠す

同じ `const` クラスでも実測は 400 倍違う:

| 式 | 実測 `numericWork` |
| --- | --- |
| `[ 1 2 3 4 5 ] [ 0 ] [ + ] FOLD` | 5 |
| `2 SQRT 3 SQRT +` | 2048 |
| `2 SQRT 3 SQRT + 'S' BIND S S *` | 6144 |

`numericWork` 上限は 10,000,000 なので、代数値の加算は約 4,800 回で天井に当たる。
`word_contract` のツール説明は cost を「実行前に予算を見積もるために読め」と
勧めているが、クラス（`const` / `linear` / …）だけでは代数演算の桁を予測できない。

**指示**: 実装変更は求めない。`word_contract` の説明と MCP quickstart §7 に
「クラスは増え方であって大きさではない。代数値（SQRT 由来）は同じ `const` でも
有理数の 10^2〜10^3 倍を課金する」旨を、上表の実測付きで書く。

---

## 3. テスト網羅の穴

1. **`golden/cases.json` に NIL レーンを含む Vector への算術が無い。** SQRT 系は
   `[ 2 ] SQRT` / `1 2 / SQRT` / `2 SQRT 3 SQRT +` の 3 本のみで、F-1 の形は無い。
2. **`backend/parity-test.js` は F-1 を検出できない。** 両バックエンドは同一 Rust
   ソースのコンパイル結果であり、同じバグで一致するため緑のままである
   （方法論 §「検証の盲点」の「陳腐化した二者は互いに一致する」と同型）。
   パリティは「二者が同じ」ことしか言えず、「正しい」ことは言わない。
3. **`check` は F-1 の予防にならない。** 実行しない設計なので当然だが、
   落ちるプログラムに対して `status: ok` を返す。呼び出し側から見ると
   「check が通ったから安全」は成り立たない。README にこの限界を 1 行書くこと。

追加すべきケース（F-1/F-2 の PR に含める）:

| 対象 | ソース | 期待 |
| --- | --- | --- |
| conformance | `[ 1 NIL 3 ] [ 2 ] *` | `[ 2/1 NIL 6/1 ]` |
| conformance | `[ 4 -1 ] SQRT [ 2 ] *` | `[ 4/1 NIL ]` |
| conformance | `[ 6 6 6 ] [ 1 2 0 ] /` | `[ 6/1 3/1 NIL ]` |
| golden | 上記のいずれか 1 本 | MCP 応答としての形も pin する |

---

## 4. シナジー（壊してはならない強み）

改修の際、以下は**現状が正しい**。実測で確認した。

- **三値論理が一貫している。** `NIL TRUE AND` → `NIL`、`NIL TRUE OR` → `TRUE`、
  `NIL NOT` → `NIL`、`NIL NIL =` → `NIL`。Kleene K3 と一致する。
- **代数の厳密性が本物である。** `2 SQRT 3 SQRT + 'S' BIND S S * 5 6 SQRT 2 * + =`
  → `TRUE`（(√2+√3)² = 5+2√6 を厳密判定）。`2 SQRT 2 SQRT -` → `0/1`、
  `9 SQRT 3 =` → `TRUE`、20! → `2432902008176640000`。
- **不在を値として返す設計が効いている。** `[ 1 2 3 ] [ 5 ] INDEX-OF` → `NIL`
  （例外でも -1 でもない）、`[ 1 2 3 ] [ 9 ] GET` → `NIL`。
- **候補提示が効く。** `[ 1 2 3 ] LENGHT` → `candidates: ["LENGTH"]`。
- **DEF の自己参照拒否が定義時に出る。** `[ REC ] 'REC' DEF` →
  "Cannot define 'REC': self-referential definition (REC -> REC)"。
- **MCP の三分（ok / error / hostError）は機能した。** F-1 のプロセス abort も
  `hostError: backendFailure, retryable: false` として正しく分類され、
  言語エラーと混同されなかった。アダプタ側の設計は妥当である。

---

## 5. 作業順序と分割

| 順 | 対象 | PR | 備考 |
| --- | --- | --- | --- |
| 1 | F-1 | 単独 | 最優先。表現の一本化 + conformance |
| 2 | F-2 | 単独 | F-1 と原因を共有しうるが分ける |
| 3 | F-4・F-5 | まとめて可 | どちらも「registry / README にある情報が応答に出ていない」 |
| 4 | F-6 の文言 | 小 | 判断が要る部分は分離 |
| 5 | F-7・F-8・F-9 | まとめて可 | 生成プローズ側。PR #1615 の検証規律に従う |
| — | F-3・F-6 の意味論 | — | **所有者判断待ち。着手しない** |

各 PR は方法論の Phase 3 検証行列（`npm run check`/`lint`/`test`、
`cargo fmt`/`clippy`/`test --all-targets`、全 `*:check`、
`npm run test:mcp-backends`、`npm --prefix tools/mcp-server run selftest`）を
通してから完了とする。F-1・F-2 は `spec/` に触れないため
`npm run specification:generate` は不要だが、conformance を足すので
`tests/conformance/` の実行を忘れないこと。
