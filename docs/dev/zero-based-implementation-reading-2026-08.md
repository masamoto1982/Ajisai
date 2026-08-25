# ゼロベース実装読解 — 正典の散文を一切参照せず `rust/src` だけから再構成する（2026-08）

> Status: **Non-canonical / `[観察ノート]`.** 本書は Ajisai の意味論を一切定義しない。
> 正典は `spec/` 配下の各ソースのみ。本書はその正典を**意図的に一度も参照せず**、
> `rust/src` の実装だけを読んで「本当に美しい最小の公理系」を独立に再構成する
> 実験である。導出した記述が正典と一致しない箇所は、実装のバグか、正典の
> 記述不足か、正典と実装のどちらを正すべきかの三通りがあり得るが、本書は
> それを裁定しない。裁定は仕様所有者の仕事であり、本書は材料を提供するだけである。

## 0. 方法

`spec/*.md` `spec/*.json` `SPECIFICATION.html` を一切読まず、`rust/src` のソースと
その中のコメント・テストだけを読んだ。コメントも「実装の言明」として扱ったが、
コメントと実際のコード(型定義・match腕・実行される分岐)が食い違う場合は
**コードを事実として採用し、コメントは検証対象の主張として扱った**。

## 1. 実際に見つかった値モデル

```rust
struct Value {
    data: ValueData,               // タグ付き和
    hint: Interpretation,          // 表示ロール……のはずだが、実際は意味論も運ぶ
    absence: Option<AbsenceMetadata>,
}

enum ValueData {
    Boolean(bool),
    Scalar(Fraction),
    ExactScalar(ExactReal),        // Scalar の遅延無理数版（同一ドメイン内の第二表現）
    Vector(Arc<Vec<Value>>),
    Tensor { data: Arc<DenseTensor>, shape: Arc<Vec<usize>> },  // Vector の高密度キャッシュ
    Nil,
    Symbol(Arc<str>),
    Text(Arc<str>),
}
```

8 variant、しかし観測可能なドメインは6つ(Scalar/ExactScalarとVector/Tensorがそれぞれ
1ドメインの2表現に畳まれるため)。ここまでは既に読んだ正典の記述と一致しており、
実装は綺麗である。

**しかし `hint: Interpretation` が二重の役目を負っている。** 名前は「表示ロール」だが、
実際には `ValueData::Nil` を「真のNIL」と「論理的U」に**分岐させる唯一の手掛かり**に
なっている(§3)。これは「内部表現は観測不能、表示は値から導かれる」という値の同一性
原則(`LANG.VALUES.DENOTATION` 相当の規律、既に読んだ正典にある)と緊張関係にある——
`hint` が変われば `truth_value()` の返す意味論的な答え("unknown" か絶対に出ない値か)が
変わるので、これは純粋な表示情報ではなく、値の一部になっている。

## 2. 実行モデルは本当に「一つの分配 + 二つの規律」

`execute_word_core_inner`(`interpreter/execute_builtin.rs`)が唯一の分配点。
束縛 → 辞書解決 → `def.lines.is_empty()` で Core(id付きmatch) か User(本体ループ)かの
一分岐、それだけ。特殊語ごとの別分配経路は無い。

フレーム規律は実測で**本当に二つ、かつ独立したコード形状**として存在する:

- **透過(whole-stack)**: `execute_nested_block`。`EXEC` と `DEF` 本体、および
  `MAP`/`FILTER`/`ANY`/`ALL` のブロック呼び出し自体が使う下請け。スタックはそのまま。
- **隔離(isolated frame)**: `MAP`/`FILTER`/`FOLD`/`ANY`/`ALL`/`COND`のガード評価が使う、
  `mem::swap` でスタックを退避 → 対象値だけ積む → ブロック実行 → 1個だけ取り出す →
  退避したスタックを戻す、という共通パターン。

これは改修Ⅵ(「フレーム表を四行から二則へ」)が正典で主張した通りの構造で、
**実装がそれを裏付けている**。ここは美しい。ただし2つの規律は今もコードとして
別々に書かれていて、1つのパラメータ化された関数には収束していない
(「同じ操作の n=all/n=1 特殊形」として統一できる余地はある)。

## 3. 最大の発見 — 「K3三値論理」は実行時にほぼ存在しない

`ajisai-minimal-core-identity.md`(このセッションで既に読んだ設計メモ)は
Ajisaiの同一性を担う四本柱の一つとして次を挙げていた:

> `k3.domain/meet/join/involution`(#1-4) — **まだ分からない**を論理で GLB/LUB 透過
> 語(代表): `TRUE` `FALSE` `AND` `OR` `NOT`

実装を読むと、これは実行時には成立していない。

- `logic.rs` の `AND`/`OR`/`NOT` は**厳密に二値**。オペランドが真の Boolean で
  なければハードエラー。NIL パススルーの特例はあるが、Unknown 分岐は
  `compute_boolean_binary`/`compute_inverted_value` のどこにも無い。
- 「U」は `comparison.rs` の予算切れ比較(`ExactCmp::Starved`)が返す**観測軸の
  文字列**("unknown")としてのみ存在する。これはチェック時・診断用の投影であって、
  スタック上を流れる第三の真偽値ではない。
- U の実体は独自の値ドメインではなく、**`ValueData::Nil` に
  `Interpretation::TruthValue` という補助タグを付けただけ**
  (`value_absence.rs::truth_value_for_role` の `ValueData::Nil => Some("unknown")` 腕)。

つまり「K3」を名乗れるほどの三値論理系(AND/OR/NOTが三値をGLB/LUBで畳み込む)は、
今のランタイムには実装されていない。あるのは「NILの一種として符号化された、
表示軸でだけ 'unknown' と読める値」であり、それを三値論理として演算する語は無い。

### 3.1 さらに: 実装のコメント自身が、存在しない型を8箇所で主張している

`error.rs` `value_absence.rs` `execution_loop.rs` `nil_diagnostics.rs`
`nil_diagnostics_tests.rs` `protocol_string_tests.rs` など複数ファイルのコメント・
テストが、口を揃えてこう主張する:

> U は独自の `ValueData::Unknown` variant であり、NIL ノードではない。
> `is_unknown()` で検出せよ、内部表現を直接matchするな。
> (CS4 PR-2 で独自 variant 化、PR-3 で旧理由 `LogicallyUnknown` を廃止)

しかし:

- `types/mod.rs` の `ValueData` 定義に `Unknown` variant は存在しない(実測、§1)。
- `is_unknown()` という関数はリポジトリ全体に**定義が一つも無い**
  (`grep -rn "fn is_unknown"` がヒットしない)。
- `unknown_advertises_truth_valued_capability` というテスト(名前も上記の物語を
  前提にしている)は、実際には `Capability::TruthValued.as_protocol_str() ==
  "truthValued"` という無関係な文字列一致しか検証しておらず、`ValueData::Unknown`
  を一度も構築していない。

これは「CS4」という名の一連のリファクタリング(Uを独自型に昇格し、NILとの型レベルの
ファイアウォールを作る)が、**複数ファイルにわたって構想・文書化されたが、
実装そのものが存在しないか、実装後に差し戻されてコメントだけ取り残された**状態を
示している。改修Ⅲが `spec/words.json` で刈った「到達不能な契約」の残骸と同じ現象が、
今度は Rust のソースコード自身の中で(散文の正典ではなく!)起きている。

### 3.2 これは実害のあるバグでもある

`value_semantics.rs::capabilities()` は `ValueData::Nil` に無条件で
`Capability::NilPassthrough` を付与する:

```rust
ValueData::Nil => {
    capabilities.push(Capability::NilPassthrough);
    capabilities.push(Capability::Diagnosable);
    capabilities.push(Capability::AiExplainable);
}
```

すぐ上のコメントは「Uはこれを持ってはいけない、NilPassthroughを広告すると
Uを吸収してよいというファイアウォール違反の主張になる」と明記しているのに、
`hint` による分岐が実際には無い。したがって**予算切れ比較が返すUは、
MCP/ホストプロトコル経由で観測可能な `capabilities` 配列に
`"nilPassthrough"` を(誤って)含む**。これは今回のセッションの元々の目的
(長所をMCP経由でAIに開放する)に直接反する——エージェントは「これは
NILとして安全に読み飛ばせる」という誤った信号を受け取ることになる。

この不整合は他のどのコード経路からも `Capability::NilPassthrough` が
実際にクエリされていないため(protocol文字列としてシリアライズされるだけ)、
実行時の誤動作は起きない。しかし外部に公開される診断情報としては誤りである。

## 4. 副次的な発見 — 横断的関心事が「一箇所」に集約されていない

### 4.1 KEEP修飾子は、N個の演算実装それぞれに散らばっている

`KEEP` は `Interpreter` 上の一つのモードフラグだが、これを実際に読んで
「消費するかクローンするか」を決めるのは、`logic.rs` `comparison.rs` など
**個々の `op_*` 実装がそれぞれ自分で** `interp.consumption_mode ==
ConsumptionMode::Keep` をチェックするコードになっている。中央の分配点
(`execute_word_core_inner`)は User Word の呼び出し境界でだけこれを一括処理し
(オペランドをスナップショットして後で復元)、Core Word(組み込み語)については
各実装に委ねている。

### 4.2 NIL パススルーも同様に散らばっている

「理由付きNILは派生語を変えずに透過する」というBubbleパススルー規律
(`ajisai-minimal-core-identity.md`が同一性の柱の一つとして挙げるもの)は、
`logic.rs`・`comparison.rs`(`lift_comparison`)・共有ヘルパ
`nil_passthrough_binary` など、**個々の演算実装がそれぞれ`is_nil()`を
チェックして自前でパススルーする**形で実現されている。中央の分配点には、
NILが生成された後の診断トレース(`trace_direct_nil_produced`)はあるが、
NILパススルー自体を強制する中央ゲートは無い。

### 4.3 両者は同じ形の問題である

KEEPもNILパススルーも、「この語は特定の規律に従わなければならない」という
契約が、**一つの場所で強制されるのではなく、N個の実装がそれぞれ正しく
書くことに依存している**。これは`words.json`の`nilPolicy`/`consumption`
フィールドが宣言する契約と、実際にそれを守るコードの間に、機械的な
架け橋が無いことを意味する——ある `op_*` が新設されたとき、NILパススルーの
チェックを書き忘れても、コンパイラは何も言わない。§3で見つけたのと同じ
「宣言と実装の間に橋が無い」という病気の、三箇所目の発現である。

## 5. 副次的な発見 — もう一つの「宣言されているが読まれない」フィールド

`WordDefinition` は `Capabilities` というビットフラグ(`PURE`/`IO`/`TIME`/
`RANDOM`/`MUTATES_DICT`等、`types/mod.rs`)を持つ。ビルトイン語登録時に
値が設定され(例: `WordId::Print => Capabilities::IO`)、ユーザー語は常に
`PURE`が設定される。しかし**このビットフラグを実行時にチェックして何かを
許可/拒否するコードはランタイムに一つも無い**——`run_effect_schema`自身の
コメントが「出力だけが唯一ホストに届く効果なので、ゲートするcapabilityは
存在しない」と明言している。

これは改修Ⅲが `spec/words.json` から刈った `capability`/`hostedEffect`
フィールド(65語全件、実装参照0件)と**全く同じ形の死んだ宣言**だが、
場所が違う——`spec/`の正典ではなく、Rust内部の`WordDefinition`構造体に
今も生きている。改修Ⅲの監査は`spec/`とその読み手(生成スクリプト)だけを
grepしたため、この兄弟のような死骸をおそらく見落としている。

## 6. まとめ — 「本当に美しい」形と、今の形の差分

実装だけから逆算すると、Ajisaiの本当に最小の骨格は次の三点に見える。
これは正典が既に主張している三分法(絞り込みの三値: 値・NIL(理由付き)・
ERROR)と一致しており、そこは実装が裏切っていない。

1. **一つの状態**: (Stack, Dictionary, Output) の組。
2. **一つの値木**: タグ付き葉、または葉のVector。Tensorは全数値矩形Vectorの
   キャッシュに過ぎず観測上ほぼ透明。
3. **一つの分配**: 名前 → 組み込み(idマッチ) | ユーザー本体(トークン再生)。
   フレーム規律は今のところ二形態(透過/隔離)で、まだ一つのパラメータには
   畳み込まれていない。

対して、**今の実装との差分**は三つの具体的な穴として見える:

| 穴 | 症状 | 規模 |
|---|---|---|
| K3三値論理が同一性の柱として宣言されているが、AND/OR/NOTは二値のまま。Uは独自ドメインではなくNilの隠しタグ。 | 8ファイルのコメント/テストが存在しない型`ValueData::Unknown`を前提に書かれている。`capabilities()`がUに`NilPassthrough`を誤って付与し、MCP越しに観測可能。 | 大(同一性の主張と実装の間の食い違い) |
| KEEPとNILパススルーが中央集約されず、N個の`op_*`実装がそれぞれ自分で規律を守る形になっている。 | 契約(`words.json`の`nilPolicy`/`consumption`)を実装が守っているかを機械的に保証する橋が無い。新しい語を書くとき忘れても検出されない。 | 中(設計の一貫性、まだ実害の報告なし) |
| `WordDefinition::Capabilities`ビットフラグが設定されるが一切読まれない。 | 改修Ⅲが`spec/words.json`側で刈ったのと同じ死骸が、Rust内部に兄弟として残っている。 | 小(改修Ⅲと同じ手順で即座に刈れる) |

## 7. 提案(優先度順、いずれも未実施・未承認)

1. **[即時・低リスク]** `capabilities()`のNilPassthroughバグを塞ぐ——Uに相当する
   `hint == TruthValue`のNilには`NilPassthrough`/`Diagnosable`/`AiExplainable`を
   与えない分岐を足す。1関数の修正。
2. **[即時・低リスク]** §5の死んだ`Capabilities`ビットフラグを、改修Ⅲと同じ基準
   (「読み手が無い宣言は到達不能な契約」)で刈るか、実際に何かをゲートする
   ように配線するかを決める。
3. **[要・設計判断]** 「CS4」の物語(Uを独自variant化する)を、実際に最後まで
   実装するか、それとも8箇所のコメント/テストを今の実装(Nil+hintタグ)に
   合わせて書き戻すかを決める。中途半端な現状——正典は三値論理を柱として謳い、
   コードのコメントは独自型があると謳い、実際のコードはどちらでもない——が
   最も悪い。**この決定は`ajisai-minimal-core-identity.md`の§2.1にも波及する**
   (K3をidentity層の柱として維持するなら実装が要る。維持しないなら同メモの
   四本柱は三本になる)。
4. **[中規模・任意]** KEEPとNILパススルーを、改修Ⅵがフレーム表にしたのと同じ
   発想で、中央の分配点(`execute_word_core_inner`)に一本化できないか検討する。
   `words.json`の`nilPolicy`/`consumption`フィールドを「宣言するだけの散文」
   から「実際に実行を駆動する入力」に格上げする方向。

いずれも本書は実装・正典を一切変更していない。
