# コスト契約の発見可能性：改修指示書（2026-08）

Status: 非正典・`[設計根拠]`。本書は Ajisai の意味論も互換性方針も定義しない。
正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。

前提文書：`docs/dev/cost-contract-design.md`（コスト軸の設計根拠）、
`docs/dev/competitive-advantage-work-order-2026-08.md`（Phase 5 でコスト軸を導入した指示書）。

---

## 0. この文書の読み方（実装者向け・最初に必ず読む）

### 0.1 ⚠️ 起点の訂正：3項目のうち2項目は既に実装済み

本書は当初「コスト軸まわりの最適化3項目」として起案された。しかし着手前の実測で、
**3項目のうち2項目は既にマージ済み**であることが判明した。指示書として再実装を
命じると二重作業になるため、ここで確定させる。

| 当初の項目 | 現状 | 根拠（`origin/main` で確認済み） |
| --- | --- | --- |
| `DepCost::of_builtin` の重複ルックアップ除去 | **実装済み** | `word_cost.rs` の `DepCost::of(contract, operand_driven)` が `contract.cost` を読む。`builtin_cost_for` の再呼び出しは無い |
| provenance 追跡の一本化（`SpaceSim`/`CostSim` の二重実装解消） | **実装済み** | `word_space::OperandProfile` を `feed_word` が返し、`CostSim::feed_word` がそれを受け取る |
| `ajisai contract` が cost を報告しない | **未対応** | `contract_report.rs` に `cost` の出現は 0 件（`space` は 13 件） |

したがって**本書が指示するのは 3 項目めだけ**である。Phase は 1 つしかない。
1・2 項目めに手を入れてはならない（現状が正しい）。

### 0.2 禁止事項

- `SPECIFICATION.html` を手で編集しない（`spec/` から生成する）。
- `word_cost.rs` の分類表・`refine_axis` の規律を変更しない。本書の作業は
  **推論結果の提示**だけであり、推論そのものは対象外。
- 「実測していない効果」をコメント・コミットメッセージに書かない。
- exact でない軸を `suggested` に出さない（§1.4 落とし穴 B）。

### 0.3 停止条件（詰まったら黙って続行せず、ここで止めて報告する）

- `suggested` の出力が `ajisai check --contract` を通らない形になった場合。
  `suggested` は paste-ready であることが唯一の存在意義であり、通らないなら設計が誤っている。
- 端末出力の1行フォーマットが破綻し、既存の見た目を大きく変えないと収まらない場合。

### 0.4 よく使う検証コマンド

```
# Rust（作業ディレクトリは rust/）
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests

# CLI をビルドして実際の出力を見る
cargo build --bin ajisai
./target/debug/ajisai contract <file.ajisai> --json

# リポジトリ全体のゲート（ルートで）
npm run check:file-size
npm run check:agent-cli-contract
npm run test:mcp
```

### 0.5 用語

- **軸（axis）** — `steps` / `numeric` / `collection` の3つ。`ResourceUsage` の3カウンタに対応。
- **クラス（class）** — `const` < `linear` < `superlinear` < `unbounded`。
- **exact（厳密性の証拠）** — そのクラスが**実際に到達される**と証明できている印。
  宣言検査で `note` ではなく `error` を出す許可証。軸ごとに独立に持つ。

---

## Phase 1 — 推論されたコストを `ajisai contract` に出す

### 1.1 目的

コスト軸は宣言できるが、**利用者が何を宣言すべきか知る手段が無い**。

`ajisai contract` は各ユーザー語の推論契約を報告し、貼り付け可能な `#:contract`
行（`suggested`）を添えることで「報告 → 宣言 → `check --contract`」の輪を閉じる、
というのがこのコマンドの設計意図である（`contract_report.rs` のモジュールコメント）。
空間計算量はこの輪に入っている：

```
LITERAL-ADD : ( 0 -- 1 ) pure nil-propagating deterministic space:const [complete]
    #:contract LITERAL-ADD ( 0 -- 1 ) pure nil-free space:const
```

しかしコストは輪の外にある。`contract_report.rs` は `cost` を一切扱わないため、
推論器が `steps=const numeric=const collection=unbounded` を導いていても、
それが利用者に見えることは無い。宣言構文だけが存在して発見手段が無い状態であり、
Phase 5 の作業が実質的に未完であることを意味する。

**これは不具合ではない**（誤った値を出しているわけではない）。欠落である。

### 1.2 触ってよいファイル（ホワイトリスト）

- `rust/src/agent/contract_report.rs` — 本体
- `rust/src/cli/mod.rs` — 端末出力（1箇所）
- `rust/src/agent/contract_report_tests.rs` — 新規（存在しなければ作る。`agent/mod.rs` に `#[cfg(test)]` で登録）
- `rust/src/agent/mod.rs` — 上記テストモジュールの登録のみ
- `docs/dev/agent-cli-output-contract.md` — `contract` 節の記述
- `docs/dev/INDEX.md` — 本書の索引行

これ以外に触れる必要が出たら §0.3 の停止条件に該当する。

### 1.3 事前に読むファイル（コードを書く前に必ず）

- `rust/src/agent/contract_report.rs` 全体（157行）。特に `suggested_directive` の
  `space_exact` ゲート（93〜98行のコメント）。この規律をコストにも適用する。
- `rust/src/interpreter/word_cost.rs` の `CostClass` / `CostBound` / `refine_axis`。
- `rust/src/agent/contract_cost.rs` の `parse_cost_terms`。**`suggested` が生成する
  文字列は、この関数が受理する形でなければならない。**
- `docs/dev/cost-contract-design.md` §3（exact の意味）。

### 1.4 ⚠️ 落とし穴（実装前に必ず読む）

#### 落とし穴 A：cost は3軸なので `space` と同じ形に収まらない

`WordReport::space` は `&'static str` 一本である。コストは軸ごとにクラスが違うため
同じ形にできない。`WordReport` には3軸を保持するフィールドを持たせること
（`cost_steps` / `cost_numeric` / `cost_collection` の3つ、または小さな構造体1つ）。
「代表値1つに畳む」ことはしてはならない — 軸が独立であることがコスト軸の要点であり、
畳んだ瞬間に `numeric` と `collection` の区別という設計の中心が消える。

#### 落とし穴 B：exact でない軸を `suggested` に出してはならない

`suggested_directive` は空間クラスについて既にこの規律を持っている：

> Only codify the space class when the inference *proves* the bound is attained;
> an unproven upper bound would only ever check as a note, so suggesting it would
> invite a declaration weaker than the checker verifies.

コストは exact を**軸ごとに**持つので、**軸ごとに**この判定をする。
`contract.cost.steps.1` が真の軸だけを出す。語全体の `confidence` で判定してはならない
（`docs/dev/cost-contract-design.md` §3 の通り、両者は独立である）。

#### 落とし穴 C：exact な軸が1つも無いとき、`cost` を裸で出すと**パースエラーになる**

`#:contract` の文法は `cost` キーワードに続けて1つ以上の `axis=class` 項を要求する。
`contract_cost::parse_cost_terms` は項が0個のとき
`` `#:contract NAME`: `cost` with no `axis=class` term `` を返す。

つまり「exact な軸が無い語」に対して素朴に `parts.push("cost")` してから軸を足す実装を
書くと、**`suggested` 行そのものが `check --contract` を通らなくなる**。
`suggested` は貼り付けて動くことが唯一の存在意義なので、これは致命的である。

**exact な軸が1つ以上あるときだけ `cost` キーワードごと出す。** テストで固定すること
（§1.5 Step 1.4 の3件目）。

#### 落とし穴 D：`Const` の軸は常に exact になる

`refine_axis` は `CostClass::Const` に対して無条件で `(Const, true)` を返す。
これは正しい（Const は格子の底なので、これより下は無く、宣言不一致を生じ得ない）。
結果として `{ 1 2 ADD }` のような語は3軸すべてが exact になり、
`suggested` は `cost steps=const numeric=const collection=const` を含む長い行になる。

これは**冗長だが正しい**。「Const は自明だから省く」最適化をしてはならない —
「この語は安い」は利用者が最も欲しい保証であり、かつ検査器が実際に検証できる主張である。
不要なら利用者が消せばよい。

#### 落とし穴 E：MCP の公開スキーマは壊れない（確認済み・安心してよい）

`tools/mcp-server/result.schema.json` の `contracts` は
`{"type":"array","items":{"type":"object"}}` であり `additionalProperties` の制約が無い。
配列要素にフィールドを足してもスキーマ検証は落ちない。`npm run test:mcp` の
「infer_contracts satisfies the published result schema」も通る。
**スキーマを編集する必要は無い**（触ると §1.2 のホワイトリスト違反になる）。

### 1.5 手順

#### Step 1.1 — `WordReport` に3軸を持たせる

`contract_report.rs` に `cost_label(CostClass) -> &'static str` を追加する
（`"const"` / `"linear"` / `"superlinear"` / `"unbounded"`)。

**接頭辞を付けない。** `space_label` は `"space:const"` を返すが、コストは
JSON でも端末でも軸名の下に置かれるため接頭辞が冗長になる。加えて、
宣言文法の語彙（`cost steps=const`）とそのまま一致する方が、報告と宣言の対応が
目で追える。

`WordReport` に3フィールドを足し、`report_contracts` で `contract.cost` から埋める。

#### Step 1.2 — JSON に `cost` を出す

`reports_json` に軸をキーとするオブジェクトを1つ足す：

```json
"cost": { "steps": "const", "numeric": "linear", "collection": "unbounded" }
```

**exact は JSON に出さない。** 既存の `space` も `space_exact` を出しておらず
（`reports_json` を参照）、exact は `suggested` に載るか否かという形でのみ観測される。
この非対称は意図的である：報告は「推論が何を導いたか」、`suggested` は
「検査器が何を検証できるか」であり、後者だけが exact に依存する。
`space` と同じ規律をコストにも適用する。

#### Step 1.3 — `suggested_directive` にコストを足す

`space_exact` のブロックの直後に置く。落とし穴 B・C を守ること：

- exact な軸を集める。
- 集まった軸が**0個なら何も足さない**。
- 1個以上なら `cost` に続けて `axis=class` を **steps → numeric → collection の順**で足す
  （順序を固定しないと出力が非決定的になり、差分が読めなくなる）。

#### Step 1.4 — テスト（この Phase の本体）

`rust/src/agent/contract_report_tests.rs` を新規作成し、最低5件：

| テスト | 内容 |
| --- | --- |
| `cost_axes_are_reported_per_axis` | `{ RANGE }` の JSON `cost` が `collection: "unbounded"` を含む |
| `suggested_includes_only_exact_axes` | inexact な軸が `suggested` に現れない |
| `suggested_omits_the_cost_keyword_when_no_axis_is_exact` | 落とし穴 C。`suggested` に `"cost"` が現れないこと |
| `suggested_round_trips_through_the_checker` | **最重要。** `report_contracts` が出した `suggested` 行を元ソースに連結し、`check --contract` にかけて `outcome == "value"`（違反0・note0）を確認する |
| `axis_order_is_stable` | 同一ソースを2回報告して文字列一致 |

4件目は他の4件より価値が高い。`suggested` が「貼り付けて通る」ことを機械で保証する
唯一のテストであり、落とし穴 B・C・D をまとめて捕まえる。**これを省略しないこと。**

#### Step 1.5 — 端末出力

`cli/mod.rs` の1行フォーマットは既に6項目あり、3軸を横に足すと破綻する。
既存の `effects` と同じ継続行の形にする：

```
    cost: steps=const numeric=linear collection=unbounded
```

3軸すべてが `const` の場合も省略しない（落とし穴 D と同じ理由）。

#### Step 1.6 — ドキュメント

`docs/dev/agent-cli-output-contract.md` の `## contract` 節（434行付近）は現在
「space class」までしか列挙していない。コスト3軸を加え、**exact な軸だけが
`suggested` に載る**という規律を1文で書く。

### 1.6 受け入れ条件（すべて満たすこと）

- `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` が無警告。
- `cargo test --lib --tests` が全通過（新規5件を含む）。
- `npm run check:file-size`（`contract_report.rs` は現在157行なので余裕がある）。
- `npm run check:agent-cli-contract` と `npm run test:mcp` が通過。
- `ajisai contract` の実出力を目視し、`suggested` 行を実際に貼り付けて
  `ajisai check --contract` が exit 0 で通ることを**手でも1回確認**する。

### 1.7 コミット

件名は「何を可能にしたか」で書く（例：`Report the inferred cost bound in ajisai contract`）。
本文には次を含める：

- Phase 5 でコスト軸を導入したが報告面が欠落しており、宣言構文だけがあって
  発見手段が無い状態だったこと。
- exact な軸だけを `suggested` に載せる規律と、その理由（検査器が検証できない宣言を
  勧めない）。
- 軸が0個のとき `cost` キーワードごと省く必要があること（さもなくば `suggested` 自体が
  パースエラーになる）。

---

## 付録 A — SHA-256 を BLAKE3 に置換する案の評価：**採用しない**

### A.1 前提の訂正

> Ajisai に採用されているハッシュ技術は SHA-256 だと思いますが

実測の結果、**この前提は成り立たない**。

- **Rust コア（言語処理系・エージェント境界）は既に全面 BLAKE3。**
  `rust/Cargo.toml` は `blake3 = { version = "1", ... }` を宣言し、
  `Cargo.lock` の解決済みバージョンは **1.8.5**。
  語の同一性（`word_identity.rs`）も観測ダイジェスト（`observation_digest.rs`）も BLAKE3 である。
- **Rust 側に SHA-256 は1箇所も無い**（`sha2` クレートへの依存も無い）。
- SHA-256 が使われているのは **Node 側の `tools/mcp-server/` 内の4箇所のみ**。

### A.2 その4箇所の実態

| 箇所 | 用途 | 突き合わせ相手 |
| --- | --- | --- |
| `capture-traces.js:54` | トレース記録の短縮ID（先頭16桁） | 無し（同一プロセス内の同定のみ） |
| `capture-repairs.js:36` | 同上 | 無し |
| `sync-assets.js:19` | 同梱レジストリの `registryDigest` を**書く** | `index.js` |
| `index.js:300` | 同梱レジストリの digest を**照合する** | `sync-assets.js` |

後半2つは「JS が書いて JS が検証する」自己完結した破損検知であり、
**Rust 側に対応する digest は存在しない**（`registryDigest` 相当の実装は Rust に0件）。
前半2つは短縮IDで、暗号学的性質を要求していない。

### A.3 置換しない理由

1. **シナジーが発生する接点が存在しない。** シナジーが生まれるのは
   「Rust が作った digest を JS が独立に再計算して検証する」場合だが、その要件は現在無い。
   4箇所はいずれも言語をまたがない。
2. **依存が増える。** Node には BLAKE3 が組み込まれていない。現在の4箇所は
   標準の `crypto.createHash` で**依存ゼロ**である。`ajisai-mcp-server` は npm に公開する
   パッケージで、実行時依存は現在2つしかない。内部識別子のために
   サプライチェーン依存（多くは wasm もしくはネイティブビルドを伴う）を足すのは割に合わない。
3. **`1.0.0` への固定は後退になる。** 現行の解決版は 1.8.5 であり、
   1.0.0 を指定すると Rust 側が8マイナーバージョン分の後退となる。
   BLAKE3 のバージョンを動かす積極的な理由は見当たらない。

### A.4 再検討の条件（この条件が満たされたら A.3 は無効になる）

**JS が Rust の作った digest を独立に検証する要件が生まれたとき。**
例えば観測ダイジェストを MCP アダプタ側で再計算して突き合わせたい、という要求である。

ただしその場合でも、ハッシュ関数の選択は問題の**小さい方**であることに注意する。
観測ダイジェストは `AJISAI-OBS-1` のバイト文法（値の符号化・代数的数の包囲キー・
辞書の正規化順序）の上に載っており、JS で再現すべき本体はそちらである。
ハッシュを揃えるのは、その文法を移植し終えた後の最後の1行に過ぎない。
「まずハッシュを統一する」順序で着手すると、労力の大半を占める部分が手つかずのまま残る。
