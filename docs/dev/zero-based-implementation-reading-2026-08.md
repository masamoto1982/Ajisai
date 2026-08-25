# ゼロベース実装読解 — 正典の散文を参照せず `rust/src` だけから再構成する（2026-08）

> Status: **Non-canonical / `[観察ノート]`.** 本書は Ajisai の意味論を一切定義しない。
> 正典は `spec/` 配下の各ソースのみ。本書は正典を**意図的に一度も参照せずに**
> `rust/src` だけを読んで言語の骨格を独立に再構成し、**その後で**正典と突き合わせた
> 記録である。実装・正典ともに一切変更していない。

## 0. 方法と、その途中で自分が犯した誤り

`spec/*.md` `spec/*.json` `SPECIFICATION.html` を読まずに `rust/src` だけを読み、
骨格を再構成した。コメントも「実装の言明」として読んだが、コメントとコード
（型定義・match腕・実行される分岐）が食い違う場合は**コードを事実とし、
コメントは検証対象の主張として**扱った。

**本書の初稿は、この最後の規律を自分自身に適用し損ねて誤った結論を出した。**
初稿は「正典が三値論理を柱として謳っているのに実装は二値だ」と書いた。実際には
正典は三値論理を謳っていない——`LANG.VALUES.TRUTH` の見出しは
"**Two-valued truth**" であり、本文は「真偽領域はちょうど TRUE と FALSE の二値、
すべての比較は決定する」「NIL は未決ではなく不在である」と明記している。
突き合わせを最後に回した手順自体は正しかったが、突き合わせの相手を
`ajisai-minimal-core-identity.md`（非正典の設計メモ）と取り違えたまま結論を書いた。
訂正した結論が §3 であり、この誤りの構造そのものが §3.3 の論点になる。

## 1. 実装から再構成した骨格

```rust
struct Value {
    data: ValueData,
    hint: Interpretation,
    absence: Option<AbsenceMetadata>,
}

enum ValueData {
    Boolean(bool),
    Scalar(Fraction),
    ExactScalar(ExactReal),   // Scalar と同一ドメインの第二表現（遅延無理数）
    Vector(Arc<Vec<Value>>),
    Tensor { .. },            // Vector と同一ドメインの第二表現（全数値矩形のキャッシュ）
    Nil,
    Symbol(Arc<str>),
    Text(Arc<str>),
}
```

8 variant、観測可能なドメインは 6。二組の (ドメイン, 高速表現) の対
——`Scalar`/`ExactScalar` と `Vector`/`Tensor`——が variant 数と
ドメイン数の差を説明しきる。**実装だけを読んで数えたドメイン数が、後で
突き合わせた正典の `LANG.VALUES.DISJOINT`（Scalar/Boolean/String/Vector/NIL/Symbol）
と一致した。** 表現の選択はどちらの対でも観測不能である（§5）。

実行モデルも同様に素直だった。分配点は
`execute_word_core_inner`（`interpreter/execute_builtin.rs`）ただ一つで、
束縛 → 辞書解決 → `def.lines.is_empty()` による Core/User の一分岐しかない。
語ごとの特別な分配経路は存在しない。フレーム規律は実測で本当に二つ:

- **透過（whole-stack）** — `execute_nested_block`。`EXEC` と `DEF` 本体。
- **隔離（isolated frame）** — `mem::swap` でスタックを退避し、対象値だけを積み、
  ブロックを実行し、結果を 1 個取り出して復元する。`MAP`/`FILTER`/`FOLD`/`ANY`/`ALL`
  と `COND` のガード評価。

改修Ⅵ（「フレーム表を四行から二則へ」）が正典で主張した構造を、実装が裏切って
いない。ただし二則は今も別々のコード形状であり、「n=all と n=1 の特殊形」として
一つの関数にパラメータ化する余地は残っている（実害はなく、美観の問題）。

## 2. 数値タワーは、実装の中で最も美しい部分である

`types/exact/` は三層に分かれ、層は `Observation` という単一のインターフェース
（`observation.rs`）を共有する。値がどの層を流れるかは観測不能である。

| tier | 表現 | 比較の停止性 |
|---|---|---|
| 0 | `Rational(Fraction)` | 即決（区間が一点） |
| 1 | `AlgebraicSqrt` / `Gosper` | 有限の water で必ず決定 |
| 2 | `Computable`（遅延縮小区間） | **starve しうる** |

`Refine::Starved` を返せるのは Tier 2 だけであり（`observation.rs`）、
これが論理的 Unknown（U）の唯一の正当な源である。そして `computable.rs` は
自らこう宣言する:

> **No vocabulary constructs this tier yet.** The type exists so the `ExactScalar`
> enum, the comparison router, and the U diagnosis have their Tier 2 arms wired
> and tested ahead of the first Tier 2 word; unit tests pin that the current
> vocabulary cannot reach it.

つまり Tier 2 は**死んだコードではなく、意図的な先行足場**である。`pi.rs` が
π を Tier 2 の厳密区間として既に実装しており（語彙からは到達不能）、
将来 π・e・log を入れる日のために比較ルータと U 診断の腕が配線され、
テストで「現在の語彙からは到達できないこと」が固定されている。

**この設計の帰結が、正典の「比較は全域である」という主張の正体である。**
数値領域は「有理数と √ 拡大」と列挙されているのではなく、
「実装が提示する正規形により等号と順序が有限時間で決定できる実数の全体」という
**条件**で定義されている（`LANG.VALUES.EXACT`。改修Ⅳの成果）。Tier ≤1 はその条件を
満たす現在の証拠であり、条件を満たすからこそ比較は決定する。全域性は別途の保証
ではなく、領域の定義の半分である。

実測でも一致する:

```
8 SQRT 2 SQRT 2 SQRT + =   → TRUE
2 SQRT 2 SQRT =            → TRUE
```

## 3. 訂正された発見 — 食い違っているのは正典ではなく、非正典の記述である

初稿の結論は誤りだった。正典と実装は一致している。食い違っているのは
**それ以外の三つの表面**である。

### 3.1 実装のコメント 8 箇所が、存在しない型を前提に書かれている

`error.rs` `value_absence.rs` `execution_loop.rs` `nil_diagnostics.rs`
`nil_diagnostics_tests.rs` `protocol_string_tests.rs` などのコメントとテスト名が、
口を揃えて次を主張する:

> U は独自の `ValueData::Unknown` variant であり、NIL ノードではない。
> `is_unknown()` で検出せよ、内部表現を直接 match するな。
> （CS4 PR-2 で独自 variant 化、PR-3 で旧理由 `LogicallyUnknown` を廃止）

実測:

- `types/mod.rs` の `ValueData` に `Unknown` variant は**無い**。
- `is_unknown()` は**定義が一つも無い**（`grep -rn "fn is_unknown" src/` が空）。
- `unknown_advertises_truth_valued_capability` というテストは、名前に反して
  `Capability::TruthValued.as_protocol_str() == "truthValued"` という無関係な
  文字列一致しか検証せず、U を一度も構築しない。

「CS4」と呼ばれる一連の改修が、複数ファイルにわたって**構想され文書化されたが、
型そのものは landing しなかった**（あるいは landing 後に差し戻され、コメントだけ
取り残された）。U の実体は今も `ValueData::Nil` に `Interpretation::TruthValue`
を付けた値であり、`truth_value_for_role` の `ValueData::Nil => Some("unknown")`
の腕がそれを表示軸へ投影している。

### 3.2 `ajisai-minimal-core-identity.md` §2.1 の K3 の柱は、現行正典と矛盾する

同メモは Ajisai の**同一性を担う**四本柱の一つとして次を挙げる:

> `k3.domain/meet/join/involution`(#1-4) — **まだ分からない**を論理で GLB/LUB 透過
> 語（代表）: `TRUE` `FALSE` `AND` `OR` `NOT`

現行正典 `LANG.VALUES.TRUTH` は真偽を二値と規定し、`AND`/`OR`/`NOT` を
「通常の Boolean 演算」と規定する。`logic.rs` はその通り厳密に二値で、
Unknown 分岐を持たない（NIL パススルーの特例はある）。したがってこの柱は
**現行の言語には存在しない**。

同メモは概念削減・単一軸提案より前に書かれており、その後の決定を反映していない。
決定的なのは `ajisai-single-axis-proposal-2026-08.md` §4「採らない案」で、
そこには次が明記されている:

> `UNKNOWN` / 三値 Kleene 論理の復活 — 軸はこれを**必要としない**。絞り込みの
> 途中状態は検査時にのみ存在し、実行時の値には決してならない。……**改修Ⅱを
> 「UNKNOWN の復活」と読んではならない。**

同提案の改修Ⅰ〜Ⅶはすべて実装済みである。よって「実行時に三値論理を持たない」は
実装の遅れではなく、**承認され実施された設計判断**である。

### 3.3 三つの表面は、同じ一つの病気である

初稿の誤りも含めて、これらは同型である——**宣言と、それを裏づける機構との間に
橋が無い**。

| 表面 | 宣言 | 橋の不在 |
|---|---|---|
| Rust のコメント（§3.1） | 「U は独自 variant」 | コメントは型検査されない |
| 設計メモ（§3.2） | 「K3 は同一性の柱」 | 非正典文書は CI が読まない |
| 本書の初稿 | 「正典が三値を謳う」 | 私が正典を最後まで読まなかった |

改修Ⅲが `spec/words.json` から刈った「到達不能な契約」（実装参照 0 件のフィールド
65 語全件）と、これは同じ病気の別の宿主である。改修Ⅲはゲート
（`check:unreachable-contract`）を作って `spec/` のフィールド名に対しては再発を
止めたが、その射程は `spec/` のトップレベル分類フィールドに限られる。Rust の
コメントも、`docs/dev/` の設計メモも、そのゲートの外にある。

## 4. 派生した発見

### 4.1 `capabilities()` は U に `NilPassthrough` を与える — 潜在的な罠

`value_semantics.rs::capabilities()`:

```rust
ValueData::Nil => {
    capabilities.push(Capability::NilPassthrough);
    capabilities.push(Capability::Diagnosable);
    capabilities.push(Capability::AiExplainable);
}
```

直上のコメントは「U はこれを持ってはならない。`NilPassthrough` を広告することは
『U を NIL として吸収してよい』という、ファイアウォールに反する主張になる」と
明記するが、`hint` による分岐は無い。

**これは現在バグとして発火しない。** U は Tier 2 からしか生じず、Tier 2 は
語彙から到達不能だから（§2）、到達可能なすべての `ValueData::Nil` は本物の NIL で
あり、`NilPassthrough` はそれらに対して正しい。初稿はこれを「MCP 越しに観測可能な
実バグ」と書いたが、**誤りである**。

正確には、**最初の Tier 2 語が入った瞬間に発火するように装填された罠**である。
`computable.rs` は「比較ルータと U 診断の腕を先行配線した」と述べるが、
`capabilities()` はその先行配線から漏れている。Tier 2 を入れる作業が、
このファイルを見に来る保証は何も無い。

### 4.2 KEEP と NIL パススルーは、N 個の実装がそれぞれ守っている

`KEEP` は `Interpreter` 上の一つのモードフラグだが、Core 語については
個々の `op_*` が自分で `interp.consumption_mode == ConsumptionMode::Keep` を読んで
分岐する。中央の分配点が一括処理するのは User 語の呼び出し境界だけである
（オペランドをスナップショットし、`restore_kept_operands` で復元する）。

NIL パススルーも同様に、`logic.rs`・`comparison.rs`（`lift_comparison`）・共有ヘルパ
`nil_passthrough_binary` などが個別に `is_nil()` を見る。中央の分配点にあるのは
生成後の診断トレース（`trace_direct_nil_produced`）であって、パススルーを
強制するゲートではない。

`words.json` の `nilPolicy`/`consumption` が宣言する契約と、それを守るコードの間に
機械的な橋が無い。新しい `op_*` がチェックを書き忘れても、コンパイラは何も言わない。
§3.3 の病気の、四つ目の宿主である。

### 4.3 `WordDefinition::Capabilities` は設定されるが一度も読まれない

`Capabilities` ビットフラグ（`PURE`/`IO`/`TIME`/`RANDOM`/`MUTATES_DICT`）は
ビルトイン登録時に設定され（`WordId::Print => Capabilities::IO`）、ユーザー語は
常に `PURE` になる。しかし**実行時にこれを読んで許可/拒否する経路は一つも無い**。
`run_effect_schema` 自身のコメントが「出力だけが唯一ホストに届く効果なので、
ゲートすべき capability は存在しない」と述べている。

改修Ⅲが `spec/words.json` から刈った `capability`/`hostedEffect` と同じ形の
死んだ宣言が、Rust 内部の構造体に残っている。改修Ⅲの監査は `spec/` とその
読み手（生成スクリプト）を grep したため、この兄弟を見落としたと思われる。

## 5. Tensor / Vector の観測不能性（確認）

昇格条件は `value_tensor.rs::try_collect_dense` が決める: 全葉が
`ValueData::Scalar` で、各段が矩形で、かつ全 Fraction の分子分母が `i64` に
収まること（`tensor_storage.rs::extract_i64_pair`）。一つでも欠ければ静かに
ネスト `Vector` に落ちる。昇格は既定であり、明示的な選択ではない。

観測面では `eq`/`as_vector_view`/`shape()`/`len()`/`child()` が両表現を同一に
扱い、`push_child` は Tensor を Vector へ hydrate してから進む。表現を言語から
問う Core 語は無い。**「内部表現は観測不能」という規律が、ここでは実際に
守られている。**

## 6. まとめ

実装だけから再構成した骨格は三点に収束し、正典と一致する。

1. **一つの状態** — (Stack, Dictionary, Output)。
2. **一つの値木** — タグ付き葉、または葉の Vector。二組の高速表現は観測不能。
3. **一つの分配** — 名前 → 組み込み | ユーザー本体。フレーム規律は二則。

そして数値領域は、条件（有限時間で等号と順序が決定できること）によって定義され、
Tier ≤1 がその証拠であり、Tier 2 の足場が「条件を満たさない値をどう扱うか」を
先行して配線している。**「比較は全域である」は宣言ではなく、この構造の帰結である。**

利用者の言う三分法——欠落・未決・不正——を実装に照らすと、次のように読める。

| | 実行時 | 検査時 |
|---|---|---|
| 欠落 | 理由付き NIL（`AbsenceMetadata`） | — |
| **未決** | **存在しない（構造的に排除されている）** | `cannot verify` + `gap.*` |
| 不正 | ERROR（構造化診断） | `violated` |

**未決が実行時に無いのは欠落ではなく、設計の成果である。** 領域を「決定できる値の
全体」と定義したから、未決な比較は原理的に生じない。三分法の中辺は検査時
（契約推論の三値: verified / cannot verify / violated）へ移されており、
`trichotomy-unification.md` が「二つの三分法は同一の軸を二つの高さで切ったもの」と
論じたのは、まさにこの構造である。実行時の三分法は
**値 / 理由付き不在 / 不正**であって、真偽の三値ではない。

## 7. 提案（優先度順・いずれも未実施）

1. **[即時・低リスク]** §3.1 の 8 箇所のコメントとテスト名を、実装の現状
   （U は `Nil` + `TruthValue` hint、Tier 2 からのみ到達可能、現在は語彙から
   到達不能）に合わせて書き戻す。存在しない型と関数を指す記述は、読み手を
   確実に誤らせる——本書の初稿がその実例である。
2. **[即時・低リスク]** §4.1 の `capabilities()` に `hint == TruthValue` の
   分岐を足し、罠を今のうちに解除する。到達不能な今こそ、挙動を変えずに
   直せる唯一の時期である。Tier 2 語が入ってからでは、これは回帰になる。
3. **[即時・低リスク]** §4.3 の死んだ `Capabilities` ビットフラグを、改修Ⅲと
   同じ基準で刈るか、実際に何かをゲートするよう配線するかを決める。
4. **[要・仕様所有者の裁定]** §3.2 の `ajisai-minimal-core-identity.md` §2.1 を
   訂正する。K3 は identity 層の柱から外れ、四本柱は三本になる
   （欠落の透過 / 予算付き順序 / 構造化診断）。あるいは Tier 2 の到来時に
   K3 が実行時へ入ってくるという条件付きの柱として書き直す。
   **同メモは「Ajisai の同一性とは何か」を定義する文書であり、そこに現行言語に
   存在しない柱が立っていることは、他のどの stale よりも重い。**
5. **[中規模・任意]** §4.2 の KEEP と NIL パススルーを中央の分配点へ寄せ、
   `words.json` の `nilPolicy`/`consumption` を「宣言するだけの散文」から
   「実行を駆動する入力」へ格上げできないか検討する。

提案 1・2・3 は互いに独立で、いずれも観測可能な挙動を変えない。
