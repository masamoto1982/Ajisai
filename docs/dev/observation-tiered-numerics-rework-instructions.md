# 改修指示書: Observation 一元化と三層厳密数値コア (Tier 0/1/2)

Status: 実装指示書（この文書自体は non-canonical。正典は `SPECIFICATION.html` のみ）
Audience: 実装担当セッション（以下「実装者」）
Owner intent: 連分数 (CF) を万能内部表現とする現行設計を廃し、単一の観測
インターフェースの背後に三層の厳密数値表現を置く。**観測可能な値の意味は
原則不変**、表現だけを差し替える。

---

## 0. 一段落サマリ

Ajisai の数値は現在 `Fraction`（任意精度有理数）と `ExactReal`
（連分数ベース: `AlgebraicSqrt` + `Gosper` 変換, `rust/src/types/continued_fraction.rs`,
約 5,150 行）の二本立てである。本改修では、数値を
**「水 (budget) を与えると有理区間を精緻化して返す観測過程 (Observation)」**
として一元化し、その背後に

- **Tier 0: 有理数 ℚ**（既存 `Fraction` をそのまま採用。常に即確定）
- **Tier 1: 代数的数**（√ と体演算の閉包。比較が**常に決定可能**）
- **Tier 2: 一般計算可能実数**（縮小有理区間の遅延精緻化。今回は骨組みのみ）

を置く。現行語彙が到達できる無理数は「有理数の √ と体演算の閉包」だけであり
（超越数を生む語は存在しない）、これは Tier 1 で完全に覆える。したがって
**改修後、現行語彙の比較から `UNKNOWN` は発生しなくなり**、`UNKNOWN` は
Tier 2（将来の π, e, log 等）専用の正当な出力として温存される。
Gosper 算術と CF 内部表現は退役させる。

## 1. ゴール / 非ゴール

### ゴール
1. `ValueData::ExactScalar` の背後の表現を CF/Gosper から Tier 1
   （代数的数）へ置換し、`continued_fraction.rs` の実装複雑性を退役させる。
2. 数値観測の統一インターフェース（`Observation` / `Refine` / water）を導入し、
   比較予算・表示予算をその上の一資源に統合する。
3. Tier ≤ 1 同士の比較を予算非依存・完全決定可能にする。
4. Tier 2 のインターフェースと最小参照実装を用意する（語彙には未接続）。
5. `SPECIFICATION.html` を新モデルに合わせて改訂する（§ 一覧は §7 参照）。

### 非ゴール（今回やらないこと）
- 表面文法（トークン・リテラル・語名）の変更。新語の追加は原則なし。
- elastic engine / child runtime / audio / GUI レイアウトへの変更。
- Tier 2 の語彙接続（π や LOG の追加）。受け皿だけ作る。
- 有理数演算の観測結果の変更。**1 bit も変えない**。

## 2. 現状の事実（実装者はまず自分で再確認すること）

改修前に必ず読むファイルと、依拠してよい確認済みの事実:

- `rust/src/types/fraction.rs` — `Fraction` は
  `FractionRepr::Small(i64,i64)` / `Big{BigInt,BigInt}` の二段有理数。
  分母 0 を NIL 番兵に使う（`Fraction::nil()`）。**これがそのまま Tier 0**。
- `rust/src/types/continued_fraction.rs` —
  `ExactReal { Rational(Fraction), AlgebraicSqrt{radicand}, Gosper(Arc<Gosper>) }`。
  算術は Möbius / Bihomographic の Gosper 変換。比較は
  `cmp_with_budget*`（`DEFAULT_COMPARISON_BUDGET = 256`）で、予算切れは
  `CmpOutcome::Undecided { agreed_prefix }`。
- `rust/src/types/multiquadratic.rs` — 「admitted exact-real domain D の
  multiquadratic 正規形」が**既に存在する**。Tier 1 の種として最大限再利用する。
- `rust/src/types/mod.rs` — `ValueData::Scalar(Fraction)` と
  `ValueData::ExactScalar(ExactReal)` が併存。`DenseTensor` は Fraction レーン。
- `rust/src/interpreter/comparison.rs` — `Undecided` を論理 `UNKNOWN`
  （`diagnosis.agreedPrefix` 付き）へ射影する唯一の経路。
- `rust/src/types/display.rs` — 無理数の正準表示は SPEC §4.2.3 の
  入れ子 CF 形 `( a0 ( a1 ( a2 … ) ) )`（`CF_DISPLAY_BUDGET` で打ち切り）。
  テストが「sqrt() 表示や ~近似表示を使ってはならない」ことを固定している。
- 無理数を生む語は `MATH` モジュールの `SQRT` / `SQRT-EPS` / `POW` 系のみ
  （`rust/src/interpreter/modules/module_builtins.rs`）。超越関数語は存在しない。
- WASM 境界: `rust/src/wasm_interpreter_bindings/wasm_value_conversion.rs`。
- 品質ゲート: `cargo test --lib` / `cargo test --tests`（`rust/` 内）、
  `npm run check` / `npm run test` / `npm run check:semantic-firewall` /
  `npm run provenance:check`、ファイルサイズ予算
  （`scripts/check-file-size-budget.mjs`）、形式化カバレッジ
  （`docs/formalization-coverage.json` + `scripts/check-formalization-coverage.mjs`）。

Phase 0 でこれらを棚卸しし、事実と食い違いがあればこの文書ではなく
**コードと SPEC を正とする**こと。

## 3. 新アーキテクチャ

### 3.1 Observation インターフェース（意味的契約）

新モジュール `rust/src/types/exact/`（複数ファイルに分割し、ファイルサイズ
予算を守る）に以下を導入する。シグネチャの細部は実装者裁量だが、
**意味的契約**は次の通り:

```rust
/// 数値観測の統一プリミティブ。
/// water を消費して、値の有理区間近似を精緻化する。
trait Observation {
    /// 現在知られている包含区間（端点は有理数）を返す。water 消費なし。
    fn current_interval(&self) -> RatInterval;
    /// water を最大 `w` 消費して区間を精緻化する。
    fn refine(&mut self, w: Water) -> Refine;
}

enum Refine {
    /// 正確値に到達（以後 water 消費ゼロ）。Tier 0 は常に即これ。
    Settled(Fraction),
    /// 区間は狭まったが正確値は表せない（無理数）。何度でも呼べる。
    Narrower,
    /// この観測過程は恒久的に空（= NIL の観測論的対応物）。
    Empty,
    /// 与えられた water では進めなかった（= UNKNOWN の源泉）。
    Starved,
}
```

契約:
- **単調性**: `refine` 後の区間は直前の区間に包含される。
- **収束性**: Tier ≤ 1 の値は有限 water で符号・floor・比較が必ず決定する
  （分離限界が計算可能なため）。Tier 2 のみ `Starved` を返し得る。
- **決定性**: 同じ値に同じ総 water を与えたら同じ区間列になる。
- **非観測性** (SPEC §4.8): どの Tier・どの内部表現を通ったかは値の同一性・
  表示・シリアライズに現れない。

### 3.2 三層の実体

- **Tier 0 — `Fraction`（変更なし）**
  `ValueData::Scalar(Fraction)` と `DenseTensor` はそのまま。
  `Observation` としては `current_interval` が点区間、`refine` が即 `Settled`。
- **Tier 1 — 代数的数（今回の主工事）**
  実装方式は既存 `multiquadratic.rs` の正規形を第一級に昇格させる案
  （後述 D1 の既定解）。代表 API:
  - 構築: `from_fraction`, `sqrt`（完全平方・零は Tier 0 へ射影。現行
    `ExactReal::from_sqrt_rational` と同じ規範）、体演算 `add/sub/mul/div/neg/recip`。
  - 決定可能演算: `cmp`（**予算引数なしで `Ordering` を返す**）、`floor/ceil/round`、
    `is_zero`、`sign`。ゼロ判定は正規形で代数的に行い、区間精緻化は
    高速化にのみ使う（正しさを区間に依存させない）。
  - 降格: 値が有理数になった瞬間 Tier 0 表現へ正規化する
    （cheapest-tier-wins。`(1+√2)·(√2−1) = 1/1` は Tier 0 の `1/1` と同一値・同一表示）。
- **Tier 2 — 一般計算可能実数（骨組みのみ）**
  「単調に縮小する有理区間の遅延生成器」としての実装型を 1 つ定義し、
  `Observation` を実装し、単体テストを付ける。**語彙には接続しない**。
  比較で `Starved` → 論理 `UNKNOWN` へ射影する経路だけ結線し、
  現行語彙からは到達不能であることをテストで固定する。

### 3.3 water（予算の一元化）

- 既存の比較予算（`DEFAULT_COMPARISON_BUDGET`）と表示予算
  （`CF_DISPLAY_BUDGET`）を「観測に費やす water」として一つの語彙に統合する。
- **Tier ≤ 1 の比較は water を消費しない**（決定可能なので概念上不要）。
  `COMPARE-WITHIN` の budget 引数は受理し続けるが、Tier ≤ 1 では結果に影響
  しない（SPEC の文言更新は §7 参照）。Tier 2 が絡む比較でのみ water として
  機能する。
- 評価ステップ上限（SPEC §5.3）との統合は**今回はしない**。将来の統合を
  妨げない命名・型にだけ留意する。

### 3.4 比較と UNKNOWN

`rust/src/interpreter/comparison.rs` を次のルーティングに変更:

1. Tier 0 × Tier 0 → 既存の `Fraction` 比較（完全に現状維持）。
2. Tier ≤ 1 が絡む → Tier 1 の決定可能 `cmp`。**`UNKNOWN` は出ない**。
3. Tier 2 が絡む（現行語彙では到達不能）→ water 消費付き区間比較。
   枯渇時のみ論理 `UNKNOWN` + 診断。

診断キー `diagnosis.agreedPrefix` は Tier 2 経路専用として温存し、意味を
「CF の一致項数」から「両観測が分離しないまま消費した精緻化ステップ数」に
再定義する（D3 参照）。NIL/UNKNOWN/error の三分、Kleene 論理
（SPEC §7.5）、NIL passthrough（§4.5.1）は**一切変更しない**。

### 3.5 表示・シリアライズ（互換性の要）

- 有理数の表示（`p/q`）は不変。
- 無理数の正準表示は **SPEC §4.2.3 の入れ子 CF 形を維持する**。
  Tier 1 値からの CF 項抽出は `floor` + 逆数の反復で行う
  （Tier 1 は `floor` と比較が正確に決定できるため、CF 項は表示予算まで
  正確に導出できる。√有理数は既存 `sqrt_cf_period` 相当の周期性も使える）。
  つまり **CF は「内部表現」から「表示用の導出形」へ格下げ**される。
  `display.rs` の既存テスト（sqrt() 表示禁止・~近似禁止）は緑のまま保つ。
- WASM/TS 境界のプロトコル (`value_protocol.rs`, `wasm_value_conversion.rs`)
  で CF 項列・radicand 等を露出している箇所は Phase 0 で棚卸しし、
  観測結果（表示文字列・区間・分類タグ）ベースで互換に保つ。プロトコル
  形状の変更が避けられない場合は D4 として起票し、TS 側
  （`src/wasm-interpreter-types.ts` ほか）と同時に更新する。

## 4. 段階的実装計画

各 Phase の終わりで全品質ゲート（§2 末尾）が緑であること。Phase を跨いで
壊れた状態をコミットしない。

- **Phase 0 — 棚卸し（コード変更なし）**
  `ExactScalar` / `ExactReal` / `cmp_with_budget` / `partial_quotients` /
  `agreedPrefix` の全参照箇所（interpreter, display, wasm bindings, cli,
  semantic, tests）を列挙し、`docs/dev/` に移行マップの短いメモを残す。
  `SQRT` が既に無理数である入力（例: `2 SQRT SQRT`）に現在どう応答するか
  （bubble か、admitted domain 拡張か）を確認し、その挙動を保存対象として
  記録する。
- **Phase 1 — インターフェース導入（挙動不変）**
  `types/exact/` に `Observation` / `Refine` / `Water` / `RatInterval` を追加。
  `Fraction` に Tier 0 アダプタを付ける。既存経路は触らない。
- **Phase 2 — Tier 1 実体**
  `multiquadratic.rs` を核に代数的数型（仮称 `Algebraic`）を実装。
  現行 `ExactReal` の算術 API（`add/sub/mul/div/neg/reciprocal/floor/ceil/
  round/best_rational_approximation`）と等価な決定可能 API を揃え、
  性質テスト（環公理、`√r·√r = r`、有理数への降格、正規形の一意性）を書く。
- **Phase 3 — 切替**
  `ValueData::ExactScalar` の中身を `Algebraic`（+ Tier 2 受け皿の enum）に
  差し替え、`SQRT`/算術/比較/表示/WASM 境界を新経路へ。
  `comparison.rs` から Tier ≤ 1 の `Undecided` 経路を削除。
  既存テストのうち「予算切れで UNKNOWN になる」ことを固定しているものは、
  仕様変更として Tier 2 のユニットテストへ移設する（削除ではなく移設）。
- **Phase 4 — CF 退役**
  `continued_fraction.rs` から Gosper 算術・予算付き比較を削除し、
  残すのは CF 項導出（表示用）のみ。ファイルは `types/exact/cf_display.rs`
  程度の規模（目安: 数百行）へ縮退。`elastic` 等からの参照が残っていれば追随。
- **Phase 5 — 正典と文書の更新**（§7 の仕様改訂を実施）
  `npm run provenance:attest` を忘れずに再生成。
- **Phase 6（任意）— Tier 2 骨組み**
  §3.2 の通り。時間が余った場合のみ。

## 5. 不変条件（違反したらその Phase は失敗）

1. 有理数のみのプログラムの観測結果（値・順序・表示・シリアライズ・
   NIL 理由）は完全不変。
2. NIL / UNKNOWN / error の三分と伝播規則は不変。UNKNOWN の**発生源が
   縮む**（現行語彙では出なくなる）ことは意図された仕様変更であり、
   UNKNOWN という値・Kleene 論理・`COMPARE-WITHIN` の存在は不変。
3. 無理数の正準表示は引き続き入れ子 CF 形（§3.5）。
4. 表現の非観測性（SPEC §4.8）: 適合判定・値同一性が Tier やレーンに
   依存しない。
5. 決定論: 同一プログラムは同一結果。
6. `1 0 /` → 理由付き NIL、`^`(VENT) フォールバック等、README の
   「A small taste」の例が全て同じ観測結果を返す。
7. `'math' IMPORT 2 SQRT 2 LT` は `TRUE` を返す。**ただし改修後は予算に
   無関係に決定される**。

## 6. 設計裁量点（Decision Points）と既定解

- **D1: Tier 1 の実装方式。**
  既定解: `multiquadratic.rs` 正規形の昇格（現行到達領域と一致し工数最小）。
  代替: 最小多項式 + 分離区間の一般代数的数。**√ の入れ子（`SQRT` の再適用）
  が現行 admitted domain に含まれる場合**は既定解で覆えるか Phase 0 で判定し、
  覆えなければ (a) 現行と同じ境界で bubble にする (b) 一般代数的数に切替える、
  のいずれかを選び、理由をメモに残す。
- **D2: `SQRT-EPS` / `POW` の扱い。**
  既定解: 観測結果を現状と一致させる（実装だけ Tier 1/Tier 0 経由に変更）。
- **D3: `agreedPrefix` の再定義。**
  既定解: キー名は維持、意味を「分離に至らなかった精緻化ステップ数」へ
  SPEC 側で再定義（Tier 2 専用）。
- **D4: WASM プロトコルに CF 内部が漏れていた場合。**
  既定解: 観測ベースの互換フィールドで置換し、TS 側を同時更新。破壊的変更が
  必要なら実装を止めてオーナーに確認する。
- 上記以外で SPEC の観測可能な意味を変えたくなった場合は、**実装を進めずに
  オーナーへ質問すること**。

## 7. 正典 (`SPECIFICATION.html`) の改訂範囲

Phase 5 で以下を改訂する（文体は `docs/dev/ajisai-authoring-style.md` に従う）:

- **§1 言語アイデンティティ**: "continued-fraction dataflow language" を
  "exact-real dataflow language"（表現非依存の厳密実数）へ。連分数は
  §4.2.3 の正準表示形としてのみ言及。
- **§4.2**: 内部表現の規定を Observation 契約（§3.1 の単調・収束・決定・
  非観測）+ 三層コストクラスに置換。§4.2.5（nearest-integer CF 比較）は削除
  または「歴史的注記」へ。
- **§7.4.1 / §7.4.2**: 「Tier ≤ 1（現行全語彙）の比較は決定可能。
  比較予算は Tier 2 観測にのみ意味を持つ」へ書き換え。`COMPARE-WITHIN` の
  シグネチャは不変。
- **§4.8 コストモデル**: 表現コストクラスを Tier 0/1/2 で言い直す。
- **§4.5.2 / §11**: 変更なしを確認（三分は不変）。
- 併せて `README.md`・`SKILL.md`・`public/docs/`（Reference）の該当例・
  文言、`docs/formalization-coverage.json` の tier 記載を整合させ、
  `npm run check:semantic-firewall` と conformance manifest を緑にする。

## 8. テスト規律

- SPEC §15 に従う: 新規 Tier 1 演算の契約カバレッジ、NIL 理由カバレッジ、
  複合判定の MC/DC。
- 追加必須テスト:
  - Tier 1 決定可能性: `2 SQRT 2 LT` が予算 1 でも `TRUE`。
    `(1+√2)(√2−1) = 1` が Tier 0 表示 `1/1` に一致。
    `√2 = √2` が `TRUE`（現行では予算次第で UNKNOWN になり得た代表例）。
  - 表示互換: `2 SQRT` の入れ子 CF 表示が改修前後で一致
    （√2 = [1; 2, 2, …] を含む代表値のゴールデンテスト。改修前の期待値は
    Phase 0 で採取しておく）。
  - Tier 2 隔離: 現行語彙で `Starved`/UNKNOWN 比較経路に到達しないこと。
- 性能: `rust/benches/interpreter-performance-benchmarks.rs` と
  `bench-baselines/` で回帰確認。有理数経路の悪化は不可。無理数演算は
  改善されるはず（Gosper 廃止のため）だが、悪化した場合は原因を記録。

## 9. 進め方・作法

- 作業ブランチはこの文書が載っているブランチから派生させる（またはオーナー
  指定のブランチ）。Phase ごとに小さくコミットし、コミットメッセージに
  Phase 番号を含める。
- 既存コードのコメント密度・命名・イディオムに合わせる。巨大ファイルを
  作らない（ファイルサイズ予算スクリプトが CI で落とす）。
- 各 Phase 完了時に §2 末尾の全ゲートを実行し、結果をそのまま報告する
  （失敗を丸めない）。
- この文書と現実が食い違ったら、コードと SPEC を正とし、食い違いを
  報告してから進む。

## 10. 完了条件チェックリスト

- [ ] Phase 0 の移行マップが `docs/dev/` にある
- [ ] `Observation`/`Refine`/water が `types/exact/` に導入されている
- [ ] `ExactScalar` の中身が Tier 1 代数的数になり、Gosper 算術が消えている
- [ ] Tier ≤ 1 比較が予算非依存で決定し、現行語彙から UNKNOWN 比較が消えた
- [ ] 無理数の入れ子 CF 表示が改修前とゴールデン一致
- [ ] `continued_fraction.rs` が表示用導出のみに縮退（大幅減行）
- [ ] Tier 2 骨組みが存在し、語彙から到達不能なことがテストで固定
- [ ] SPEC / README / Reference / coverage / firewall / provenance がすべて更新・緑
- [ ] 全品質ゲート緑、ベンチ回帰なし
