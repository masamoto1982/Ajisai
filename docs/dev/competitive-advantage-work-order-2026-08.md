# 競争優位の研磨：改修指示書（2026-08）

Status: **非正典（`[設計根拠]`）**。この文書は Ajisai の意味論を定義しない。
正典は `SPECIFICATION.html` と `spec/` 配下のソースのみ。本書と正典が矛盾したら正典が勝つ。

対象実装者: **Claude Sonnet**（またはそれに準ずるエージェント）
作業ブランチ: 各 Phase ごとに新しいブランチを切る（`npm run branch:new` を使う）

---

## 0. この文書の読み方（実装者向け・最初に必ず読む）

### 0.1 進め方の鉄則

1. **Phase は上から順に、1つずつ、独立したコミット／PR として仕上げる。**
   Phase をまたいだ変更を1つのコミットに混ぜてはならない。
2. **各 Phase の「触ってよいファイル」ホワイトリストを守る。** そこに無いファイルを
   編集したくなったら、それは設計判断が必要になった合図なので **Step 0.3 に従って停止**する。
3. **各 Step の「完了条件」は機械的に判定できる形で書いてある。** 完了条件を満たさないまま
   次の Step に進んではならない。
4. **設計判断はすべて本書で確定済み。** 定数値・命名・フィールド順・エラー文言まで
   指定してある。「もっと良い名前がある」と思っても変更しない。名前の一貫性は
   後続 Phase が依存している。
5. **落とし穴セクションを読む前にコードを書き始めない。** 各 Phase の「落とし穴」は、
   素朴に実装すると *テストが通ったまま間違う* 箇所を列挙してある。

### 0.2 全 Phase 共通の禁止事項

以下は Phase を問わず禁止。破ると CI が落ちるか、言語の同一性が壊れる。

- ❌ **`spec/words.json` の既存エントリの意味論的フィールドを変更しない**
  （`stack` / `consumption` / `nilPolicy` / `projection` / `errorWhen` / `purity` /
  `determinism` / `capability` / `hostedEffect`）。本書のどの Phase も語彙を変えない。
- ❌ **Core Word を追加・削除・改名しない。** 65語は固定。
- ❌ **`SPECIFICATION.html` を直接編集しない。** 生成物である
  （`npm run specification:generate`）。
- ❌ **既存の JSON フィールドを削除・改名・型変更しない。** 追加のみ許される
  （`LANG.OBSERVATION.PROTOCOL`：現行バージョン内では optional フィールドの追加のみ）。
- ❌ **`report.rs` の `SCHEMA_VERSION` を上げない。** 本書の変更はすべて純粋な追加。
- ❌ **既存テストを削除・スキップ・`#[ignore]` しない。** 落ちたら実装を直す。
- ❌ **`rust/src/` に 500 行を超える新規ファイルを作らない**
  （`npm run check:file-size` が落ちる）。超えそうなら本書の指示どおりモジュールを分割する。
- ❌ **`unsafe` を書かない。**
- ❌ **`docs/dev/` 以外に散文ドキュメントを新設しない。**

### 0.3 停止条件（詰まったら黙って続行せず、ここで止めて報告する）

次のいずれかに当たったら、**作業を止めて、何が起きたかと選択肢を報告する**。
推測で進めてはならない。

- 本書の「落とし穴」に書かれていない形で、既存テストが落ちた
- ホワイトリスト外のファイルを編集する必要が生じた
- 正典（`SPECIFICATION.html` / `spec/`）の記述と実装が食い違っていることを発見した
  （→ `docs/dev/spec-impl-drift-tactic.md` の裁定手順に載せる案件）
- Phase の受け入れ条件が、指定どおり実装しても満たせない

### 0.4 よく使う検証コマンド

```sh
# Rust（作業ディレクトリは rust/）
cd rust && cargo fmt --check
cd rust && cargo clippy --all-targets -- -D warnings
cd rust && cargo test --lib
cd rust && cargo test --tests

# CLI をビルド（Node 側ジェネレータが使う）
cargo build --bin ajisai --manifest-path rust/Cargo.toml

# リポジトリ全体のゲート（ルートで）
npm run check:file-size
npm run check:semantic-firewall
npm run check:agent-cli-contract
npm run check:skill
npm run specification:check
```

### 0.5 用語

| 用語 | 意味 |
| --- | --- |
| 観測（observation） | `LANG.OBSERVATION.PROJECTIONS`：stack / output / dictionary / diagnosis の4面 |
| 三分法 | 値 / NIL（理由付き）/ ERROR（`LANG.FAILURE.TRICHOTOMY`） |
| 保守的（conservative） | 契約推論が証明しきれず、安全側に倒した状態（`ContractConfidence::Conservative`） |
| gap | 「検証不能（cannot verify）」の具体的原因を表す安定 ID（Phase 3 で導入） |

---

## Phase 1 — 観測ダイジェスト（`observationDigest`）

### 1.1 目的

1つのプログラムの**観測全体を32バイトの BLAKE3 ダイジェストに畳む**。
これにより「同じ結果か」が、値を転送せずに、実装をまたいで判定できるようになる。

**この Phase が成立する根拠**：Ajisai は 65語のうち非決定的な語が `PRINT` の1語のみ
（`spec/words.json` の `determinism` / `hostedEffect` を参照）、`RANDOM` すら種決定的、
浮動小数点なし、時計なし、FFI なし。つまりソース → 観測が純関数である。
かつ値の等価性が**表現ではなく意味**で定義されている
（`rust/src/types/value_hash_tests.rs` の冒頭コメントを読むこと）。

### 1.2 触ってよいファイル（ホワイトリスト）

```
新規: rust/src/agent/observation_digest.rs
新規: rust/src/agent/observation_digest_tests.rs
編集: rust/src/agent/mod.rs                    （mod 宣言の追加のみ）
編集: rust/src/agent/report.rs                （フィールド1つ追加）
編集: rust/src/agent/api.rs                   （Report 構築時の値の受け渡し）
編集: rust/src/cli/mod.rs                     （error 経路の Report 構築があれば）
編集: docs/dev/agent-cli-output-contract.md   （新フィールドの記載）
```

これ以外は触らない。特に **`rust/src/types/` 配下は読むだけで、変更しない**。

### 1.3 事前に読むファイル（コードを書く前に必ず）

| ファイル | 読む理由 |
| --- | --- |
| `rust/src/types/value_hash_tests.rs`（冒頭コメント全部） | 等価性が満たすべき3つの関係が書いてある。ダイジェストはこれと一致しなければならない |
| `rust/src/types/mod.rs` の `impl PartialEq for ValueData`（101行付近） | 何が等しさに含まれるか／含まれないかの唯一の定義 |
| `rust/src/types/mod.rs` の `impl PartialEq for Value`（321行付近） | `hint` は等しさに含まれず、`absence` は含まれる |
| `rust/src/types/exact/algebraic.rs` の `impl Hash for Algebraic`（384行付近） | **Phase 1 で一番重要**。落とし穴 A の答えがここにある |
| `rust/src/types/fraction.rs` の `impl Hash for Fraction`（129行付近） | 既約化と符号正規化の正しいやり方 |
| `rust/src/interpreter/word_identity.rs` の `content_digest` / `body_content_key` | BLAKE3 の使い方と、トークン列のバイト符号化の既存規約 |
| `rust/src/agent/report.rs` の `Report` / `to_json` | 追加先 |

### 1.4 ⚠️ 落とし穴（実装前に必ず読む。ここを外すと、テストが通ったまま前提が壊れる）

#### 落とし穴 A：代数的数の正規形は**正準ではない**

`8 SQRT` と `2 SQRT 2 SQRT +` は README が保証するとおり **等しい値**だが、
内部の `normal_form_terms()` は **一致しない**。

- `8 SQRT` → 基底 `{8}`、項 `{8: 1}`（= 1·√8）
- `2 SQRT 2 SQRT +` → 基底 `{2}`、項 `{2: 2}`（= 2·√2）

理由：`rust/src/types/exact/basis.rs` の基底は **GCD-free 基底**であって素因数分解ではない。
`Basis::build([8])` は 8 が平方数でないためそのまま `{8}` を採る。
つまり **`normal_form_terms()` / `exact_terms()` をハッシュしてはならない**。
そうすると等しい値が異なるダイジェストになり、この Phase の前提そのものが壊れる。

**正しい方法**：`impl Hash for Algebraic`（`algebraic.rs` 384行付近）が既にこの問題を
解いている。`floor(value × 2^K)` を、包囲区間 `bounds(bits)` の下限と上限の floor が
一致するまで `bits` を倍々にして求める。Algebraic は必ず無理数
（有理数は eager に降格する）なので `2^-K` の格子線上に載ることはなく、必ず停止し、
**どの表現から出発しても同じ整数に収束する**。

Phase 1 の符号化はこのアルゴリズムを、`HASH_KEY_BITS` ではなく本書が定める
`DIGEST_ALGEBRAIC_BITS = 512` で再実装する（既存の `Hash` 実装は変更しない）。

**正直に記録すべき残余**：この方式は「等しい値 ⇒ 等しいダイジェスト」（健全性）は
保証するが、「異なる値 ⇒ 異なるダイジェスト」は代数的数について
`2^-512` より近い相異なる2値に対しては保証しない。これは仕様として
`docs/dev/agent-cli-output-contract.md` に明記すること（Step 1.8）。
「保証している」と書いてはならない。

#### 落とし穴 B：Tensor と Vector は**等しいのに構造が違う**

`ValueData::Vector` と `ValueData::Tensor` は、矩形かつ数値の場合に等しくなる
（`tensor_eq_vector`、`rust/src/types/mod.rs` 133行付近）。
**`ValueData` の variant で分岐して符号化してはならない。**

**正しい方法**：コレクション（Vector / Tensor）は `Value::len()` と `Value::child(i)`
（`rust/src/types/value_children.rs`、表現非依存）だけを使って再帰的に符号化する。

ただし **Scalar / ExactScalar は `len() == 1` かつ `child(0) == self` を返すので、
先に葉として分岐しないと無限再帰する**。符号化関数は必ず
「まず葉（Scalar / ExactScalar / Boolean / Text / Nil / CodeBlock）を判定し、
残った Vector / Tensor だけを `len` + `child` で再帰」という順序にすること。

#### 落とし穴 C：`hint` は等しさに入らない、`absence` は入る

- `Value::hint`（`Interpretation`）は**表示の役割**であって意味ではない。
  **ダイジェストに含めてはならない。**
- `Value::absence`（`AbsenceMetadata`）の **reason は含める**。
  `LANG.VALUES.NIL` が「reason は NIL の観測内容の全て」と定めており、
  `PartialEq for Value` も reason を比較している。
  reason 以外のリッチ診断（メッセージ等）は含めない。

#### 落とし穴 D：`stackDisplay` を digest してはならない

`stackDisplay` は連分数表示を**表示予算で打ち切った**文字列である
（`rust/src/types/value_protocol.rs` の `exact_display` 周辺コメント参照。√2 は
約194文字で `...)` で終わる）。打ち切られた表示をハッシュすると、
異なる値が同じダイジェストになる。**必ず値そのものを符号化する。**

#### 落とし穴 E：`serde_json` の出力順に依存してはならない

このリポジトリの `serde_json` は `preserve_order` を有効にしていないため
現状はキー順が安定するが、**それに依存した実装をしない**。
本書が定める明示的なバイト文法（Step 1.4）で符号化すること。

#### 落とし穴 F：`ExactReal::Computable` は符号化できない

Tier 2（`Computable`）は遅延包囲であり、正準な有限表現を持たない。
現在の語彙にはこれを構成する語が無い（`rust/src/types/exact/value.rs` のコメント参照）が、
将来のために **`Computable` に出会ったら `None` を返して、ダイジェスト自体を出力しない**
（`observationDigest` は `null`）。捏造した値を返してはならない。

### 1.5 手順

#### Step 1.1 — 新規モジュールの骨組み

`rust/src/agent/observation_digest.rs` を作成する。冒頭に必ずモジュールコメントを書き、
**落とし穴 A・B・C・D の理由をそこに記録する**（このリポジトリの慣習：なぜその形なのかを
コードの隣に残す）。

公開 API はこの2つだけ：

```rust
/// この符号化スキーマのバージョン。バイト文法を変えたら上げる。
pub(crate) const DIGEST_SCHEMA_TAG: &[u8] = b"AJISAI-OBS-1";

/// 代数的スカラーを正準整数キーに落とすときの精度（ビット）。
/// 落とし穴 A を参照：これは健全性（等値 ⇒ 同ダイジェスト）を与えるが、
/// 2^-512 より近い相異なる代数的数の分離は保証しない。
pub(crate) const DIGEST_ALGEBRAIC_BITS: u32 = 512;

/// 観測全体の正準ダイジェスト。`None` は「この観測は符号化できない」
/// （落とし穴 F：Tier 2 の値が含まれていた）を意味する。
pub(crate) fn observation_digest(input: ObservationDigestInput<'_>) -> Option<String>;
```

入力型：

```rust
pub(crate) struct ObservationDigestInput<'a> {
    /// Report の status（"ok" / "error"）。
    pub status: &'a str,
    /// スタック（bottom → top）。`Interpreter::get_stack()` の順序をそのまま使う。
    pub stack: &'a [Value],
    /// PRINT が生んだ出力行（順序どおり）。
    pub output: &'a [String],
    /// User 辞書：(正規化された語名, content identity)。呼び出し側は
    /// 名前の昇順にソートして渡すこと。
    pub user_words: &'a [(String, String)],
    /// エラーの安定カテゴリ ID。成功時は None。人間向けメッセージは渡さない。
    pub error_category: Option<&'a str>,
}
```

**完了条件**：`cd rust && cargo check` が通る（中身は `todo!()` でよい）。

#### Step 1.2 — バイト文法の実装

以下の文法を**そのまま**実装する。独自に変えないこと。

```
observation := DIGEST_SCHEMA_TAG
             , section(0x01) , str(status)
             , section(0x02) , u64(stack.len()) , value* (bottom→top)
             , section(0x03) , u64(output.len()) , str(line)*
             , section(0x04) , u64(user_words.len()) , (str(name) , str(identity))*
             , section(0x05) , opt_str(error_category)

section(id)  := 0x1D , id:u8
u64(n)       := n を 8 バイトのビッグエンディアンで
str(s)       := u64(s.len() as u64) , s の UTF-8 バイト列
opt_str(o)   := None なら 0x00 / Some(s) なら 0x01 , str(s)
sint(i)      := str(i.to_str_radix(16))     # BigInt。符号付き 16 進文字列
```

値の符号化（**必ずこの順に判定する**。落とし穴 B）：

```
value :=
    # --- 葉：先に判定する ---
    | b'N' , str(absence_reason)          # ValueData::Nil。reason はプロトコル文字列
    | b'B' , 0x00 or 0x01                 # ValueData::Boolean
    | b'S' , str(text)                    # ValueData::Text
    | b'K' , u64(tokens.len()) , token*   # ValueData::CodeBlock
    | b'Q' , sint(num) , sint(den)        # 有理スカラー（既約・分母正）
    | b'A' , sint(floor(x * 2^512))       # 代数的スカラー（落とし穴 A）
    # --- コレクション：最後 ---
    | b'V' , u64(len()) , value*          # Vector / Tensor を区別せず child(i) で再帰
```

- `b'Q'` の既約化と符号正規化は **`impl Hash for Fraction`（`fraction.rs` 129行付近）と
  同じ手順**を踏むこと：gcd で割り、分母が負なら両方の符号を反転する。
  ただし `i64` へのダウンキャストはしない（`sint` で BigInt のまま符号化する）。
- `b'A'` の整数キーは、`impl Hash for Algebraic` と同じループを
  `DIGEST_ALGEBRAIC_BITS` で回して求める。`bits` の初期値は
  `DIGEST_ALGEBRAIC_BITS + 8`、収束しなければ倍にする。
- `token*` の符号化は `rust/src/interpreter/word_identity.rs` の `body_content_key` が
  使っているバイト割り当て（`b'N'` / `b'n'` / `b'S'` / `b'Y'` / `[` / `]` / `{` / `}` /
  `^` / `|` / `\n`）を**そのまま流用**する。数値トークンは `body_content_key` と同じく
  `Fraction::from_str` で正規化してから符号化すること。
  同じ実装を2箇所に書かないこと：`word_identity.rs` 側に
  `pub(crate) fn encode_token(bytes: &mut Vec<u8>, token: &Token)` を切り出し、
  両者から呼ぶ。**これが `word_identity.rs` に対して許される唯一の編集**である
  （抽出のリファクタのみ。既存の digest 出力バイト列が変わってはならない）。
- `ExactReal::Computable` に出会ったら即座に `None` を返して全体を中止する（落とし穴 F）。

最後に `crate::interpreter::word_identity::content_digest(&bytes)` に渡し、
`#` + 64桁小文字16進の文字列を返す。

**完了条件**：`cd rust && cargo build` が通り、`clippy -D warnings` も通る。

#### Step 1.3 — Report への追加

`rust/src/agent/report.rs`：

1. `Report` 構造体に `pub observation_digest: Option<String>` を追加。
   ドキュメントコメントに「`None` は符号化不能（Tier 2 の値）を意味する」と書く。
2. `Report::to_json` の JSON オブジェクトに `"observationDigest": self.observation_digest`
   を追加する。**位置は `"stackElided"` の直後**（末尾に追加）。
3. `SCHEMA_VERSION` は**上げない**。

`rust/src/agent/mod.rs` の `error_report` と `rust/src/agent/api.rs` の成功経路の
両方で `observation_digest` を計算して詰める。User 辞書のエントリは
`Interpreter` から語名と content identity を取り、**名前の昇順にソートしてから**渡す。

`rust/src/cli/mod.rs` で `Report` を構築している箇所があれば同様に埋める
（フィールド追加でコンパイルエラーになる箇所がそのまま作業リスト）。

**完了条件**：`cd rust && cargo test --lib` が通る。

#### Step 1.4 — テスト（この Phase の本体）

`rust/src/agent/observation_digest_tests.rs` を作り、`mod.rs` に
`#[cfg(test)] mod observation_digest_tests;` を追加する。

**必須テスト（全部書くこと）**：

| # | 名前 | 内容 |
| --- | --- | --- |
| 1 | `equal_values_digest_equally` | `rust/src/types/value_hash_tests.rs` の値コーパスを流用し、`a == b` なら `encode(a) == encode(b)` を全ペアで検査する。**これがこの Phase の中心的な不変条件** |
| 2 | `sqrt_eight_matches_sqrt_two_plus_sqrt_two` | `8 SQRT` と `2 SQRT 2 SQRT +` を実行し、観測ダイジェストが一致する（落とし穴 A の回帰テスト） |
| 3 | `tensor_and_nested_vector_digest_equally` | 矩形数値 Vector とその Tensor 版が一致する（落とし穴 B の回帰テスト） |
| 4 | `hint_does_not_change_the_digest` | `hint` だけが違う2値が一致する（落とし穴 C） |
| 5 | `nil_reasons_separate_digests` | 異なる reason の NIL が異なるダイジェストになる（落とし穴 C の逆向き） |
| 6 | `unreduced_fraction_matches_reduced` | `create_unreduced` 系で作った分数と既約形が一致する |
| 7 | `different_results_digest_differently` | 明らかに違う 10 個程度のプログラムのダイジェストが全て相異なる |
| 8 | `output_order_is_observable` | `1 PRINT 2 PRINT` と `2 PRINT 1 PRINT` が異なる |
| 9 | `dictionary_state_is_observable` | 同じスタック・同じ出力でも User 語が違えば異なる |
| 10 | `digest_is_stable_across_runs` | 同じプログラムを2回実行して一致する |

テスト1がこの Phase の合否を決める。**テスト1が落ちるなら符号化が間違っている**ので、
テストを緩めずに符号化を直すこと。

**完了条件**：`cd rust && cargo test --lib observation_digest` が全て緑。

#### Step 1.5 — ドキュメント

`docs/dev/agent-cli-output-contract.md` に `observationDigest` の節を追加する。
書くべき内容：

- 何を含み、何を含まないか（stack / output / user 辞書 / status / error category を含み、
  `stackDisplay`・人間向けメッセージ・`hint`・リッチ診断を含まない）
- 保証の向き：**「等しい観測 ⇒ 等しいダイジェスト」は保証する。逆は代数的数について
  `2^-512` の残余がある**（落とし穴 A）。ここを「保証する」と書かないこと
- `null` になる条件（Tier 2 の値）
- `DIGEST_SCHEMA_TAG` を変えたら値が変わるので、変更は互換性のある変更ではないこと

**完了条件**：`npm run check:agent-cli-contract` と `npm run check:reading-surfaces` が通る。

#### Step 1.6 — 仕上げ

```sh
cd rust && cargo fmt
cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings
cd rust && cargo test --lib && cargo test --tests
cd .. && npm run check:file-size && npm run check:semantic-firewall
cargo build --bin ajisai --manifest-path rust/Cargo.toml && npm run check:skill
```

### 1.6 受け入れ条件（すべて満たすこと）

- [ ] 上記の検証コマンドが全て緑
- [ ] `ajisai run <file> --json` の出力に `observationDigest` が現れ、`#` + 64桁16進である
- [ ] `8 SQRT` と `2 SQRT 2 SQRT +` の `observationDigest` が一致する
- [ ] `report.rs` の `SCHEMA_VERSION` が 1 のまま
- [ ] 新規 Rust ファイルがいずれも 500 行以下
- [ ] `word_identity.rs` への変更が `encode_token` の抽出のみで、既存の
      content identity のバイト列が変わっていない（既存の identity テストが緑であること）

### 1.7 コミット

```
Add an observation digest to the agent JSON envelope

An Ajisai program is a pure function from source to observation: 64 of 65
Words are deterministic and the one host-relative Word is PRINT, so the whole
observation collapses to one BLAKE3 digest. Value equality is semantic rather
than representational, so the digest is a property of the meaning: 8 SQRT and
2 SQRT 2 SQRT + agree.

The algebraic encoding reuses the enclosure key from `impl Hash for Algebraic`
rather than the normal form, because the GCD-free basis is not canonical —
√8 keeps basis {8} while 2√2 keeps {2}.
```

---

## Phase 2 — 全数意味論表（exhaustive semantics table）

### 2.1 目的

**（Word × 入力ドメイン組）→ 結末カテゴリ** の全数表を生成し、全セルを実インタプリタで
検証する。65語・フラット・import なしという有限性を、
**「AIが一度に読める大きさの完全な意味論」**という資産に変える。
同時に、`spec/words.json` の契約と実装の乖離を総当たりで炙り出す。

規模は事前に計算済み：**入力ドメイン組の総数 1,642**（可変アリティの
`COLLECT` / `COND` / `VENT` を除く）。CLI 起動 1,642 回で数分。実行可能な規模である。

### 2.2 触ってよいファイル（ホワイトリスト）

```
新規: scripts/generate-semantics-table.mjs
新規: docs/semantics-table.json          （生成物。コミットする）
編集: package.json                       （scripts に2行追加）
編集: .github/workflows/test.yml         （ゲート1ステップ追加）
編集: docs/dev/INDEX.md                  （必要なら）
```

**`rust/` は一切触らない。** 実装のバグを見つけても、この Phase では直さず
「発見一覧」として報告する（Step 2.5）。

### 2.3 事前に読むファイル

| ファイル | 読む理由 |
| --- | --- |
| `scripts/generate-skill-md.mjs` の 1〜80 行 | CLI ハーネスの雛形。`resolveAjisaiBin` / `runSnippet` をこの形で真似る |
| `scripts/generate-word-manifest.mjs` | `--check` モードの実装パターン |
| `spec/words.json` | 入力：`stack.inputs` と `name` |
| `SKILL.md` の §9 | 語の一覧と呼び出し形の確認 |

### 2.4 ⚠️ 落とし穴

#### 落とし穴 A：アリティが整数でない語が3つある

`spec/words.json` の `stack.inputs` は `COLLECT` / `VENT` が `"variable"`、
`COND` が `"control"`。**この3語は全数展開の対象から外す**。
除外したことを生成物の中に `"excluded"` セクションとして明示的に記録すること
（黙って落とすと「表が完全である」という主張が嘘になる）。

#### 落とし穴 B：`KEEP` は修飾子であって語ではない

`KEEP` は次の語を修飾する。単独で全数展開しても意味がない。
`KEEP` も除外し、`excluded` に理由を書く。

#### 落とし穴 C：`DEF` / `DEL` は辞書を変える

これらを表に含めると各セルの実行が独立でなくなる。
CLI は1プロセス1プログラムなので実害はないが、`DEF` は
`{ body } 'NAME' DEF` という形しか受け付けないため、ドメイン代表値を機械的に
並べたプログラムはほぼ全て ERROR になる。**それでよい**——表の目的は
「何がどう失敗するか」を全数で記録することであって、成功例を集めることではない。
除外しないこと。

#### 落とし穴 D：結末カテゴリの判定は message ではなく安定 ID で行う

`--json` の `message` は人間向けで、文言は変わりうる。判定に使うのは：

- 成功かつ最終スタック最上位が NIL → `nil:<reason>`（`reason` は
  スタック値の `semantics.absence.reason`。プロトコル文字列）
- 成功かつそれ以外 → `value`
- 失敗 → `error:<category>`（`diagnosis.why` の安定 ID。`message` は使わない）

`message` を表に入れてはならない（文言が変わるたび CI が落ちる）。

#### 落とし穴 E：出力は決定的な順序で

セルの順序は **`spec/words.json` の登場順 × ドメイン組の辞書順**に固定する。
`Object.keys` の順序や `Map` の挿入順に依存しないこと。
`--check` モードは生成物と committed ファイルを**文字列一致**で比較する。

### 2.5 手順

#### Step 2.1 — ドメイン代表値の固定

以下を**そのまま**使う。変更しないこと（後続 Phase と表の互換性が壊れる）。

```js
const DOMAINS = [
  { id: 'scalar',    source: '1' },
  { id: 'boolean',   source: 'TRUE' },
  { id: 'string',    source: "'a'" },
  { id: 'vector',    source: '[ 1 2 ]' },
  { id: 'nil',       source: 'NIL' },
  { id: 'codeblock', source: '{ 1 }' },
];
```

順序も固定。`DOMAINS` の並び順がドメイン組の辞書順を定める。

#### Step 2.2 — ジェネレータ本体

`scripts/generate-semantics-table.mjs`：

1. `spec/words.json` を読み、`stack.inputs` が整数の語だけを対象にする
   （落とし穴 A）。`KEEP` を除外（落とし穴 B）。
2. 各語について、`inputs` 個のドメイン組を全列挙（`DOMAINS.length ** inputs` 通り）。
3. 各組についてプログラム文字列を組む：`<operand1> <operand2> ... <WORD>`。
   オペランドは `DOMAINS[i].source` をスペース区切りで並べる。
4. `ajisai run <tmpfile> --json` を実行し、落とし穴 D の規則で結末カテゴリを求める。
5. 出力 JSON を書く。

出力形式（**この形をそのまま使う**）：

```json
{
  "schemaVersion": 1,
  "generator": "scripts/generate-semantics-table.mjs",
  "domains": ["scalar", "boolean", "string", "vector", "nil", "codeblock"],
  "excluded": [
    { "word": "COLLECT", "reason": "variableArity" },
    { "word": "COND",    "reason": "controlArity" },
    { "word": "VENT",    "reason": "variableArity" },
    { "word": "KEEP",    "reason": "modifierNotWord" }
  ],
  "cells": [
    { "word": "ADD", "inputs": ["scalar", "scalar"], "outcome": "value" },
    { "word": "ADD", "inputs": ["scalar", "boolean"], "outcome": "error:..." },
    { "word": "DIV", "inputs": ["scalar", "scalar"], "outcome": "value" }
  ]
}
```

`cells` は「語の `words.json` 登場順」→「ドメイン組の辞書順」で並べる。
JSON は 2 スペースインデント、末尾改行あり。

#### Step 2.3 — `--check` モードと npm scripts

`--check` を渡したら、生成結果と `docs/semantics-table.json` の内容を比較し、
差分があれば非ゼロ終了する（`scripts/generate-word-manifest.mjs` の実装に倣う）。

`package.json` に追加：

```json
"semantics:table": "node scripts/generate-semantics-table.mjs",
"semantics:table:check": "node scripts/generate-semantics-table.mjs --check"
```

#### Step 2.4 — CI ゲート

`.github/workflows/test.yml` の「Verify committed SKILL.md is in sync」ステップの
**直後**に追加する（`ajisai` バイナリが既にビルド済みの位置であること）：

```yaml
      - name: Exhaustive semantics table is in sync
        # 65語 × 入力ドメイン組の全数表。契約（spec/words.json）と実装の乖離を
        # 総当たりで検出し、同時に AI 向けの完全な意味論資産になる。
        run: npm run semantics:table:check
```

#### Step 2.5 — 乖離の報告（この Phase の隠れた成果）

生成後、`docs/semantics-table.json` を `spec/words.json` の契約と突き合わせ、
**契約が予告していない結末**を一覧にして報告する。典型例：

- `errorWhen` に該当する条件が無いのに `error:` になったセル
- `nilPolicy` が `rejectNil` なのに NIL 入力が `value` を返したセル
- `projection.when` が `never` なのに `nil:` になったセル

**この Phase では修正しない。** 一覧を報告するだけ。修正は別 PR で人間が優先度を決める。

### 2.6 受け入れ条件

- [ ] `npm run semantics:table` が完走し、`docs/semantics-table.json` が生成される
- [ ] `npm run semantics:table:check` が緑
- [ ] `cells` の件数が 1,642 と一致する（一致しなければ除外リストか展開ロジックが間違い）
- [ ] `outcome` に人間向けメッセージが1つも含まれていない
- [ ] `rust/` に変更が無い
- [ ] 乖離一覧が報告されている（0 件なら「0 件」と明示）

### 2.7 コミット

```
Generate the exhaustive Word x input-domain semantics table

65 Words in one flat dictionary with no imports means the language's whole
input/outcome surface is finite: 1,642 domain tuples. Executing every one of
them through the real CLI turns that finiteness into a complete, machine-checked
artifact, and cross-checks spec/words.json against the implementation in one
sweep.

Outcomes are recorded as stable identifiers only (value / nil:<reason> /
error:<category>); human-readable messages are deliberately excluded so the
gate does not fail on rewording.
```

---

## Phase 3 — gap ID（「検証不能」を測れる負債にする）

### 3.1 目的

`check --contract` の「cannot verify」に**安定した gap ID** を与える。
「保守的で不完全」という正直な注意書きを、その日から**測れる負債**に変える。

### 3.2 触ってよいファイル（ホワイトリスト）

```
編集: rust/src/interpreter/word_contract.rs   （不完全性の理由を記録する）
編集: rust/src/agent/contract_decl.rs         （Finding に code を付ける）
新規: rust/src/agent/contract_gap.rs          （gap ID の定義と集計）
編集: rust/src/agent/mod.rs                   （mod 宣言）
編集: docs/dev/agent-cli-output-contract.md
```

### 3.3 事前に読むファイル

| ファイル | 読む理由 |
| --- | --- |
| `rust/src/agent/contract_decl.rs` 全体 | `Severity::Note` を出す4箇所がここにある |
| `rust/src/interpreter/word_contract.rs` の 350〜420 行 | **不完全性の発生源はちょうど3箇所**。すべてここ |

### 3.4 ⚠️ 落とし穴

#### 落とし穴 A：不完全性の発生源は3箇所しかない（調査済み）

`word_contract.rs` の `complete = false` は3箇所だけ：

| 行（概算） | 状況 | 割り当てる gap ID |
| --- | --- | --- |
| 370 | シンボルが解決できない（`resolve_word_entry` が None） | `gap.unresolvedWord` |
| 378 | 再帰（`visiting` に依存語が既に居る） | `gap.recursiveDependency` |
| 386 | 依存語の推論自体が失敗した | `gap.dependencyUnknown` |

さらに `WordContract::conservative()`（107行）を種として使う経路があるので、
そこには `gap.conservativeSeed` を割り当てる。
**4つ以外を勝手に増やさないこと。** 新しい gap が必要になったら停止して報告する。

#### 落とし穴 B：`Severity::Error` に gap を付けない

gap は「検証**不能**」の理由である。`Error`（= violated、契約違反が証明された）には
gap は無い。`code` フィールドは `Note` のときのみ `Some(...)` になる。

#### 落とし穴 C：三値の意味を変えない

`LANG.CONTRACT.CHECK` は「verified / violated / cannot verify のちょうど3つ」を定めている。
gap ID は cannot verify の**内訳**であって、4つ目の結果ではない。
`violated` の判定条件（`findings.iter().any(|f| f.severity == Severity::Error)`）を
変更してはならない。

#### 落とし穴 D：`WordContract` の `PartialEq` / キャッシュ

`WordContract` に理由フィールドを足すと `WordContractCacheKey` の意味に影響しないか
確認すること。**キャッシュキーには gap を含めない**（キーは identity とスキーマ版で決まる）。
`WordContract` 構造体自体に `Vec<GapCode>` を持たせるのが最も素直だが、
`PartialEq`/`Clone` の derive が壊れないよう `GapCode` にも derive を付けること。

### 3.5 手順

#### Step 3.1 — gap ID の定義

`rust/src/agent/contract_gap.rs`：

```rust
/// 契約推論が「証明しきれなかった」理由の安定 ID。
///
/// `LANG.CONTRACT.CHECK` の三値のうち "cannot verify" の内訳であって、
/// 4つ目の結果ではない。NIL の reason と同じ性格の識別子：
/// 人間向けの文言は変わりうるが、この ID は変わらない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum GapCode {
    UnresolvedWord,
    RecursiveDependency,
    DependencyUnknown,
    ConservativeSeed,
}

impl GapCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            GapCode::UnresolvedWord => "gap.unresolvedWord",
            GapCode::RecursiveDependency => "gap.recursiveDependency",
            GapCode::DependencyUnknown => "gap.dependencyUnknown",
            GapCode::ConservativeSeed => "gap.conservativeSeed",
        }
    }
}
```

#### Step 3.2 — 推論側で理由を記録する

`word_contract.rs`：

1. `WordContract` に `pub gaps: Vec<GapCode>` を追加（`ContractConfidence` の隣）。
   `Complete` のときは常に空。
2. `complete = false` の3箇所で、対応する `GapCode` を収集用の `Vec` に push する。
3. `WordContract::conservative()` は `gaps: vec![GapCode::ConservativeSeed]` を持つ。
4. `WordContract::identity()` は `gaps: Vec::new()`。
5. 依存語の contract を `widen_with` するとき、**依存語の gaps も合流させる**
   （不完全性は伝播する）。合流後は `sort()` して `dedup()` し、決定的な順序にする。

**完了条件**：`cd rust && cargo test --lib` が緑。

#### Step 3.3 — Finding に code を付ける

`contract_decl.rs`：

1. `DeclFinding` に `pub code: Option<&'static str>` を追加。
2. `check_one` の4箇所（arity / purity / nil / linearity）で、
   `conservative` が真なら `code: contract.gaps.first().map(|g| g.as_str())`、
   偽なら `code: None` を入れる。
   `gaps` が空なのに conservative という状態は起きないはずだが、
   起きたら `None` にして黙って進める（panic しない）。
3. パースエラー由来の Finding は `code: None`。
4. `to_json` の各 finding に `"code": f.code` を追加する。

#### Step 3.4 — 集計を出す

`ContractDeclCheck::to_json` に集計セクションを追加する：

```json
{
  "violated": false,
  "findings": [ ... ],
  "gapSummary": {
    "declarationsChecked": 12,
    "verified": 9,
    "cannotVerify": 3,
    "violated": 0,
    "byGap": { "gap.recursiveDependency": 2, "gap.unresolvedWord": 1 }
  }
}
```

- `declarationsChecked` は `#:contract` 宣言の総数
- `verified` は finding を1つも生まなかった宣言の数
- `byGap` のキーは `GapCode::as_str()`。**キー順は昇順に固定する**
  （`BTreeMap` を使う。`HashMap` は不可）

#### Step 3.5 — テスト

`rust/src/agent/` の既存テストモジュールの形に倣って、以下を追加：

| # | 名前 | 内容 |
| --- | --- | --- |
| 1 | `recursive_word_reports_recursive_gap` | 自己再帰する User 語に不成立な `#:contract` を付け、`code` が `gap.recursiveDependency` になる |
| 2 | `unresolved_word_reports_unresolved_gap` | 未定義語を呼ぶ本体で `gap.unresolvedWord` |
| 3 | `violated_declaration_has_no_gap_code` | 証明された違反（`Severity::Error`）の `code` が `None`（落とし穴 B） |
| 4 | `gap_summary_counts_add_up` | `verified + cannotVerify + violated == declarationsChecked` |
| 5 | `gap_summary_key_order_is_stable` | 同じ入力で2回生成した JSON 文字列が一致する |

### 3.6 受け入れ条件

- [ ] `cargo test --lib` / `cargo test --tests` が緑
- [ ] `clippy -D warnings` が緑
- [ ] `ajisai check <file> --contract --json` の `contractDecls` に `gapSummary` が出る
- [ ] `violated` の判定ロジックが変わっていない（落とし穴 C）
- [ ] gap ID が4種類ちょうど（落とし穴 A）
- [ ] `docs/dev/agent-cli-output-contract.md` に gap ID 一覧と、
      「これは cannot verify の内訳であって4つ目の結果ではない」旨が書かれている

### 3.7 コミット

```
Give every "cannot verify" a stable gap identifier

The contract check is deliberately conservative and partial, which the spec
states honestly — but an unnamed incompleteness cannot be measured or closed.
Inference goes conservative at exactly three sites in word_contract.rs plus one
conservative seed; naming each one turns the caveat into a countable backlog and
makes "verified 78.4%, dominated by gap.recursiveDependency" a release number.

Gap identifiers are the breakdown of "cannot verify", not a fourth outcome:
LANG.CONTRACT.CHECK's three results are unchanged, and a proven violation
carries no gap.
```

---

## Phase 4 — 三分法の統一（実装する）

### 4.1 目的

実行時の三分法（値 / NIL+理由 / ERROR）と、静的検査の三値
（verified / cannot verify / violated）は、**同じ三分法を時刻を変えて適用したもの**である。
`check --contract` の結果を、その対応が読み取れる形で出力する。

**これは装飾ではない。** 対応が本物であることの構造的な証拠が3つある：

1. **gap は伝播する。** Phase 3 で依存語の gaps を合流させるのは、
   `LANG.FAILURE.PASSTHROUGH`（NIL は理由を変えずに下流へ流れる）と同じ挙動である。
   意図して似せたのではなく、不完全性が伝播するという性質から独立に出てきた。
2. **「値」の場合は既に別ツールとして存在する。** `ajisai contract` /
   MCP の `infer_contracts` が返すのは推論された契約——つまり検査の成功結果そのもの。
   三分法は既に2つのツールに分裂して実装されている。
3. **gap ID は NIL reason と同じ性格の識別子である。** どちらも
   「整形式な部分操作が値を作れなかった理由」の安定 ID であり、
   人間向け文言とは独立に安定する。

### 4.2 スコープ：何をやり、何をやらないか（重要）

この Phase がやるのは **(a) だけ**である。

- **(a) 出力の語彙と構造を三分法に合わせる（やる）**
  `check --contract` が per-declaration の結果を `value` / `nil` / `error` として報告し、
  ファイル全体の判定をその畳み込みとして出す。意味論の変更なし、追加のみ。
  正典には「この三値は三分法の検査時刻への適用である」という*記述*を1段落足す。

- **(b) 検査器が実際に Ajisai の `Value` を返す（やらない）**
  gap ID を NIL reason のレジストリに合流させ、`NIL-REASON` で読めるようにする案。
  **技術的理由で見送る**：(b) の利益は「検査結果を Ajisai プログラムが加工できる」ことだが、
  それが意味を持つのは検査器自体が Ajisai で書かれたとき（セルフホスティング）であって、
  それは本ロードマップに無い。検査器が Rust のままなら (b) は
  「JSON に NIL 形のノードが入る」以上のものにならず、それは (a) が既に与える。
  さらに (b) は reason レジストリを実行時と検査時の2平面にまたがらせるため、
  `spec/words.json` の reason 語彙に触る。利益が来ていない段階で払うコストではない。

  → (b) は `docs/dev/` に「なぜ今やらないか」として記録する（Step 4.5）。
  セルフホスティングが議題に上がったとき、この判断は再検討に値する。

**この線引きは技術的判断として確定している。** (b) に手を出さないこと。

### 4.3 触ってよいファイル（ホワイトリスト）

```
編集: spec/language-semantics.md              （LANG.CONTRACT.CHECK に1段落）
編集: SPECIFICATION.html                      （生成物。生成コマンドの結果のみコミット）
編集: rust/src/agent/contract_decl.rs         （出力の追加）
編集: rust/src/agent/contract_gap.rs          （畳み込み規則）
編集: docs/dev/agent-cli-output-contract.md
新規: docs/dev/trichotomy-unification.md      （(b) を見送る理由の記録）
編集: docs/dev/INDEX.md
```

**Phase 3 の完了が前提。** gap ID が無いと `nil` の reason が書けない。

### 4.4 事前に読むファイル

| ファイル | 読む理由 |
| --- | --- |
| `spec/language-semantics.md` の `LANG.FAILURE.TRICHOTOMY` と `LANG.CONTRACT.CHECK` | 統一する2つの三分法の正典定義 |
| `spec/language-semantics.md` の `LANG.FAILURE.PASSTHROUGH` | gap の伝播がこれと同型であることの確認 |
| `docs/dev/ajisai-authoring-style.md` | 正典 HTML の執筆規約。§4.5 で従う |
| `rust/src/agent/contract_report.rs` の冒頭コメント | 「値」の場合が既にここにあることの確認 |
| `rust/src/error.rs` の `ErrorCategory` | **足さない**ことを確認するために読む（落とし穴 B） |

### 4.5 ⚠️ 落とし穴

#### 落とし穴 A：既存の `violated` と `findings` を消さない

追加のみが許される（`LANG.OBSERVATION.PROTOCOL`）。`violated` boolean と
`findings` 配列は**そのまま残す**。新しい `outcome` / `declarations` を足す。

一時的に同じ事実が2つの形で出ることになる。これはこのリポジトリが嫌う
「並行する定義」だが、**プロトコルの追加専用規律のほうが優先する**。
`agent-cli-output-contract.md` に「`findings` / `violated` は `declarations` の射影であり、
次の schemaVersion で削除される」と明記して、負債を可視化すること。
`SCHEMA_VERSION` はこの Phase では**上げない**。

#### 落とし穴 B：`ErrorCategory` に新しい variant を足さない

`rust/src/error.rs` の `ErrorCategory` は**実行時**エラーのレジストリである。
契約違反は実行時エラーではない。JSON には文字列 `"contractViolation"` を書くが、
**enum に variant を追加してはならない**。これがスコープ (a) と (b) の境界そのもの：
(a) は三分法の*語彙と構造*を使い、*レジストリは統合しない*。

#### 落とし穴 C：exit code と `violated` の判定を変えない

`check --contract` の終了コードは現行のまま：違反があれば 1、無ければ 0。
`Note`（cannot verify）は**決して失敗させない**。
`outcome` フィールドの追加が exit code に影響してはならない。
既存の CLI テストが緑のままであることで確認する。

#### 落とし穴 D：畳み込み規則は恣意的に決めない

ファイル全体の `outcome` は per-declaration の畳み込みだが、その順序は
`LANG.FAILURE` から**導出される**のであって、選ぶものではない：

- ERROR は伝播して評価を止める → 1つでも `error` があれば全体は `error`
- NIL は下流へ流れる → `error` が無く、1つでも `nil` があれば全体は `nil`
- どちらも無ければ `value`

コードのコメントに**この導出を書く**こと。「そう決めた」ではなく
「三分法からこうなる」と読めなければ、この Phase の意味が半分失われる。

#### 落とし穴 E：正典を「実行する」と読ませない

`LANG.CONTRACT.CHECK` は「without running the program」を明言している。
追加する段落は、三分法が**結果の分類**であって**機構の主張ではない**ことを
明示すること。ゼロ除算・パース失敗・範囲外添字はどれも機構が全く違うのに
同じ NIL という結末を共有している——検査の不決定をその一覧に加えても、
機構は何一つ変わらない。この論法を段落に含めること。

#### 落とし穴 F：`SPECIFICATION.html` を手で編集しない

生成物である。`spec/language-semantics.md` を編集し、
`npm run specification:generate` を実行し、その結果をコミットする。
`npm run specification:check` が緑になることで確認する。

### 4.6 手順

#### Step 4.1 — 正典に1段落足す

`spec/language-semantics.md` の `LANG.CONTRACT.CHECK` 節、第2段落の**後ろ**に、
以下の内容の段落を1つ追加する（既存の HTML 執筆規約に合わせること）。

内容（英語。文言はこの意味を保てば調整してよい）：

> These three results are the trichotomy of LANG.FAILURE.TRICHOTOMY applied at
> check time rather than at run time: verified corresponds to a value — the
> inferred contract itself — cannot verify to a reasoned absence, and violated
> to an error. The correspondence classifies outcomes, not mechanisms, and does
> not make the check evaluate the program: division by zero, a failed parse and
> an out-of-range index already share one outcome category while sharing no
> mechanism, and an inference that could not decide joins that list on the same
> terms.

**完了条件**：`npm run specification:generate && npm run specification:check` が緑。

#### Step 4.2 — 畳み込み規則の実装

`rust/src/agent/contract_gap.rs` に：

```rust
/// 宣言1つの検査結果。LANG.CONTRACT.CHECK の三値を、
/// LANG.FAILURE.TRICHOTOMY の語彙で表したもの。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckOutcome {
    /// verified — 推論が契約を出し、宣言と一致した。値は契約そのもの。
    Value,
    /// cannot verify — 整形式な検査が判定を出せなかった。理由は gap ID。
    Nil(GapCode),
    /// violated — 宣言が、証明された契約と矛盾する。
    Error,
}

impl CheckOutcome {
    pub(crate) fn as_str(self) -> &'static str { /* "value" / "nil" / "error" */ }
}

/// ファイル全体の判定。順序は選択ではなく LANG.FAILURE からの導出：
/// ERROR は伝播して評価を止めるので、1つでもあれば全体を決める。NIL は
/// 下流へ流れるので、ERROR が無いときにだけ全体を決める。どちらも無ければ値。
pub(crate) fn fold_outcomes(outcomes: &[CheckOutcome]) -> CheckOutcome;
```

`fold_outcomes` は空スライスに対して `Value` を返す（宣言が無いファイルは
検査すべきものが無く、それは不決定でも違反でもない）。

#### Step 4.3 — 出力の追加

`contract_decl.rs` の `ContractDeclCheck`：

1. per-declaration の結果を保持する `Vec<(String /*word*/, CheckOutcome)>` を持つ。
   `check_one` が finding を1つも出さなければ `Value`、`Note` を出せば
   その `code` から `Nil(gap)`、`Error` を出せば `Error`。
   1つの宣言が `Note` と `Error` の両方を出した場合は `Error`（畳み込み規則と同じ）。
2. `to_json` に追加（既存キーは残す。落とし穴 A）：

```json
{
  "violated": false,
  "findings": [ ... ],
  "gapSummary": { ... },
  "outcome": "nil",
  "declarations": [
    { "word": "INC",       "outcome": "value" },
    { "word": "NORMALIZE", "outcome": "nil", "reason": "gap.recursiveDependency" },
    { "word": "BAD",       "outcome": "error", "category": "contractViolation" }
  ]
}
```

- `declarations` の順序は `#:contract` の**ソース出現順**に固定する
- `reason` は `outcome == "nil"` のときだけ現れる
- `category` は `outcome == "error"` のときだけ現れ、値は常に `"contractViolation"`

#### Step 4.4 — テスト

| # | 名前 | 内容 |
| --- | --- | --- |
| 1 | `verified_declaration_is_a_value` | 成立する宣言1つ → `outcome: "value"`、`declarations` も `value` |
| 2 | `unverifiable_declaration_is_a_nil_with_a_reason` | 再帰語 → `nil` + `reason` が gap ID |
| 3 | `violated_declaration_is_an_error` | 証明された違反 → `error` + `category: "contractViolation"` |
| 4 | `error_dominates_nil_in_the_fold` | `error` と `nil` が混在するファイルの全体判定が `error` |
| 5 | `nil_dominates_value_in_the_fold` | `nil` と `value` の混在 → `nil` |
| 6 | `empty_declarations_fold_to_value` | 宣言ゼロのファイル → `value` |
| 7 | `outcome_does_not_change_the_exit_code` | `nil` のみのファイルで exit 0、`error` を含むファイルで exit 1（落とし穴 C） |
| 8 | `legacy_fields_still_present` | `violated` と `findings` が従来どおり出る（落とし穴 A） |

テスト7が最重要。**cannot verify が検査を失敗させないという既存の保証を、
この変更で壊さないこと。**

#### Step 4.5 — (b) を見送る理由の記録

`docs/dev/trichotomy-unification.md` を作り、以下を書く：

1. 対応が本物である3つの構造的証拠（§4.1）
2. (a) で何を実装したか
3. **(b) をなぜ今やらないか**（§4.2 の理由をそのまま。
   「人間が決めるべきだから」ではなく「セルフホスティングが来ていないから」と書くこと）
4. (b) を再検討すべき条件：検査器の一部でも Ajisai で書かれるとき
5. 概念数についての正直な記述：これは十概念を9にする変更**ではない**。
   #8 が三分法の第2定義を持つのをやめる、という**定義面の削減**である。
   「概念が減る」と書かないこと

`docs/dev/INDEX.md` に `[方針記録]` として追加。

### 4.7 受け入れ条件

- [ ] `npm run specification:check` が緑（`spec/` と生成 HTML が同期）
- [ ] `cargo test --lib` / `cargo test --tests` / `clippy -D warnings` が緑
- [ ] `ErrorCategory` に variant が増えていない（落とし穴 B）
- [ ] `SCHEMA_VERSION` が 1 のまま
- [ ] `violated` と `findings` が残っている
- [ ] exit code の挙動が変わっていない（テスト7）
- [ ] 畳み込み規則のコメントに `LANG.FAILURE` からの導出が書かれている（落とし穴 D）
- [ ] `docs/dev/trichotomy-unification.md` に (b) を見送る技術的理由がある
- [ ] `spec/words.json` に変更が無い

### 4.8 コミット

```
State the static check's three results as the runtime trichotomy

verified / cannot verify / violated stand in exact correspondence with
value / NIL-with-reason / ERROR, and the correspondence is structural rather
than decorative. Gap identifiers propagate through dependency inference exactly
as NIL reasons propagate downstream; the "value" case already exists as its own
tool, since `ajisai contract` returns the inferred contract that `check` asserts
against; and a gap identifier is the same kind of thing as a NIL reason — a
stable identifier for why a well-formed partial operation produced nothing.

The check still does not run the program. The trichotomy classifies outcomes,
not mechanisms: division by zero, a failed parse and an out-of-range index
already share one outcome category while sharing no mechanism.

This lands the reporting vocabulary and the fold rule only. Merging the gap
registry into the NIL reason registry is deliberately not done: its payoff
depends on the checker itself being written in Ajisai, which is not on the
roadmap, and until then it would widen the reason vocabulary across two planes
for no gain. See docs/dev/trichotomy-unification.md.
```

---

## Phase 5 — コスト契約（`cost` を `#:contract` の軸にする）

### 5.1 目的

`#:contract` に**コスト上限の宣言軸**を足し、実行前に検証する。
Ajisai の計量器は limb×limb 単位で**演算前に**課金するため、
コストは「測定値」ではなく「機械をまたいで再現する数」になり得る。
これは他言語が構造的に真似できない。

**この Phase は5つの中で最大。Phase 1・3 の完了を前提とする。**

### 5.2 触ってよいファイル

```
新規: rust/src/interpreter/word_cost.rs          （word_space.rs の骨格を複製）
新規: rust/src/interpreter/word_cost_tests.rs
編集: rust/src/interpreter/word_contract.rs      （CostBound フィールドの追加と join）
編集: rust/src/interpreter/mod.rs                （mod 宣言）
編集: rust/src/agent/contract_decl.rs            （`cost` 項のパースと検査）
編集: docs/dev/agent-cli-output-contract.md
新規: docs/dev/cost-contract-design.md
```

### 5.3 事前に読むファイル（この Phase は読む量が多い。順番も守ること）

| 順 | ファイル | 読む理由 |
| --- | --- | --- |
| 1 | `rust/src/interpreter/word_space.rs` **全体** | **この Phase の設計図そのもの**。`SpaceClass` / `SpaceBound` / `builtin_space` / `SpaceSim` の構造を時間側に複製する |
| 2 | `rust/src/interpreter/runtime_limits.rs` の 1〜130 行 | 計量器の単位（`work_limbs` / `binary_numeric_work` / `ALGEBRAIC_PAIR_UNITS`）と、なぜその単位なのか |
| 3 | `rust/src/agent/contract_decl.rs` の `parse_contract_directives` | `cost` 項を足す場所 |
| 4 | `rust/src/interpreter/interpreter_core.rs` の `ResourceUsage`（143〜163 行） | 実測側の3フィールド。宣言と実測が同じ単位で語られること |
| 5 | `docs/dev/collection-word-billing-2026-08-13.md` | collection 側の課金設計 |

### 5.4 ⚠️ 落とし穴

#### 落とし穴 A：`word_space.rs` の「never a false error」不変条件を必ず継承する

`word_space.rs` のモジュールコメントにある通り、推論が証明しきれないものは
**必ず緩い上限＋`exact = false`** に落ちる。`exact = false` のときは
宣言違反を `Error` にしてはならず、`Note`（cannot verify）にする。
Phase 3 の gap ID もここに乗せる。**この不変条件を破ると、
`check --contract` が偽陽性を出す道具になり、仕様違反になる**
（`LANG.CONTRACT.CHECK`：「a tool that reports verified for an unanalyzable body
is nonconforming」の裏返しとして、証明していない違反を報告してもならない）。

#### 落とし穴 B：多項式まで一気に行かない

`3n^2 + 7n + 2` のような厳密多項式は最終目標だが、**この Phase では実装しない**。
最初は `word_space.rs` と同じ**クラス**（`Const` / `Linear` / `Superlinear` / `Unbounded`）
だけを時間側にも持つ。多項式は別 PR。
理由：クラスの join は単調で健全性が示しやすいが、多項式の join は
係数の扱いを決める設計判断が要る。ここで判断してはならない。

#### 落とし穴 C：`ALGEBRAIC_PAIR_UNITS` は経験値である

`runtime_limits.rs` のコメントが正直に書いているとおり、この定数は
1台のコンテナで測った値であり、桁が合っていればよいという性格のもの。
**「コストは機械非依存である」と書くときは、「課金される単位数が機械非依存」であって
「実時間が機械非依存」ではないと明記すること。** ここを混同した記述をしてはならない。

#### 落とし穴 D：宣言の単位を実測と揃える

宣言できる軸は `ResourceUsage` の3つと1対1にする：
`executionSteps` / `numericWork` / `collectionWork`。
**`ResourceUsage` に無い軸を宣言可能にしてはならない**
（`interpreter_core.rs` 150行付近のコメント：「inventing fields for them would mean
reporting a number nothing measured」——同じ罠を宣言側でも踏まない）。

#### 落とし穴 E：ファイルサイズ

`word_space.rs` は 474 行。同じ骨格の `word_cost.rs` も 500 行に迫る。
**500 行を超えたら `word_cost.rs`（クラスと join）と
`word_cost_builtins.rs`（組み込み語の分類表）に分割する。**
最初から2ファイルで書き始めてよい。

### 5.5 手順

#### Step 5.1 — 設計メモを先に書く

`docs/dev/cost-contract-design.md` に、実装前に以下を書く：

1. 宣言できる3軸と、それぞれが `ResourceUsage` のどのフィールドに対応するか
2. クラス（`Const` / `Linear` / `Superlinear` / `Unbounded`）の定義と join 規則
3. `exact` フラグの意味と、`Error` を出せる条件
4. 「機械非依存なのは単位数であって実時間ではない」ことの明記（落とし穴 C）
5. 多項式版を今回やらない理由（落とし穴 B）

**完了条件**：この文書が存在し、`docs/dev/INDEX.md` に `[設計根拠]` として載っている。

#### Step 5.2 — `CostClass` と `CostBound`

`word_space.rs` の `SpaceClass` / `SpaceBound` を、**同じ構造・同じ join 規則**で
`word_cost.rs` に写す。名前は `CostClass` / `CostBound`。
`join` の実装（`max` を取り、同値なら `exact` を OR する）はそのまま。

軸が3つあるので、`CostBound` は3軸分を持つ：

```rust
pub(crate) struct CostBound {
    pub steps: (CostClass, bool),
    pub numeric: (CostClass, bool),
    pub collection: (CostClass, bool),
}
```

#### Step 5.3 — 組み込み語の分類表

`word_space.rs` の `builtin_space` に倣って `builtin_cost(id: WordId) -> CostBound` を書く。
分類の根拠は `runtime_limits.rs` の課金箇所。迷ったら**緩い側 + `exact = false`**
に倒す（落とし穴 A）。分類表には、なぜそのクラスなのかを1語ずつコメントで残すこと
（`word_space.rs` の書き方に倣う）。

#### Step 5.4 — シミュレーションと合流

`SpaceSim` に倣った `CostSim` を書き、`word_contract.rs` の推論ループの中で
`sim.feed_*` と同じ位置から呼ぶ。**推論ループを2回回さないこと**——
既存の1回の走査に相乗りする。

`WordContract` に `pub cost: CostBound` を追加する。

#### Step 5.5 — `#:contract` の `cost` 項

`parse_contract_directives` に構文を足す：

```
#:contract F ( 1 -- 1 ) pure nil-free cost steps=linear numeric=const collection=const
```

- `cost` の後は `axis=class` の並び。軸名は `steps` / `numeric` / `collection`。
- クラス名は `const` / `linear` / `superlinear` / `unbounded`。
- 書かれなかった軸は検査しない（既存の「省略された項は検査しない」規約と同じ）。
- 未知の軸名・クラス名は**パースエラー**にする（既存の `unknown term` と同じ扱い）。

`check_one` に `cost` の検査を足す：推論クラスが宣言クラスより**緩い**場合に指摘。
`exact == true` なら `Error`、`false` なら `Note` + gap ID（Phase 3）。

#### Step 5.6 — テスト

| # | 名前 | 内容 |
| --- | --- | --- |
| 1 | `const_word_verifies_const_cost` | 定数語に `cost steps=const` が verified |
| 2 | `map_is_unbounded_in_steps` | `MAP` を使う語は `steps=const` を宣言すると指摘される |
| 3 | `inexact_bound_is_a_note_not_an_error` | `exact = false` の経路では必ず `Note`（落とし穴 A の回帰テスト） |
| 4 | `unknown_cost_axis_is_a_parse_error` | `cost foo=const` がパースエラー |
| 5 | `omitted_axis_is_not_checked` | 軸を書かなければ何も指摘されない |
| 6 | `cost_join_is_monotone` | proptest：任意の join 順で結果が同じ（可換・結合的） |

テスト3が最重要。**偽陽性を出さないことがこの機能の生死を決める。**

### 5.6 受け入れ条件

- [ ] `cargo test --lib` / `cargo test --tests` / `clippy -D warnings` が緑
- [ ] `npm run check:file-size` が緑（落とし穴 E）
- [ ] 宣言可能な軸が `ResourceUsage` の3フィールドと1対1（落とし穴 D）
- [ ] `exact == false` の経路が `Error` を出さないことがテストで固定されている
- [ ] `docs/dev/cost-contract-design.md` に落とし穴 C の但し書きがある
- [ ] 多項式は実装していない（落とし穴 B）

### 5.7 コミット

```
Add a cost axis to #:contract, checked before execution

The work meter prices operations in limb-multiply units and charges before the
operation runs, so the unit count a program spends is a property of the program
rather than of the machine that ran it. That makes cost declarable and checkable
the way arity and purity already are.

This lands the coarse class lattice only (const/linear/superlinear/unbounded),
mirroring word_space.rs including its "never a false error" invariant: a bound
that is not provably attained yields a note, never a violation. Exact cost
polynomials are deliberately left for a separate change.
```

---

## 付録 A — Phase 間の依存関係

```
Phase 1 (observationDigest)  ── 独立。最初にやる
Phase 2 (semantics table)    ── 独立。Phase 1 と並行可
Phase 3 (gap ID)             ── 独立
Phase 4 (三分法の統一)        ── Phase 3 の gap ID を reason に使うので Phase 3 の後
Phase 5 (cost contract)      ── Phase 3 の gap ID を使うので Phase 3 の後
```

推奨順：**1 → 2 → 3 → 4 → 5**。1 と 2 は片方が詰まってももう片方を進められる。

## 付録 B — 各 Phase の想定規模

| Phase | 新規 Rust 行数（目安） | 新規 JS 行数 | 主なリスク |
| --- | --- | --- | --- |
| 1 | 250〜400（本体）+ 200〜300（テスト） | 0 | 落とし穴 A を外すと前提が壊れる |
| 2 | 0 | 200〜300 | 実行時間（CLI 1,642 回）と出力順の安定性 |
| 3 | 80〜150 | 0 | 既存の三値の意味を変えてしまうこと |
| 4 | 80〜150 + 150（テスト） | 0 | 既存の exit code / `findings` を壊すこと、スコープ (b) に手を出すこと |
| 5 | 500〜700（2ファイル）+ 300（テスト） | 0 | 偽陽性（`exact` の扱い） |

## 付録 C — 各 Phase の PR 説明に必ず含めること

- 何を追加したか（1段落）
- **何を追加していないか**、およびその技術的理由
  （Phase 4：reason レジストリを統合していないこと／Phase 5：多項式を実装していないこと）
- 保証の向きと、その残余（特に Phase 1：代数的数の `2^-512` 残余）
- 実行した検証コマンドとその結果

「未実装」を書くときは、**なぜ今やらないかを技術的理由で書く**こと。
「判断が必要だから」「人間が決めるべきだから」は理由ではない——
それは判断を先送りにした記録であって、判断の記録ではない。
