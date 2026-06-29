# 改修指示書: Ajisai Identity Model と文書ロールの明確化、`=>` 残骸の整理

Status: **Instruction (non-canonical) / 実装担当: Codex**
Authority: 非正典。本書は作業指示であり、言語意味論を定義しない。言語意味論の
正準文書は `SPECIFICATION.html`(Specification Authority 節)のみ。本書の指示に
従って `SPECIFICATION.html` を改訂した時点で、その改訂部分が正準になる。

関連既存文書(本書はこれらを置き換えず、束ねる):

- `SPECIFICATION.html` §2.3(内部表現の非観測性)、§4.2.4(表現の同値)、§4.2.5
  (NICF 比較展開)、§4.2.6(数値誤差方針)、§7.4.1(比較予算と Undecidable)、
  §12.1–12.2(`render = (data, role)` の純関数表示)。
- `docs/dev/exact-algebraic-equality-spec-proposal.md` — 代数領域での**値同値**を
  決定可能にする提案(本 Identity Model の「値同値」層の実装エンジン)。
- `docs/dev/ajisai-mathematical-formalization.md` §0・§8 — 外延的(conformance)
  同一性と内包的(`⟦·⟧`)同一性、観測関数 `observe`。
- `docs/dev/three-layer-documentation-model.md` — Reference / LOOKUP / hover の
  **ワードヘルプ内**三層。本書の「文書ロール」とは**軸が異なる**(後述 §3.4)。
- `README.md` の Documentation テーブル(Spec / Reference / Playground)。

---

## 0. 背景と本指示書の判断

ChatGPT との議論で「Ajisai Identity Model の受容」「文書ロールの明確化」
「Bubble sugar を `^` に一本化」という三方針が出た。リポジトリを精査した結果、
方針はいずれも妥当だが、以下の補正を加えたうえで実施する。

1. **`=>` は『調査対象』ではなく『確定した残骸』である。** 実測:
   - `SPECIFICATION.html` 内の `=>` 出現数は **0**。正準仕様は Bubble/NIL
     coalescing を `VENT`(sugar `^`)でのみ定義している(§7.14 周辺・サンプル)。
   - トークナイザ(`rust/src/tokenizer.rs:192`)は `'^' => Token::NilCoalesce`
     のみを生成する。`=>` を `NilCoalesce` 等に写像する規則は**存在しない**。
     `rust/src` 全体に `"=>"` の文字列リテラルは無い。
   - `canonicalize_core_word_name("^") == "VENT"`(`core_word_aliases.rs:137`,
     テスト `core_word_canonicalization_tests.rs:77`)。
   - `=>` が残るのは非正典 dev 文書群と、`rust/src/cli/plan_check.rs:59` の
     **コメント文**(コードは `^`/VENT のみ照合)だけ。

   したがって ChatGPT 観察の「`=>` / `^` は Bubble の recovery handler」という
   並記は `=>` の地位を過大評価している。`^`(VENT)が現行の唯一の sugar、
   `=>` は過去設計の残骸。本書では「最新実装で残骸であることを再確認したうえで
   除去する」(調査ではなく確認+整理)とする。

2. **Identity Model は語・構文を一切増やさない。** ユーザー要件「同値性を究極まで
   突き詰めつつ学習曲線を上げない/ユーザーは仕組みを意識せず享受する」を厳守する。
   新ワード(`EXACT_EQ` / `APPROX_EQ` 等)も新 surface 構文も**導入しない**
   (`exact-algebraic-equality-spec-proposal.md` §6・Decision 3 と整合)。Identity
   Model は**仕様内部の統合概念**+**Reference の概念ページ 1 枚**であり、既に
   §2.3 / §4.2.4–4.2.6 / §12 に散在する規定を 1 つの名前付き階層として束ね直す
   作業である。新しい意味論は足さない。

3. **スコープ規律。** ChatGPT が挙げた 16 の「発見/兆し」のうち本指示書で扱うのは
   Identity Model・文書ロール・`=>` 整理の 3 点のみ。その他(CONCAT 単一要素 top、
   空文字列 NIL、GET-as-lens、効果三射影 等)は §5 で明示的に範囲外とする。

---

## 作業項目 1 — Bubble sugar を `^`(VENT)へ一本化し、`=>` 残骸を整理する

### 1.1 確認(着手前ゲート)

実装担当はまず以下を再確認し、結果を PR 本文に記す:

- `rg -n '=>' rust/src` の結果が Rust の match アーム以外を含まないこと。
- `rg -n 'NilCoalesce' rust/src/tokenizer.rs` が `'^'` 由来のみであること。
- `SPECIFICATION.html` に `=>` が無いこと(`grep -c '=>' SPECIFICATION.html` → 0)。

この 3 点が成立する限り、`=>` は除去対象として確定。万一いずれかが崩れていた
(= `=>` が生きた構文だった)場合は、除去を中止しユーザーに確認すること。

### 1.2 残骸の除去・修正

正準・準正準・コードコメントの順に整理する。**コードの挙動は変えない**
(トークナイザは既に `^` のみ。除去対象は文書とコメントの表記)。

1. `rust/src/cli/plan_check.rs:59` の
   「`` `^` (VENT) and `=>` (OR-NIL) tokenize as `NilCoalesce` ``」コメントを、
   実態に合わせて「`^`(VENT)が `NilCoalesce` にトークナイズされる唯一の sugar。
   `OR-NIL` / `=>` は旧称・旧 sugar であり現行トークナイザは生成しない」へ修正。
   `OR-NIL` という旧名を残す場合も「歴史的別名」と明示する。
2. 非正典 dev 文書の `=>` を `^` に置換、または旧称である旨の注記を付す。対象:
   - `docs/dev/ajisai-mathematical-formalization.md` — §0 周辺の中置糖衣記述、
     §9.2 の NIL モナド則 `⊥_r => v ≡ v`(→ `⊥_r ^ v ≡ v`)、`A => B` 脱糖の節
     (行 88・214・224・291 付近)。数式の意味は不変、表記のみ `^` に統一。
   - `docs/dev/agent-cli-output-contract.md`(行 374・437 の `^`/`=>` 並記)。
   - `docs/dev/exact-algebraic-equality-spec-proposal.md`(行 228・296 の `=>`)。
   - `docs/dev/three-layer-documentation-model.md` 行 241 の hover 例
     `` | `OR-NIL` | `NIL => [ 0 ]` | `` を `` `NIL ^ [ 0 ]` `` に修正
     (語名 `OR-NIL` 自体の扱いは下記注意を参照)。
   - `docs/dev/ajisai-algebraic-simplification-rollout-plan.md` 行 26 の sugar 一覧
     から `=>` を削除。
   - `implementation-portability-evaluation-2026-06.md` / `ajisai-algebraic-
     simplification-review.md` の `=>` は**結果併記の矢印**(`TRUE => 1/1` 等)で
     あり Bubble sugar ではない。**置換しない**。誤爆させないこと。

   注意: 「`OR-NIL`」は VENT の旧 word 名の可能性が高い。`core_word_aliases.rs` /
   `builtin_word_definitions.rs` に `OR-NIL` が canonical/alias として**存在しない**
   ことを確認したうえで、文書中の `OR-NIL` も `VENT` に寄せる(歴史的記述として
   残す箇所は「旧称」と明記)。

### 1.3 回帰防止

`SPECIFICATION.html` と `SKILL.md` に `=>` を Bubble sugar として再導入しないこと。
可能なら CI / `scripts/` のリンタに「`SPECIFICATION.html` は `=>` を含まない」
不変条件を 1 行追加してよい(任意・低コストなら推奨)。

---

## 作業項目 2 — Ajisai Identity Model を正準仕様に明文化する

### 2.1 ねらい

「丸め誤差が無い言語」から一段進んで「**同一性の階層を明示的に扱う言語**」へ。
ただし**ユーザーには透明**であること。ユーザーは `EQ`(`=`)を書くだけで、
背後の階層は意識しない。階層は仕様の組織原理と Reference の概念ページとして
存在し、語数・構文・既定挙動は一切増えない。

### 2.2 同一性の階層(5 層)— 仕様への追加内容

`SPECIFICATION.html` に **新節「Identity Model(同一性モデル)」** を 1 つ設ける
(配置案: §2.x「Conformance and Identity」の直後、または §4.2.4 を見出しに格上げ)。
この節は新規定を作るのではなく、既存規定への**参照付き索引**として 5 層を定義する:

| 層 | 名称 | 定義 | 既存の正準根拠 |
|---|---|---|---|
| L1 | **値同値** (value equivalence) | 同一の数学的値か。標準的な意味での「等しい」。`EQ` 等が観測する対象。 | §4.2.4。代数領域では `exact-algebraic-equality-spec-proposal` により決定可能。 |
| L2 | **構造同一性** (structural identity) | 内部表現(Rational / Algebraic / Gosper、式 DAG)が同じか。**非観測**で、Coreword は分岐してはならない。 | §2.3, §4.2.2, §4.2.4 末尾(「representation tag is not part of value identity」)。 |
| L3 | **予算付き観測同値** (budgeted observational equivalence) | 比較予算内で順序/等値が確定するか。確定しなければ `UNKNOWN`。 | §4.2.5(NICF), §7.4.1。代数領域では total(予算不消費)。 |
| L4 | **表示同値** (display equivalence) | `render(data, role)` が同じ文字列を返すか。role 依存。 | §12.1–12.2。表示はデータと role の純関数。 |
| L5 | **conformance/外延同値** (conformance equivalence) | conformance suite の全標本で `observe` が一致するか=「実装として同じ Ajisai」。 | §「Conformance and Identity」, formalization §8 `observe`, `PORTABILITY.md`。 |

各層について明記すべき不変条件:

- **L1 ⇏ L2**: 値が等しくても構造は異なりうる(`√2`=`Algebraic{2:1}` vs Gosper 経由)。
  だから L2 は非観測であり、`EQ` は L1 を見る。
- **L2 ⇒ L1**: 構造が同一なら値も等しい(逆は不成立)。
- **L1 ⇒ L3 が代数領域では total**: 代数領域では観測予算を消費せず L1 を確定。
  L3 が `UNKNOWN` を返しうるのは将来の超越数領域(`Gosper`/`LazyCf`)のみ
  (§7.4.1 と exact-algebraic 提案 §2 の「正直な境界」を引用)。
- **L4 は L1 を含意しない**: 同じ表示でも role が違えば別物になりうる(逆も)。
  表示は機械判断の根拠にしてはならない(§4.5.0 の NIL 表示規定と同じ精神)。
- **L5 は L1–L4 の上位**: 観測関数 `observe`=`(render∘π_Stack, π_Eff)` の全標本
  一致。完全な同一性は `⟦·⟧` の等しさで、conformance はその有限標本(§8)。

### 2.3 透明性の保証(ユーザー要件の核)

新節に「**User-facing transparency**」小節を設け、次を明記する:

- 利用者が書くのは `EQ`/`NEQ`/`LT`… の 6 語(と sugar)だけ。Identity Model は
  これらの**意味の説明**であって、新たな操作ではない。
- 代数領域(現行 Coreword が構成できる全値)では比較は total かつ exact。利用者は
  「`√2 √2 SUB 0 EQ` が `TRUE`」を、予算や表現を意識せず得る。
- `UNKNOWN` / `^`(VENT)は将来の超越数領域のための正直な逃げ道であり、現行語彙
  では到達しない(到達したら明示シグナル+利用者選択の `^`)。
- L2(構造)・L4(role)は実装/表示の都合であって、利用者の値モデルには現れない。

### 2.4 範囲と分割(PR 単位)

- 本項目は **`SPECIFICATION.html` の節追加 + Reference 概念ページ**に限る。
  Rust/TS の挙動変更は伴わない(`exact-algebraic-equality-spec-proposal` の実装は
  別 PR・別系列。本節はその提案が前提とする同一性の枠組みを先に正準化する)。
- 既存 §4.2.4/§4.2.5/§7.4.1/§12 の本文は**書き換えず**、新節からの相互参照で束ねる
  (二重定義を作らない=`SPECIFICATION` の単一権威を維持)。
- `docs/formalization-coverage.json` に Identity Model の 5 層を観測項目として
  追加してよい(任意)。整合チェックは `scripts/check-formalization-coverage.mjs`。

---

## 作業項目 3 — 文書ロールと位置づけの明確化

### 3.1 ユーザーが定めたロール

| 文書 | 読者 | 役割 |
|---|---|---|
| **Specification**(`SPECIFICATION.html`) | **Ajisai を作りたい人**(実装者・移植者) | 言語を再実装できる単一設計権威。 |
| **Reference**(`public/docs/` 配下) | **Ajisai を使いたい人**(利用者) | 検証済み例・概念ガイド・ワードカタログ。Playground で開ける。 |
| **README** | 入口に来た全員 | Spec / Reference / Playground への**導線提供**(ハブ)。 |

ユーザーは現行の体裁を強く気に入っている。**構成は大きく変えない。**役割と
位置づけの明文化に限定する。

### 3.2 README(導線ハブ)

- 既存の Documentation テーブル(`README.md:17–22`)に **Audience 列**を追加し、
  「Specification = for those building/porting Ajisai」「Reference = for those
  using Ajisai」「Playground = run it now」を 1 語句で示す。テーブル構造・URL は
  維持。
- README は「導線」役に徹し、言語仕様の実体説明を増やさない(増えていれば
  Reference 側へ寄せる)。冒頭の Documentation 節が最初に来る現構成は維持。

### 3.3 Specification(実装者向け)冒頭の位置づけ宣言

`SPECIFICATION.html` の Specification Authority / 導入部に 1 文を追加:
「This document is written for those implementing or porting Ajisai; users who
want to *use* the language should start from the Reference and Playground.」
既存の権威宣言と矛盾しない範囲で、読者像を明示するのみ。

### 3.4 Reference(利用者向け)位置づけ宣言と三層モデルとの整理

- `public/docs/index.html`(現状スタブ)または Reference トップに「Reference は
  Ajisai を**使う**ための文書。言語を**作る**ための単一権威は Specification」と
  明記。
- **重要な非衝突メモ**: 本項目の「文書ロール」(Spec / Reference / README の 3
  文書)は、`three-layer-documentation-model.md` の「Reference / LOOKUP / hover」
  (ワードヘルプ**面**の 3 層)とは**直交する別軸**。両者を混同しないよう、
  `three-layer-documentation-model.md` 冒頭に「これはワードヘルプ面の階層であり、
  文書間ロール(Spec/Reference/README)とは別軸」の 1 行を追加する。
- `reference-writing-style.md` の位置づけ節は既に「Reference は仕様から派生」と
  述べており整合。変更不要(必要なら相互リンクのみ)。

---

## 5. 非目標(本指示書では扱わない)

ChatGPT 観察のうち以下は**範囲外**。Codex は触れないこと(別途、独立指示で扱う):

- `CONCAT` の単一要素ベクタ top の underflow 挙動(`string_laws.rs` で固定)。
- 空文字列が `NIL` になる pointed-monoid 的挙動。
- `GET` の lens / store comonad 的再定式化。
- 効果系の三射影(Outbound / InboundObservation / InternalStateDelta)分解。
- 子ランタイム並行性、修飾子の線形資源型、テンソルの添字函手化、GUI の LTS 化。

これらは数学的に興味深いが、本指示書のテーマ(同一性の明文化・文書ロール・`=>`
整理)とは独立で、学習曲線・正準仕様への影響が大きいため切り離す。

---

## 6. 完了条件(Definition of Done)

1. `grep -c '=>' SPECIFICATION.html` が 0 のまま。Bubble sugar の正準・準正準表記が
   `^`(VENT)に統一され、結果併記の `=>` は誤爆していない。`plan_check.rs` の
   コメントが実態(`^` のみがトークナイズされる)に一致。
2. `SPECIFICATION.html` に Identity Model 節が追加され、L1–L5 が既存節への相互参照
   として定義され、User-facing transparency 小節を含む。新ワード・新構文・既定
   挙動の変更が**無い**こと。Reference に対応する概念ページ 1 枚。
3. README に Audience が示され、Spec/Reference の冒頭に読者像宣言。
   `three-layer-documentation-model.md` に軸の違いを示す 1 行。
4. 既存テスト(`rust/tests/*_laws.rs` ほか)と `scripts/check-formalization-
   coverage.mjs` が緑。挙動不変のため新規法則テストは原則不要(Identity Model の
   不変条件を法則化したい場合は `observation_laws.rs` への追加に留める)。
