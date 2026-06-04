# Ajisai 数学的定式化の全面拡張 — 改修ロードマップ

> Status: **Non-canonical / planning.** 正典は `SPECIFICATION.md` のみ。
> 本書は計画文書であり、Ajisai の意味論・実行時挙動・互換性方針を定義しない
> (§2.2, §16.1「第二の設計権威を導入しない」を尊重)。仕様と食い違う場合は
> 仕様が優先する。本書は `docs/dev/ajisai-mathematical-formalization.md`
> (以下「形式化本体」)が与える意味関数 `⟦·⟧` を **Ajisai 全域へ拡張する**ための
> 工程表である。

## 0. 目的と現状

### 0.1 目的

形式化本体 §0 の主張——「いかなるプログラミング言語にも依存しない究極形は、
参照実装でも有限テスト集合でもなく、意味関数 `⟦·⟧` 自身を数式で与えること」——
を **Ajisai の全構成要素へ及ぼす**。現状の `⟦·⟧` は健全な核(数・論理・部分性・
合成)だけを覆っており、仕様の大半(高階語・構造データ・契約・名前解決・効果・
並行性)は未定義のまま実装の自由に委ねられている。本ロードマップはその空白を
段階的に埋め、`⟦·⟧` を全域関数にする。

### 0.2 被覆行列(出発点)

仕様各節を、形式化本体での扱いで分類する。`Defined` = 数式で定義済み・法則あり、
`Sketched` = 言及はあるが操作規則・法則が未完、`Absent` = 未着手。

| 仕様節 | 領域 | 現状 | 主な数学的道具(目標) |
|---|---|---|---|
| §3 | 構文・字句・脱糖 | **Defined**(Phase 2 済) | `tokenize/parse/desugar` を全域関数として |
| §4.1 / §5(Σ) | 値空間・配置 | Defined | 直和 `V`、`Σ = Stack×Dict×Eff` |
| §4.2 | 連分数スカラ | Defined | GL₂(ℤ) 行列積、遅延ストリーム |
| §4.2.5 / §7.4.1.1 | NICF 比較 | Sketched | 半正則展開・丸め・予算単位 |
| §4.3 / §7.2 | ベクトル・テンソル | **Defined**(Phase 5 済) | 添字函手・reshape 群作用・broadcast applicative |
| §4.4 | レコード | Absent | 順序保存有限写像 `Name ⇀ V` の代数 |
| §4.5 | NIL・absence metadata | Defined(metadata は Sketched) | Bubble モナド `M(X)=X+(⊥×R∞)` |
| §6 / §13 | 修飾子・質量保存 | **Defined**(Phase 3 済) | 変換子コンビネータ・線形(資源)型 |
| §7.1 | ベクトル操作語 | **Defined**(Phase 5 済) | `V*` 上の自由モノイド+部分添字写像 |
| §7.3 / §7.13 | 算術・丸め | Defined | 双一次変換(Gosper)・整数部抽出 |
| §7.4 | 比較・U 伝播 | Defined(7.4.2/7.4.3 は Sketched) | 予算付き観測 `cmp_β` |
| §7.5 | K3 論理 | Defined | De Morgan(Kleene)束 |
| §7.6 | 文字列・変換 | Absent | 符号点列・符号化契約 |
| §7.7 | 高階・制御語 | **Defined**(Phase 4 済) | 再帰スキーム(cata/ana/…)・K3 ガード case |
| §7.8 / §8 | ユーザ辞書・DEF | Absent | `Dict` 上の状態変換子・依存グラフ |
| §7.9 | IO・ユーティリティ | Absent | 効果ラベル・非決定性の分離 |
| §7.10 / §9 | モジュール・名前解決 | Absent | 可視性格子・決定的 `resolve` |
| §7.11 / §10 | 子ランタイム(並行) | Absent | 状態機械・小ステップ計算(探索的) |
| §7.14 | Coreword 契約 | **Defined**(Phase 3 済) | Hoare 契約+部分性/効果/安全性の格子 |
| §11 | 誤差モデル・Bubble Rule | Defined(述語は Sketched) | `Σ+Error` 二層・整形/不整形述語 |
| §12 | 意味プレーン・ロール | **Defined**(Phase 1 済) | `render : (data, role) → display` 純関数 |
| §2.3 | 観測軸(protocol) | **Defined**(Phase 1 済) | 観測代数 `observe` を semantic axes で |

当初「Ajisai のごく一部」だった被覆は、Phase 1(観測基盤)・Phase 2(構文)・
Phase 3(修飾子・契約・質量保存)・Phase 4(高階語)・Phase 5(構造データ)の完了で
言語表層の中核へ拡大した。残る `Absent` はレコード・文字列・辞書/名前解決・効果/IO・
並行性であり、新セッションで Phase 6/7/8/9 として取り組む。各完了フェーズの定義は
形式化本体 §9-bis / §9-ter / §9-quater、法則は
`rust/tests/{observation,desugar,contract_modifier,higher_order,structural}_laws.rs`。

### 0.3 進捗(本セッション)

| Phase | 状態 | 定義(D) | 法則・テスト(L/T) |
|---|---|---|---|
| 1 観測基盤 | ✅ 完了 | 本体 §9-ter D | `rust/tests/observation_laws.rs`(10 群)+ `tests/test_support/{generators,observe}.rs` |
| 2 構文・脱糖 | ✅ 完了 | 本体 §9-bis A | `rust/tests/desugar_laws.rs`(6 群) |
| 3 ⭐契約・修飾子・質量保存 | ✅ 完了 | 本体 §9-quater E | `rust/tests/contract_modifier_laws.rs`(11 群) |
| 4 高階・制御語 | ✅ 完了 | 本体 §9-bis B | `rust/tests/higher_order_laws.rs`(11 群) |
| 5 構造データ | ✅ 完了 | 本体 §9-bis C | `rust/tests/structural_laws.rs`(10 群) |
| 6,7,8,9 | 未着手 | — | 新セッション |

---

## 1. 改修方針(全フェーズ共通の原則)

1. **descriptive を厳守する。** 形式化は仕様が定める現象のモデルであり、第二の
   設計権威を作らない(§2.2, §16.1)。乖離が出たら「仕様を直す/実装を直す/
   モデルを直す」のいずれかを **finding として明示**し追跡する(本体 §9.3 の
   所見 B・C と同じ運用)。
2. **各フェーズは 4 点セットで完結させる。**
   - **(D) 定義**: 形式化本体に意味方程式を追記。
   - **(L) 法則**: その領域で全入力に成り立つ代数等式を導出。
   - **(T) テスト**: 法則を `rust/tests/` の性質ベーステストとして実行可能化
     (`algebraic_laws.rs` の様式を踏襲、領域ごとに生成器を設計)。
   - **(X) 乖離・参照**: 仕様節への相互参照と、検出した乖離の finding 化。
3. **観測は protocol 軸のみ。** 等式の左右一致は §2.3 の semantic axes
   (`semanticKind`/`shape`/`capabilities`/`truthValue`/`origin`/`absence`)と
   `render` を通して判定する。Rust enum 名・Debug 文字列・表示テキストに分岐しない
   (semantic firewall)。
4. **完全性の方向**: 有限標本(conformance 37 点)→内包的全域関数 `⟦·⟧`。
   conformance は `⟦·⟧` の検証用標本へ格下げされる(本体 §8, §9.1)。
5. **段階独立**: 各フェーズは単独で PR 可能。依存があるものだけ順序を固定する
   (Phase 2 → Phase 3/4 が前提依存、それ以外は概ね並行可能)。

---

## 2. フェーズ計画

依存と価値で並べる。⭐ は「実装非依存性への寄与が特に大きい」フェーズ。

### Phase 1 — メタ理論基盤と観測代数の確定

形式化本体の前提を全フェーズが共有できる形に固める。

- **(D)** 観測関数を §2.3 の semantic axes で再定義する:
  `observe(p) = (render(π_Stack ⟦p⟧ σ₀), π_Eff ⟦p⟧ σ₀)`、
  `render : (ValueData, Role) → Display` を**全ロールについて**純関数として与える
  (本体 §8 の `render` を §12.2 の全 role へ拡張)。
- **(D)** 被覆行列(本書 §0.2)を形式化本体へ移し、各意味方程式に被覆タグを付す。
- **(T)** 性質ベーステストの土台を「意味領域ごとの生成器(generator)」へ再構成し、
  以後のフェーズが生成器を足すだけで法則を追加できるようにする。
- **成果**: 「等式が一致するか」の判定基盤と、拡張の足場。

### Phase 2 — 構文層の全域化(字句・構文・脱糖)

`⟦·⟧` の準同型(本体 §2)は「生成元が裸の語ではなくフレーズ」であることに依存する。
その前提を構文関数として厳密化する。

- **(D)** `tokenize : Source ⇀ Token*`(§3.1–3.3、文字列境界規則、`(`/`)` 禁止
  =トークナイザ誤り、`>CF` 等 conversion word の単一トークン化)。
- **(D)** `parse : Token* ⇀ Phrase*`、`desugar`(中置 `=>`/`==`、修飾子糖衣
  `;`/`;;`、区切り糖衣)を §3.9 の表に従い関数化。脱糖後に §2 の準同型が
  厳密に閉じることを示す。
- **(L)** 脱糖の健全性: `⟦desugar(s)⟧ = ⟦s⟧`、糖衣と正準形の観測同値
  (`. ,` ≡ `;`, `>` ≡ `GT` など)。
- **(T)** 糖衣/正準形ペアの観測一致を網羅。

### Phase 3 ⭐ — 修飾子と質量保存の型理論(契約)

最重要。最適化経路の健全性(§13)と契約(§7.14)を「実装に依らず静的に検証可能な
型システム」として与える。

- **(D)** 修飾子コンビネータ `⟦μ·w⟧ = κ_consume ∘ δ_region ∘ base(w)` の operational
  規則を完成(TOP/STAK の被演算域選択、EAT/KEEP の消費/複製、4 組の閉性、§6)。
- **(D)** Coreword 契約(§7.14)を Hoare 的契約 `{requires} w {ensures}` と、
  `partiality`(Total/Partial/Projecting)・`nil_policy`・`safety_level`・`effects`
  の **格子(lattice)**として型付け。契約欠落=型不在=適合違反。
- **(D)** 質量保存(§13.1)を**線形(資源)型の不変条件**として定式化:
  arity・consumption・production・bifurcation(`,,`)を資源計算で表し、過消費・
  未消費漏れ・流量分岐比違反を**型検査の失敗**として静的に検出する。最適化経路は
  契約検証後にのみ進入可能、という規則を型保存定理で裏づける。
- **(L)** 修飾子の関手性、契約合成(`•` 下での requires/ensures 伝播)、質量保存が
  合成で閉じること。
- **(T)** 4 修飾子組の stack 効果、契約メタデータ駆動の質量検査。

### Phase 4 ⭐ — 再帰スキームとしての高階・制御語

仕様の大きな空白。`MAP/FILTER/FOLD/UNFOLD/SCAN/ANY/ALL/COUNT/COND/EXEC/EVAL`(§7.7)。

- **(D)** `FOLD` = catamorphism、`UNFOLD` = anamorphism、`SCAN` = 中間累算列、
  `MAP` = 添字函手上の関手持ち上げ、`FILTER` = 述語による部分列抽出、
  `ANY/ALL/COUNT` = 述語の存在/全称/計数量化。
- **(D)** `COND` を **K3 ガード付き case** として定義(§7.4.3:U ガードは不発火、
  `false` と同様に次節へ落ちるが `truthValue=unknown` は観測可能)。`CondExhausted`
  の扱いを含める。
- **(D)** `EXEC` = `Blk` の脱参照適用、`EVAL` = `⟦·⟧` の reflection
  (メタ循環)。`EVAL(STR e)` と `e` の関係を法則として明示。
- **(L)** map fusion(`MAP f ∘ MAP g ≡ MAP (f∘g)`)、fold universality、
  filter の冪等・可換、map/fold 融合、`ALL ≡ ¬ ANY ¬`(K3 下)。
- **(T)** 上記法則 + COND の U 不発火(本体テストの K3 比較 `√2 √2 SUB 0 EQ` を再利用)。

### Phase 5 — 構造データ:ベクトル・テンソル・レコード

- **(D)** ベクトル操作語(§7.1)を `V*` 上の自由モノイド(`CONCAT`/`REVERSE`)+
  部分添字写像(`GET` 範囲外→Bubble、§11.2)として。`RANGE`/`TAKE`/`SPLIT`/
  `REORDER`/`COLLECT`/`SORT`(U 伝播は §7.4.3)。
- **(D)** テンソル(§4.3/§7.2)を `Tensor ≅ (V*, shape)` 上の **reshape 群作用**、
  broadcast を applicative(zip、長さ 1 次元の拡張)として。段階パイプライン
  「平坦化→形/ストライド→添字変換→再構築」を圏論的図式に。dense/nested 二表現の
  **観測同値(同型)**を定理化(§4.3.1、No-Rebuild 原理)。
- **(D)** レコード(§4.4)を挿入順保存の有限写像 `Name ⇀ V` の代数として。
- **(L)** `RESHAPE∘RESHAPE`、2D `TRANSPOSE` の対合、`CONCAT` 結合律、`REVERSE`
  対合、broadcast の自然性、dense≡nested 観測一致。
- **(T)** 上記 + ランク/形の整合。

### Phase 6 — 名前解決と辞書(モジュール・DEF)

- **(D)** `Dict = Name ⇀ Blk`、可視性状態を**格子**(core / module:
  imported・unimported・partial / user)として。`resolve : Name × Vis ⇀ Blk + Unknown`
  を決定的関数化(Core→imported の解決順、`MODULE@WORD` の限定解決、§7/§9)。
- **(D)** `DEF`/`DEL` を `Dict` 上の状態変換子+**依存グラフ**(`FORC` ガード、§8.2)。
- **(D)** `IMPORT`/`IMPORT-ONLY`/`UNIMPORT`/`UNIMPORT-ONLY` を可視性格子上の
  単調作用素として(§9.2、参照保持規則、core-listed 語の no-op)。
- **(L)** import の冪等、unimport が被参照語を保つこと、解決順の決定性、
  境界語(boundary word)の bare/`MODULE@WORD` 解決一致(§7.14)。
- **(T)** 名前解決表・可視性遷移・依存ガード。

### Phase 7 — 効果と観測(ホスト効果・IO・意味プレーン)

- **(D)** `Eff` を構造化ホスト効果の自由モノイド/効果代数として(§5.2)。
  `observe` の `π_Eff` 成分=順序付き効果列(conformance の観測対象)。
- **(D)** SERIAL の receive-buffer(§9.4)を「実行前注入・`READ` が inbox を drain・
  run 内決定的な event-poll」状態として。`READ` の `noData`/`portDisconnected`
  射影を Bubble として。
- **(D)** 意味プレーン(§12)の `render` をデータ面非干渉な純関数として定理化
  (§5.2 の二面分離)。`NOW`/`CSPRNG`/`HASH` の非決定性を効果ラベルで分離し、
  Core profile では純粋部分のみが `⟦·⟧` に残ることを示す(Portability Profiles)。
- **(L)** 効果列の順序保存、データ面/意味面の独立(意味面変更が計算に非干渉)、
  run 内 SERIAL 決定性。
- **(T)** 効果列観測・受信バッファ drain・ロール別 render。

### Phase 8 — 並行性(子ランタイム)[最難・探索的]

`SPAWN/AWAIT/STATUS/KILL/MONITOR/SUPERVISE`(§10)。完全な denotational 化は研究的
なので、まず**観測レベル**に留める。

- **(D)** snapshot 隔離(spawn 時に親辞書の写しを受け、stack/dict を非共有)と
  状態遷移(`running→completed/failed/killed/timeout`)を状態機械として。
- **(D)** `AWAIT` の観測(`[status result-stack]`)を、子の `⟦·⟧` の最終配置の
  射影として定義。小ステップ・インターリービング or プロセス計算(CCS/π 的)での
  denotation は **探索課題**として明示し、過剰投資を避ける。
- **(L/T)** 親子隔離(子の効果が親 stack を変えない)、`AWAIT` 後の結果一致、
  決定的入力下での再現性。
- **注意**: 本フェーズは「完全形式化」より「観測契約の固定」を優先する。

### Phase 9 — 統合・検証・自己ホストへの道

- **(D)** `⟦·⟧` の被覆を全節へ。被覆行列を `Defined`(または明示的に
  `Out-of-scope`)で埋める。
- **(X)** conformance suite を `⟦·⟧` の標本として再記述し、性質ベーステストを
  §ごとに常時グリーン化(本体 §9.4)。
- **(検証)** 証明支援系(Lean/Coq/Agda)への機械化を**核から検討**(数・K3・
  モナド・モノイド)。参照実装が `⟦·⟧` を refine することの証明を長期目標に。
- **(オラクル)** 「法則破れ=実装が Ajisai でない」を CI で自動可視化し、仕様改訂への
  フィードバックループを定着(本体 §9.3)。

---

## 3. 成果物まとめ(フェーズ→4 点セット)

| Phase | D(定義) | L(法則) | T(テスト) | 依存 |
|---|---|---|---|---|
| 1 観測基盤 | render 全ロール・観測代数 | — | 生成器土台 | — |
| 2 構文 | tokenize/parse/desugar | 脱糖健全性 | 糖衣≡正準 | — |
| 3 ⭐契約 | 修飾子・Hoare 契約・線形質量保存 | 関手性・契約合成 | 修飾子組・質量検査 | 2 |
| 4 ⭐高階 | cata/ana・K3 case・EVAL | map fusion・fold univ | 融合則・COND U | 2,3 |
| 5 構造 | V*/Tensor/Record 代数 | reshape/transpose 律 | 同型・形整合 | 1 |
| 6 名前解決 | Dict・resolve・可視性格子 | import 冪等・解決決定性 | 解決表・遷移 | 1 |
| 7 効果 | Eff 代数・render 非干渉 | 順序保存・二面独立 | 効果列・drain | 1 |
| 8 並行 | 隔離・状態機械(観測) | 親子隔離・再現性 | AWAIT 観測 | 7 |
| 9 統合 | 全域 `⟦·⟧`・機械化検討 | — | 全節グリーン | all |

---

## 4. リスクと判断指針

- **第二権威化リスク**: 形式化が仕様を追い越して規範化する危険。→ 厳密に
  descriptive を維持し、規範は常に `SPECIFICATION.md` に還元する(§1 原則 1)。
- **並行性・効果の過剰形式化**: Phase 8 の完全 denotational 化は研究的で投資対効果
  が読みにくい。→ 観測契約の固定を優先し、完全形式化は探索課題として分離。
- **生成器の浅さ**: property test の生成器が弱いと法則が空虚化。→ 領域ごとに
  境界値(零・符号・NIL・無理数・空ベクトル)と shrink を設計する。
- **乖離処理の一貫性**: 乖離を見つけたとき場当たり修正に流れる危険。→ 所見 B・C と
  同じく finding として記録し、仕様改訂・conformance 追加・テスト追加を一括で行う。

## 5. 成功基準

1. 被覆行列の全節が `Defined` か明示的 `Out-of-scope`。
2. 各意味領域に最低 1 組の代数法則が実行可能テストとして存在し常時グリーン。
3. conformance suite が `⟦·⟧` の標本として位置づけ直されている。
4. 形式化が検出した乖離がすべて finding 化され、仕様・テストへ反映済み。
5. 核(数・論理・モナド・モノイド・契約)の証明支援系機械化の実現可能性評価が完了。
