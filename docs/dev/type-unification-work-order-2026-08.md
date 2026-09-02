# CodeBlock/Vector 統合（改修Ⅰ）：改修指示書（2026-08）

Status: 非正典・`[設計根拠]`。本書は Ajisai の意味論も互換性方針も定義しない。
正典は `spec/` 配下の各ソースと、そこから生成される `SPECIFICATION.html` のみ。

前提文書：`docs/dev/ajisai-single-axis-proposal-2026-08.md`（改修Ⅰの原提案。§3「改修 Ⅰ」と
§6「課題」を必読）、`docs/dev/vector-nesting-role-redefinition.md`（Lisp 的動機を設計根拠として
用いないという既存の裁定。本書の作業はこの裁定を覆すものではない——根拠は §2 参照）。

---

## 0. この文書の読み方（実装者向け・最初に必ず読む）

### 0.1 これは他の改修（Ⅱ〜Ⅶ）と種類が違う

Ⅱ・Ⅲ・Ⅴ・Ⅵ・Ⅶ は実装され、`main` にマージ済みである（PR #1563/#1564/#1567）。
それらはいずれも**観測されるプログラムの振る舞いを変えない**か、**新しい語を足すだけ**の
変更だった。改修Ⅰは違う。

- `[ FOO ]` は現在 String `'FOO'` を表す（`LANG.VALUES.VECTOR`）。統合後は違う何かを表す。
- `{ 1 2 + } [ 1 2 + ] EQ` は現在 `FALSE`（あるいは型エラー）。統合後は `TRUE` になる。
- `EXEC` が `Vector` を拒否する・`nonCodeBlock` 系のエラーを返す、という**現在通っている
  conformance ケースが複数、意味を失う**（§2.3）。

これは**観測可能な意味論を変える変更**であり、README が明示する互換性方針
（「betaで書かれたプログラム・保存セッション・書き出した辞書は、betaで読める。現在の形式は
一つだけで、旧形式からの変換器は無い」）の対象そのものである。**したがって本書の作業は、
コードを書く前に、この互換性方針をどう扱うか——0.2.0-beta.1 の中で吸収するのか、
新しいベータ／リリース段階の開始として扱うのか——を利用者（正典の所有者）に確認する
ところから始まる。** これは実装者が現場判断で決めてよい事項ではない。

### 0.2 本書の構成

Phase 0 は**読み取り専用の調査**であり、正典・実装のどちらにも触れない。承認なしに
着手してよい。Phase 1 は**測定を伴うスパイク**で、使い捨てブランチ上で行い、
`main` へは絶対にマージしない。Phase 2（実装）は、Phase 0・1 の結果を人間が見て
「進める」と判断した場合にのみ、別途指示を得て着手する。**本書は Phase 2 の具体的な
実装手順を規定しない**——Phase 1 の結果次第で設計そのものが変わりうるため、
先に手順を固定することは無意味である。

### 0.3 禁止事項

- Phase 0・1 の間、`spec/`・`SPECIFICATION.html`・`rust/src` の**正典/実装ファイルを
  一切変更しない**。この段階の成果物は調査報告であって、コード変更ではない。
- Phase 1 のスパイクブランチを `main` にマージしない。Phase 1 は測定のためのブランチであり、
  そのまま出荷可能な変更ではない。
- 「`vector-nesting-role-redefinition.md` が Lisp 的動機を禁じている」ことを理由に、
  この作業そのものを却下しない。原提案 §3-Ⅰ が既に整理した通り、本書の根拠は
  水平方向の設計美学（Lisp への憧れ）ではなく、単一の軸（絞り込み）からの導出である。
  ただし §2 に記す通り、その根拠づけには未解決の技術的重みがある。

### 0.4 停止条件（詰まったら黙って続行せず、ここで止めて報告する）

- Phase 0 の結果、統合の技術的コストが §2 の見積もりより著しく大きい、または
  小さいと分かった場合。見積もりの前提が崩れたら、Phase 1 のスコープも人間の
  再確認を要する。
- Phase 1 のスパイク中に、conformance corpus の非通過件数が「小さい修正で吸収できる」
  範囲を超えた場合（目安：現行 279 件中の一桁台を超える場合）。
- Phase 1 のスパイク中に、`word_contract.rs`（契約推論エンジン、`PROBE` の基盤）の
  書き直し規模が「増分」ではなく「作り直し」になると判明した場合（§2.4）。
- 互換性方針の扱い（§0.1）について、利用者からまだ明示の回答を得ていない状態で
  Phase 2 に進めと言われた場合。

### 0.5 よく使う検証コマンド

```
# Rust（作業ディレクトリは rust/）
cargo build --bin ajisai
cargo test --lib --tests
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# conformance corpus
cargo test --lib conformance_suite_passes -- --nocapture

# リポジトリ全体のゲート（ルートで。フルセットは30ステップ、
# MCP アセット同期を忘れると Quality Gate が赤くなる——PR #1563 の教訓）
npm run check:semantic-kernel
npm run check:reading-surfaces
cd tools/mcp-server && node sync-assets.js
```

---

## 1. 目的

`{ }`（CodeBlock）と `[ ]`（Vector）を、同一の値領域に対する二つの綴りにする。
実行可能な記述（コード）とデータが今は互いに素な二領域だが、これを一つにし、
`REFLECT` という「橋」を不要にする——`REFLECT` が存在すること自体が、
領域を分けたことの代償だったという見立てである。詳細な動機は原提案 §2.1（軸の説明）と
§3-Ⅰ を参照。ここでは繰り返さない。

## 2. Phase 0 — 前提の再検証（読み取り専用）

**このセッションの教訓。** 改修Ⅵ・Ⅲ・Ⅱの実装では、事前の設計メモが書いた前提が
**実装に着手した時点でことごとく部分的に誤っていた**（`stack.inputs` がフレーム引数と
別物だった、`capability`/`hostedEffect` が実は MCP 経由で外部に転送されていた、
`errorWhen` の汎用到達可能性チェックが機能しなかった、等）。原提案の改修Ⅰの記述も
**検証されていない**。以下の各項目を、コードを読み実行して再確認してから Phase 1 に進むこと。

### 2.1 内部表現が異なるという事実（原提案が触れていない技術的負債）

実測済み：

```
rust/src/types/mod.rs:77   Vector(Arc<Vec<Value>>)   -- 評価済みの Value の列
rust/src/types/mod.rs:83   CodeBlock(Vec<Token>)      -- 未評価の Token の列
```

`Vector` は**既に評価済みの値**（`Fraction`・`Value::Text` 等）を保持する。`CodeBlock` は
**評価されていない字句**（数値なら元のレクセム文字列、シンボルなら名前）を保持する。
`REFLECT` の `tokens_to_code_data`/`code_data_to_tokens`（`rust/src/types/code_data.rs`）は、
まさにこの二つの表現を相互変換する関数であり、**今回の統合が本質的に何をする作業なのかを
最も正確に示すコード**である。

これは原提案 §3-Ⅰ が「符号化されて隠れていた概念（Symbol）を昇格させ、符号化を消す」と
書いた内容と整合するが、原提案はこれを**表現の綴りの問題**であるかのように軽く書いている。
実際には、統合は次のいずれかを要求する。

- **(A) Vector 側の表現に寄せる**：CodeBlock を Vector 化する。`Value` に新しい variant
  `Symbol(String)` を追加し（`REFLECT` の中間表現に既に `[ 'symbol' 'PRINT' ]` として
  存在する概念を昇格させるだけ、という原提案の主張はここでは正しい）、実行エンジンは
  「Vector 形の値を、必要に応じて命令列として解釈する」経路を新設する。
- **(B) CodeBlock 側の表現に寄せる**：Vector を CodeBlock 化する。`[ 1 2 3 ]` の要素が
  未評価の `Token` になる。GET・LENGTH・MAP 等**全てのベクトル操作**が、評価済み値ではなく
  未評価トークンの列を相手にすることになり、影響範囲が (A) より広い。**この案は採らない**
  ことを Phase 0 の結論として明記する（Phase 1 で再検討する場合は理由を書くこと）。

Phase 0 で確認すること：(A) を前提として、`Value` に `Symbol` variant を足した場合に
影響を受けるパターンマッチ箇所の総数を数える（`match value.data` / `match &value.data` の
全箇所を `rust/src` で検索し、`ValueData` の全variantを網羅する形で書かれているものを数える。
`#[non_exhaustive]` でなければ、コンパイラがこの一覧を機械的に提供する——`Value::Symbol`
を追加してビルドし、`cargo build --lib 2>&1 | grep "non-exhaustive"` の件数がそのまま
影響ファイル数の下限になる)。

### 2.2 `word_contract.rs`（`PROBE` の基盤）は Token に直接依存している

実測済み：

```
rust/src/interpreter/word_contract.rs:343   Token::Number(_) | Token::String(_) => ...
rust/src/interpreter/word_contract.rs:353   Token::Symbol(_) if scope.in_vector_literal() => ...
rust/src/interpreter/word_contract.rs:358   Token::Symbol(symbol) => ...
```

`infer_word_contract_inner`（改修Ⅱで `PROBE` の基盤にした関数）は `Token` を直接
パターンマッチしている。§2.1 の (A) 案を取り、`CodeBlock` の内部表現が `Vec<Token>` で
なくなった場合、この関数と、それが呼ぶ `word_contract_flow.rs`／`word_space.rs`／
`word_cost.rs`／`word_contract_widen.rs` の `Token` 依存箇所も同時に書き換えが要る。
これは**改修Ⅱでこのセッションが書いたばかりのコード**であり、改修Ⅰのコストを
過小評価しないための具体例としてここに記録する。

### 2.3 conformance corpus のうち何件が Vector/CodeBlock の非交差性を主張しているか

Phase 0 の粗い実測（`tests/conformance/index.html` を `never executable`・`never code`・
`nonCodeBlock` 等の語で検索）では 9 件がヒットしたが、この数字はキーワード一致による
概算であり検証していない。Phase 0 でやること：`data-category="core"` の全ケースを読み、
「CodeBlock と Vector が別領域であることを前提に、その前提の**成立**または**破綻**を
主張しているケース」を人手で数え上げ、一覧を Phase 0 の報告に含める。少なくとも
`core-exec-rejects-vector`（「A Vector is data and is never executable」というコメント付き）
は該当する。

### 2.4 `LANG.VALUES.DISJOINT` ほか、書き換えが要る正典節の一覧

現行の正典は次を明示的に規定している。書き換えが要る箇所の一覧を Phase 0 の報告に含める
（本書では網羅しない——正典の書き換え文言そのものは Phase 2 の仕事であり、Phase 0 は
「どの節に触れる必要があるか」を数え上げるだけでよい）。

- `LANG.VALUES.DISJOINT`：「六つの互いに素な領域」という数——統合後は五つになる。
- `LANG.VALUES.VECTOR`：「`[ FOO ]` は `FOO` が定義済みの Word かどうかに関わらず
  String `FOO` を表す」という規定——統合後、裸のシンボルの意味が変わる可能性がある。
- `LANG.SOURCE.CODE`：CodeBlock の定義そのもの。
- `LANG.SOURCE.REFLECTION`：`REFLECT` の節。`REFLECT` を消す場合はこの節ごと削除、
  Core Word 数は 66 から 65 に戻る。
- `LANG.SOURCE.FRAME`：改修Ⅵで書き換えたばかりの節。ブロックの評価規則が
  「Vector 形の値を、それが書かれた文脈に応じて命令列として解釈するかどうか」という
  規則に一般化されるなら、ここも影響を受ける。

### 2.5 Phase 0 の成果物

上記 2.1〜2.4 の実測結果を一つの報告（本書と同じ `docs/dev/` に置く別ファイル、
または本書の追記セクション）にまとめ、**コードを一切変更せずに**人間に提示する。
報告には最低限、次を含めること。

- `Value::Symbol` 追加でビルドが壊れる箇所の件数（§2.1 の下限値）。
- Vector/CodeBlock 非交差性を主張する conformance ケースの確定件数と一覧（§2.3）。
- 書き換えが要る正典節の一覧（§2.4）。
- これらを踏まえた、Phase 1 スパイクの推定作業量（人日換算は不要。「半日で見通しが
  立つ規模」「複数日かかる規模」のどちらかを明記すれば足りる）。

Phase 0 の報告を人間に提示し、Phase 1 に進めという明示の指示を得てから次に進むこと。

## 3. Phase 1 — スパイク（測定のみ、`main` にマージしない）

Phase 0 の承認後、使い捨てブランチ（例：`spike/codeblock-vector-unification`）を切り、
§2.1 の (A) 案で実際に手を動かす。**目的は動く実装を作ることではなく、
「この変更が壊すものの全体像」を実測で確定させることである。**

### 3.1 やること

1. `Value` に `Symbol(String)` variant を追加する。
2. `CodeBlock(Vec<Token>)` を `CodeBlock` 削除・Vector 統合、または
   `ValueData::Vector` を実行可能とみなす経路の追加、のどちらかで実装する
   （どちらが小さい差分になるかは Phase 0 の結果次第。両方試す必要はない——
   小さそうな方から着手し、詰まったら §0.4 の停止条件に従う）。
3. `REFLECT` を削除する（恒等写像になるはずなので、削除して conformance の
   `reflection-*` ケース群がどう扱われるべきかを記録する。削除するか、
   「もう REFLECT は存在しない」ことを確認するテストとして残すかは Phase 1 の判断でよい）。
4. `cargo build --lib` を通し、破壊された箇所を機械的に洗い出す（§2.1 の見積もりと
   実測を突き合わせる）。
5. `cargo test --lib conformance_suite_passes` を実行し、**非通過件数**を記録する
   （通す必要はない。数えるだけでよい）。
6. `public/docs/index.html`（Reference）の該当箇所（`blocks-frames` セクション等）を
   目視で読み、`{ }` と `[ ]` が同じ値を表すようになった場合に**読みにくくなる例が
   実際にあるか**を判定する。これは原提案 §6 が「本書はこの点について結論を持たない」
   と明記した未決事項であり、Phase 1 で初めて実例を持って判断できる。

### 3.2 Phase 1 の成果物

- 実際の差分（コミットはスパイクブランチに残す。`main` へは絶対にプッシュしない）。
- conformance 非通過件数と、各失敗が「表記の更新で直る」か「意味論そのものの
  再設計が要る」かの分類。
- Reference の可読性についての判定（3.1-6）。
- Phase 0 の見積もりと実測の差分。見積もりが外れていた場合はその理由。
- 総括：この変更を Phase 2 として実装する価値があるか、実装するとして
  どの範囲に縮小すべきか、についての推奨（決定ではなく推奨。決定は人間が行う）。

Phase 1 の成果物一式を人間に提示し、Phase 2 に進めという明示の指示——および
§0.1 の互換性方針についての回答——を得てから初めて Phase 2 に着手する。

## 4. Phase 2 — 実装

本書は Phase 2 の手順を規定しない（§0.2）。Phase 0・1 の結果を踏まえて別途指示する。

## 5. 採らない案（原提案からの引き継ぎ、再確認）

- 「Ajisai 2」として作り直す、変換器で移行する：既に却下済み。
  本書の統合作業も、既存の正典・conformance corpus・契約レジストリという
  **同じ資産の上で**行う。第二の権威を作らない。
- `UNKNOWN`・三値論理の復活：この統合作業と無関係。混同しないこと。
- Lisp への憧れを設計根拠にする：`vector-nesting-role-redefinition.md` の裁定は覆さない。
  本書の根拠は §2.1 に記した技術的事実と、原提案の軸（絞り込み）である。

## 6. 課題（本書執筆時点で未解決）

- §2.1 の (A)/(B) 判定は Phase 0 の実測前の暫定判断であり、Phase 0 の結果次第で
  再考しうる。
- `{ }` と `[ ]` という**綴りを二つ残す**ことの意味論上の扱い（構文糖として同値か、
  それとも読者向けの注記に過ぎず言語上は完全に同じトークンとして扱うか）は未決定。
- 本書は Phase 0・1 の作業量を見積もっていない。着手前に、この作業に何日〜何週間
  かけてよいかを人間に確認すること。
