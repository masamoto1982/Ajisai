# 批判的レビュー（2026-07）への妥当性評価と改修指示書

Status: `[提案・未実施]`。この文書は `docs/dev/` の設計メモであり、Ajisai の
意味論・互換性方針を定義しない。正典は `SPECIFICATION.html` のみである。記述が
`SPECIFICATION.html` と食い違う場合は正典に従う。

対象レビュー: 外部 LLM（ChatGPT）による Ajisai 批判的レビュー最新版。
評価基準コミット: `d3c6481`（`main`）。

先行文書 `docs/dev/external-evaluation-response-strategy.md` と同じ方針を踏襲する。
すなわち **外部評価は非正典の一次資料であり、コードで再現・確認できた主張だけを
改修根拠として採用する**。

---

## 0. 結論（要約）

レビューの主要な指摘は **おおむね妥当**である。特に P0 に挙げられた 2 件は、
本評価で **コードおよび実行によって再現確認できた実在の欠陥** である。

一方で、レビューには 1 件の事実誤認（CI 実行記録に関する推測）と 1 件の
ファイル配置の誤りがある。また、本評価の過程で **レビューが挙げていない
関連欠陥を 2 件新たに発見した**（§4）。

改修方針は次の 3 段構えとする。

| 段 | 内容 | 性質 |
| --- | --- | --- |
| R0 | 実在欠陥の修正（identity ハッシュ、SQRT 診断三軸） | 正しさ。即着手 |
| R1 | 再現性・検証基盤の締め直し（lockfile、CI strict、unsafe 検証） | 信頼基盤。R0 と並行可 |
| R2 | 対外表現の是正（algebraic closure、契約解析の主張範囲） | 文書のみ。安価 |
| R3 | 構造改革（span 付き AST/IR、語メタデータ単一スキーマ、conformance の外部化） | 大規模。R0–R2 完了後 |

レビューの中心的な処方箋 —— **新機能の追加ではなく、信頼基盤の縮小と意味論の
単一ソース化** —— に同意する。R3 着手までの間、新概念・新 Coreword・Tier 2 の
先行実装は凍結する（§7）。

---

## 1. 検証方法と検証範囲

本評価で **実際に実行した**こと:

- `cargo build --bin ajisai`（debug）— 成功。生成 CLI で意味論を直接プローブ。
- `python3 -m unittest test_ajisai`（`tools/ajisai-repro/`）— 9 tests, OK。
- ソース実測: `rust/src` = 76,277 行 / 264 ファイル。
- テスト実測: `rust/` 配下（`target/` 除く）で `#[test]` = 995、`proptest!` = 24。
- 語彙マニフェスト実測: `docs/word-manifest.json` = 224 項目
  （coreword 98 / moduleword 96 / alias 20 / surface_form 10）。
- GitHub Actions 実行履歴の照会（`test.yml` / `main`）。

本評価で **実行していない**こと（未確認事項として明示する）:

- `cargo test` 全量、`cargo clippy`、`cargo fmt --check` の再実行。
- `main` の branch protection / required checks 設定の直接確認（API 権限外）。
- WASM 境界テスト、差分比較器 `compare.py --conformance` の再実行。

---

## 2. 指摘別の妥当性評価

### 2.1 コードで確認できた指摘（採用）

| # | 指摘 | 判定 | 確認根拠 |
| --- | --- | --- | --- |
| P0-1 | 仕様は「cryptographic hash」と規定するが実装は暗号学的でない | **妥当** | `SPECIFICATION.html:1778` が "The identity is a cryptographic hash"。実装 `rust/src/interpreter/word_identity.rs:10-14` は "deterministic 256-bit-class polynomial hash"、`:28-35` に公開された 2 つの ~127bit 法と基数 257。`docs/dev/source-provenance-attestation-design.md:56-63` が自ら "not collision-resistant against an adversary" と明記 |
| P0-2 | 負数 `SQRT` が `divisionByZero` NIL を返す | **妥当（実行で再現）** | `rust/src/interpreter/interval_ops.rs:162-167` が `NilReason::DivisionByZero`。実測: `'MATH' IMPORT / -1 SQRT NIL-REASON` → `stack: NIL 'divisionByZero'` |
| A-1 | 契約チェッカーが「第二のインタプリタ」化 | **妥当（ただし配置に誤りあり）** | `word_contract.rs`(482) + `word_contract_lattice.rs` + `word_space.rs`(472) + `cli/contract_decl.rs` / `contract_space.rs` / `contract_linearity.rs` / `contract_report.rs`。保守的 fallback は `word_space.rs:89-118,147` で確認（`Unbounded, false` への退避）。→ 誤りは §3.2 |
| A-2 | span 付き AST / 型付き IR 層がない | **妥当** | `rust/src/types/mod.rs:599-612` の `Token` は位置情報を持たない平坦な enum、`ExecutionLine` は `Arc<[Token]>` のみ（`:614-617`）。`tokenizer.rs` に `span` の出現はゼロ |
| A-3 | 組み込み語メタデータが分散 | **妥当** | `builtins/` に 5 ファイル（`builtin_word_definitions.rs` / `_details.rs` / `_types.rs` / `_lookup_docs.rs`）＋ `coreword_registry.rs` ＋ `core_word_aliases.rs` ＋ `interpreter/modules/module_word_{docs,types}.rs` |
| A-4 | Minimal Core は監査面積を縮めていない | **妥当** | Minimal Core = 47 語（`docs/dev/ajisai-minimal-core-identity.md:167`）に対し、実処理系は 76,277 行 / 264 ファイル / 登録語 224。言語同一性の核と監査面積は別物、という指摘は正しい |
| L-1 | "algebraic closure" は数学的に不正確 | **妥当（内部不整合でもある）** | `README.md:11,54` と `SPECIFICATION.html:240` が "algebraic closure of SQRT under field arithmetic"。一方 `SPECIFICATION.html:684` は「\(D\) は `SQRT` について閉じて **いない**」と明記。正典が同一文書内で矛盾している |
| L-2 | Tier 2 が現在の説明を複雑化 | **妥当** | 現行 Coreword は Tier 2 を生成しないため、比較水位枯渇由来の `UNKNOWN` は現行語彙から到達不能。それでも中心説明に常駐している |
| L-3 | 文字列リテラルの終端規則が文脈依存 | **妥当** | `rust/src/tokenizer.rs:379-397`。`is_string_close_delimiter` が「次の文字が空白または特殊文字か」で終端を判定 |
| S-1 | shadow validation は UB の防護にならない | **妥当** | `rust/src/interpreter/parallel.rs:50` が `#![allow(unsafe_code)]`、`:184,186,213,215` の `unsafe impl Send/Sync`、`:250,346,352` の raw pointer 逆参照。結果一致検査は UB を反証しない、という指摘は正しい |
| S-2 | CI 品質ゲートが既定 advisory | **妥当** | `.github/workflows/test.yml:14-45`。`AJISAI_STRICT_QUALITY` 既定 `'false'` で fmt / clippy / coverage / `cargo-llvm-cov` install が `continue-on-error` |
| S-3 | 依存解決が再現可能でない | **妥当** | `.gitignore` が `rust/Cargo.lock` / `src-tauri/Cargo.lock` / `package-lock.json` を除外。CI は `npm install`（`npm ci` ではない） |
| S-4 | 参照実装の独立性が弱い | **妥当** | 同一リポジトリ・同一著者。Python 側の直接単体テストは実測 9 件。広域検証は `tests/conformance/index.html` 由来コーパス経由の `compare.py` に依存 |

### 2.2 規模に関する数値の照合

レビューの提示値はいずれも実測と一致した。数値の信頼性は高い。

| 項目 | レビュー | 実測 |
| --- | --- | --- |
| `rust/src` 行数 | 約 76,000 | 76,277 |
| `rust/src` ファイル数 | 264 | 264 |
| 語彙マニフェスト | 224 | 224 |
| `#[test]` | 約 995 | 995 |
| `proptest!` | 24 | 24 |
| Python 参照実装の単体テスト | 9 件通過 | 9 tests, OK |

---

## 3. レビューが不正確／要補足の点

### 3.1 CI 実行記録は「確認できない」のではなく、存在し成功している

レビューは「最新マージコミットについて workflow run や combined status が
確認できなかった」と述べる。これは **事実誤認**である。`Test Ajisai`
ワークフローは `main` 上で継続的に走っており、直近の実行は以下の通り成功している。

| head_sha | status | conclusion |
| --- | --- | --- |
| `d3c6481`（最新マージ） | completed | success |
| `44ced12` | completed | success |
| `f756501` | completed | success |

ワークフロー実行の累計は 3,385 件。したがって「CI が走っていない」という含意は
成り立たない。

ただし、これは S-2 の指摘を無効化しない。**走っていること**と
**通らなければマージできないこと**は別である。fmt / clippy / coverage が
既定で advisory である以上、成功記録は「blocking な検証を通過した」証拠に
ならない。required checks 化の必要性は依然として妥当である（→ R1-2）。

### 3.2 `contract_linearity.rs` の配置

レビューは `contract_linearity.rs` を契約解析の一部として `word_contract.rs` /
`word_space.rs` と並べて挙げるが、実際の配置は `rust/src/cli/contract_linearity.rs`
であり、`rust/src/interpreter/` 配下ではない。指摘の実質（契約解析が複数箇所に
散っている）は変わらないが、参照する際は配置を訂正すること。

### 3.3 契約解析の実サイズ

レビューはプロジェクト自身の見積り「契約・レジストリ系 約 5,400 行」を引くが、
契約解析中核 4 ファイル（`word_contract.rs` / `word_contract_lattice.rs` /
`word_space.rs` および各テスト）の実測は 2,751 行である。`cli/contract_*.rs` 群と
レジストリ系を合算すれば見積りに近づく。数値を引用する際は範囲を明示すること。

---

## 4. 本評価で新たに発見した事項（レビューにない）

### 4.1 負数 `SQRT` の誤りは reason だけでなく origin / recoverability に波及する

`Value::nil_with_reason` は `rust/src/types/value_operations.rs:12-24`
`absence_origin_for_reason` により reason から origin を **導出** する。
したがって誤った reason は診断三軸すべてを汚染する。実測:

```
'MATH' IMPORT / -1 SQRT NIL-REASON        => NIL 'divisionByZero'
'MATH' IMPORT / -1 SQRT NIL-ORIGIN        => NIL 'divisionByZero'
'MATH' IMPORT / -1 SQRT NIL-RECOVERABLE?  => NIL 'unknown'
```

`NIL-RECOVERABLE?` が `unknown` を返すのは、`nil_with_reason` が
`Recoverability::Unknown` を設定するためである。定義域外れは回復可能性が
確定している事象であり、`unknown` は情報の欠落である。修正は reason 追加だけでは
不十分で、origin と recoverability を同時に是正する必要がある（→ R0-2）。

### 4.2 正典自身が「domain miss」という語を用いている

`SPECIFICATION.html:684` は次のように述べる。

> `SQRT` of a negative rational is a **well-formed domain miss** → Bubble/NIL.

すなわち正典は既に「定義域外れ」という分類を持っており、実装側にだけ対応する
`NilReason` が存在しない。新設する variant 名は正典の語に合わせ
`NilReason::DomainMiss` / `AbsenceOrigin::DomainMiss` とするのが整合的である。
`NegativeRadicand` のような演算固有名より、`FLOOR` の非数値入力など将来の
定義域外れにも再利用できる。

### 4.3 `content_digest` の衝突は security だけでなく correctness の問題

レビューは P0-1 を主に攻撃者モデルの問題として論じるが、`content_digest` /
`body_content_key` の使用箇所を辿ると、衝突は **悪意がなくても誤実行に至る**
経路を持つ。

| 使用箇所 | 衝突時の帰結 |
| --- | --- |
| `rust/src/interpreter/execute_def.rs:139`（`body_content_key`） | 異なる本体が §8.6 content store で同一エントリを共有 → **別の語の本体が実行される** |
| `rust/src/cli/lockfile.rs:42`（`SourceEntry::new`） | 改変されたソースが `lock --check` を通過 |
| `rust/src/cli/receipt.rs:35,160` | 実行レシートの `sourceIdentity` / 結果 identity が別物と一致 |
| `rust/src/interpreter/resolve_word.rs:152`, `word_contract.rs:464-472` | 名前解決・契約継承が誤った依存を指す |

したがって P0-1 の対処は「仕様文言の削除」（レビューの選択肢 2）では閉じない。
**選択肢 1（標準暗号学的ハッシュへの移行）を採る**（→ R0-1）。

### 4.4 `divisionByZero` の origin が経路によって食い違う（要調査）

実測で次の不整合を確認した。

```
1 0 DIV NIL-REASON  => NIL 'divisionByZero'
1 0 DIV NIL-ORIGIN  => NIL 'executionFailure'
```

`absence_origin_for_reason` は `DivisionByZero => AbsenceOrigin::DivisionByZero`
と対応付けているにもかかわらず、`DIV` 経由の NIL は origin が
`executionFailure` になる。エラー projection 経路（Bubble Rule）が
`nil_with_reason` を経由していないためと推測されるが、本評価では完全には
追跡していない。**意図的な設計か欠陥かの裁定が必要**である（→ R0-2 の付随調査）。
`AbsenceOrigin::DivisionByZero` が実質的に SQRT 経路からしか生成されていない
（`rust/src/types/value_operations.rs:25` と `shadow_validation.rs:394` 以外に
生成箇所がない）ことも併せて確認すること。

---

## 5. 改修方針

### 5.1 採用する処方箋

レビューの中心的主張 —— 現在の課題は機能不足ではなく **信頼基盤の肥大** であり、
必要なのは意味論の単一ソース化と独立検証である —— を採用する。

### 5.2 優先順位の根拠

レビューの優先順位を概ね踏襲するが、以下の点で順序を変更する。

- **P0-1（ハッシュ）を最優先に据える**理由を security から correctness に
  組み替える（§4.3）。content store の本体共有は、攻撃者を仮定しなくても
  誤実行に至る唯一の経路であり、他のどの指摘よりも帰結が重い。
- **独立レビューの義務化（レビュー最優先 5）を R1 に降ろす**。これは
  プロセス変更であり、コード側の R0 を待たせる理由がない。並行して進める。
- **span 付き AST / IR（レビュー構造改革 1）を R3 に置く**。76,277 行の実装に
  対する最大級の変更であり、R0/R1 の検証基盤が締まる前に着手すると
  回帰の検出能力が不足する。

---

## 6. 改修指示

各フェーズは独立して着手・マージ可能である。フェーズ内の項目は
1 コミット 1 論点を原則とする。

### R0-1 — word identity を BLAKE3 へ移行する

**目的**: `content_digest` の衝突耐性を確立し、`SPECIFICATION.html:1778` の
「cryptographic hash」規定を実装が満たす状態にする。

**採用アルゴリズム: BLAKE3**（裁定済み。§8 の初版判断を撤回する）。

**依存フットプリントの実測**（本評価で `cargo add` + `cargo tree` により確認）:

| クレート | 実行時依存 | build 依存 | 備考 |
| --- | --- | --- | --- |
| `blake3` 1.8.5（`--no-default-features --features pure`） | 5（`arrayref` / `arrayvec` / `cfg-if` / `constant_time_eq` / `cpufeatures`） | 3（`cc` / `find-msvc-tools` / `shlex`） | `cc` クレート自体はコンパイルされるが、`pure` 下で C コンパイラは起動しない |
| `sha2` 0.11.0（`--no-default-features`） | 6（`cfg-if` / `cpufeatures` / `digest` / `block-buffer` / `hybrid-array` / `typenum`） | 0 | — |

すなわち実行時依存は **BLAKE3 のほうが少ない**。初版で BLAKE3 を
「依存が重い」として退けたのは誤りであり、撤回する。

**`pure` feature に関する重要な注意**: `pure` は **C／アセンブリのビルド経路を
外すだけであり、`unsafe` を除去しない**。x86_64 では blake3 が Rust の SIMD
intrinsics 実装（`rust_sse2.rs` / `rust_sse41.rs` / `rust_avx2.rs`、
合計約 150 箇所の `unsafe`）を選択する。`portable.rs` は `unsafe` 0 である。
crate 境界を越えるため `#![deny(unsafe_code)]` のリントには掛からないが、
**監査面積の問題として扱うこと**。

**変更対象**
- `rust/src/interpreter/word_identity.rs`（`poly_hash` / `content_digest` /
  モジュールヘッダのコメント `:10-14`）
- `rust/src/cli/lockfile.rs`（`LOCKFILE_VERSION`）
- `rust/src/cli/receipt.rs`（`RECEIPT_SCHEMA_VERSION`）
- `rust/Cargo.toml`

**手順**
1. `rust/Cargo.toml` に依存を追加する。監査面積を最小化する構成を推奨:

   ```toml
   blake3 = { version = "1", default-features = false, features = [
     "pure", "no_sse2", "no_sse41", "no_avx2", "no_avx512",
   ] }
   ```

   この構成では `portable.rs`（`unsafe` 0）のみが実行経路となる。
   `content_digest` の入力は語の本体とソースファイルであり性能上のホットパスでは
   ないため、SIMD を落とす代償は許容できる。ダイジェスト値は feature 構成に
   依存しないので、後から SIMD を有効化しても identity は変わらない。
   **`no_*` を付けるか（監査面積最小）、`pure` のみとするか（性能優先）は
   着手時に裁定し、理由を本文書に追記する**。
   本評価では両構成でビルドと公式テストベクタ通過を確認済み。
2. `content_digest` を BLAKE3 に差し替える。出力形式は `#` + 64 hex を維持
   （BLAKE3 の既定出力も 32 バイト = 64 hex なので外形は不変）。
3. `poly_hash` / `ID_PRIME_A` / `ID_PRIME_B` / `ID_BASE` を削除する。
   `hash.rs` の他用途で使われている場合はそちらへ局所化し、identity 系からは
   完全に切り離す。
4. `LOCKFILE_VERSION` を 1 → 2、`RECEIPT_SCHEMA_VERSION` を 1 → 2 に上げる。
   旧バージョンの lock / receipt は **黙って不一致にせず、
   「identity アルゴリズムが変わったため再生成が必要」と明示するエラー** に
   落とすこと。
5. lockfile / receipt のスキーマに identity アルゴリズム名を持つフィールドを
   追加する（例: `"identityAlgorithm": "blake3"`）。将来の再移行を
   バージョン番号だけに依存させない。
6. `word_identity.rs:10-14` のコメントから "may be replaced by a standard
   cryptographic hash" の予告文を削除し、実際のアルゴリズム（BLAKE3、
   採用した feature 構成を含む）を記述する。
7. `docs/dev/source-provenance-attestation-design.md:54-63` の
   「Why SHA-256 and not the §8.6 polynomial digest」節を改訂する。移行後は
   §8.6 側も暗号学的ハッシュになるため、「多項式ハッシュは衝突耐性がないから
   provenance には使えない」という現行の論拠が成立しなくなる。節の題と本文を
   「provenance は Node 側で完結する別実装であり、依存ゼロの `node:crypto`
   SHA-256 を用いる。identity（Rust 側 BLAKE3）とは用途も実装言語も異なる」
   という論拠へ書き換えること。

**受け入れ条件**
- `-` BLAKE3 公式テストベクタ（`test_vectors.json` 由来。最低でも空入力・
  `"abc"`・ブロック境界をまたぐ長さの 3 ケース）に対する単体テストが存在し
  通過する。本評価では `"abc"` →
  `6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85`
  を確認済み。
- `-` `cargo test` 全量が通過する（identity 文字列をハードコードした
  既存テストの期待値更新を含む）。
- `-` `npm run provenance:check` が通過する（ソース変更後の再 attest 込み）。
- `-` `cargo check --features wasm --target wasm32-unknown-unknown` が通過する。
  本評価で `blake3` 単体の `wasm32-unknown-unknown` ビルドは両 feature 構成で
  確認済みだが、`ajisai-core` に組み込んだ状態での確認は R0-1 着手時に行うこと。
- `-` 旧 `ajisai.lock` に対する `ajisai lock --check` が、意味不明な不一致では
  なく明示的な「再生成が必要」メッセージで失敗する。
- `-` `SPECIFICATION.html:1778` の文言変更は **不要**であることを確認する
  （実装が仕様に追いつく方向の修正であるため）。

**非目標**
- identity の意味論（何を正規化して何を含めるか）の変更。ハッシュ関数の
  差し替えのみを行う。
- `scripts/generate-source-attestation.mjs` の SHA-256 を BLAKE3 に揃えること。
  provenance 系は Node 組み込みの `node:crypto` で完結しており依存ゼロで
  動いている。両者が別のハッシュ族になっても、それぞれが暗号学的に健全で
  あれば問題はない。統一は R0-1 の目的ではない。ただし
  `source-provenance-attestation-design.md` の該当節（手順 7）は、
  「多項式ハッシュとの対比」から「同じ security-grade だが別の用途・別の
  実装言語」へ論拠を書き換えること。

### R0-2 — 定義域外れの NIL 診断を是正する

**目的**: `-1 SQRT` が返す構造化診断を、正典 `SPECIFICATION.html:684` の
「well-formed domain miss」と一致させる。

**変更対象**
- `rust/src/error.rs`（`NilReason` enum `:6-41`、protocol 文字列 `:101` 付近）
- `rust/src/semantic/absence.rs`（`AbsenceOrigin` enum `:5-31`）
- `rust/src/semantic/protocol.rs`（`:76` 付近）
- `rust/src/types/value_operations.rs`（`absence_origin_for_reason` `:12-24`）
- `rust/src/interpreter/interval_ops.rs`（`op_sqrt` `:162-167`）
- `rust/src/interpreter/execution_loop.rs`（`error_category_for_nil_reason` `:92-107`）

**手順**
1. `NilReason::DomainMiss` を追加する。protocol 文字列は `"domainMiss"`。
2. `AbsenceOrigin::DomainMiss` を追加し、`absence_origin_for_reason` に
   `DomainMiss => DomainMiss` を追加する。
3. `interval_ops.rs:162-167` の `NilReason::DivisionByZero` を
   `NilReason::DomainMiss` に差し替える。同時に recoverability を
   `Recoverability::Unknown` から適切な値へ確定させる
   （定義域外れは入力を変えれば解消するため `Recoverable` が妥当。
   最終判断は着手時に裁定し理由を記録すること）。
4. `error_category_for_nil_reason` に `DomainMiss` の分岐を追加する
   （`Some(ErrorCategory::Custom)` が妥当かは §11.2 Bubble Rule と照合）。
5. `NilReason` は網羅 match が複数箇所にあるため、コンパイルエラーが出る全箇所を
   潰す。`rust/src/semantic/protocol_string_tests.rs` の protocol 文字列一覧も
   更新する。
6. **付随調査**: §4.4 の `1 0 DIV NIL-ORIGIN => 'executionFailure'` を追跡し、
   意図的か欠陥かを裁定する。欠陥であれば同フェーズ内で修正し、意図的であれば
   `absence.rs` の `AbsenceOrigin::DivisionByZero` のコメントに
   「どの経路が生成するか」を明記する。
7. 仕様側の例・`SKILL.md` 生成入力に `-1 SQRT` の診断を示す箇所があれば更新する
   （`npm run check:skill` が検出する）。

**受け入れ条件**
- `-` 実測で次が成り立つ:
  `'MATH' IMPORT / -1 SQRT NIL-REASON` → `'domainMiss'`、
  `NIL-ORIGIN` → `'domainMiss'`、`NIL-RECOVERABLE?` が `'unknown'` 以外。
- `-` `1 0 DIV NIL-REASON` は `'divisionByZero'` のままであること（回帰なし）。
- `-` 上記 3 つを直接検証する `#[test]` が追加されていること。
- `-` `python3 tools/ajisai-repro/compare.py --conformance` が通過する
  （Python 参照実装側の同期修正を含む）。
- `-` `npm run word:manifest:check` / `npm run check:skill` が通過する。

### R1-1 — 依存解決を再現可能にする

**変更対象**: `.gitignore`, `.github/workflows/*.yml`

**手順**
1. `.gitignore` から `rust/Cargo.lock`、`src-tauri/Cargo.lock`、
   `package-lock.json` の 3 行を削除する。
2. 3 ファイルをコミットする。
3. CI の `npm install` を **すべて** `npm ci` に置換する
   （`test.yml` の `quality-gate` / `typescript-check` の 2 箇所、
   および deploy 系ワークフローに同様の記述があれば併せて）。
4. `actions/cache` の key が `hashFiles('**/Cargo.lock')` を参照している
   （`test.yml` の `rust-test` job）。lockfile がコミットされることで
   この key が初めて意味を持つようになるため、キャッシュ挙動を確認する。

**受け入れ条件**
- `-` 同一コミットからの 2 回のクリーンビルドが同一依存バージョンを解決する。
- `-` CI が `npm ci` で通過する（`package-lock.json` と `package.json` の
  整合が取れていること）。

**非目標**
- SBOM 生成とリリース成果物への dependency digest 添付。レビューは併記するが、
  lockfile のコミットが前提条件であり、別フェーズとする。

### R1-2 — CI 品質ゲートを blocking にする

**変更対象**: `.github/workflows/test.yml:14-45`、リポジトリ設定

**手順**
1. リポジトリ変数 `AJISAI_STRICT_QUALITY` を `true` に設定する。
   ただし **設定前に `cargo fmt --check` と `cargo clippy --all-targets --
   -D warnings` をローカルで通し、既存の違反をすべて解消しておく**こと。
   違反が残ったまま strict 化すると `main` がブロックされる。
2. 違反解消後、`test.yml:14-16` の既定値を `'false'` → `'true'` に変更する。
   段階導入は目的を達したため、変数による切り替え機構自体を削除してよい。
   その場合 `continue-on-error` の行と末尾の "Quality gate mode summary" step も
   併せて削除する。
3. `main` に branch protection を設定し、required checks として少なくとも
   以下を指定する: `Quality Gate` / `Rust Tests` / `TypeScript Check` /
   `Reference Interpreter Differential` / `WASM Boundary Tests`。
4. "Detect stale committed wasm bundle" は意図的な advisory であり、
   `continue-on-error: true` を維持してよい（`test.yml` の当該 step の
   コメントに理由が記載済み）。

**受け入れ条件**
- `-` fmt / clippy 違反を含む PR が実際にブロックされることを確認する。
- `-` required checks の設定内容を本文書に追記する。

### R1-3 — unsafe 並列経路の検証を強化する

**目的**: `rust/src/interpreter/parallel.rs` の unsafe island について、
「結果一致検査」ではなく **UB そのものを対象とする検証** を持つ。

**手順**
1. **まず safe rewrite の実現可能性を再評価する**。`parallel.rs:33-49` の
   モジュールコメントは scoped thread のオーバーヘッドを理由に unsafe を
   選択したと記録しているが、この判断の根拠となった計測が現在も有効かを
   `bench/` で再確認する。`std::thread::scope`（Rust 1.63+）/ rayon /
   crossbeam のいずれかで許容範囲に収まるなら、unsafe を削除する。
2. safe rewrite が不可なら、CI に以下を追加する（いずれも新規 job、
   まず advisory で導入し安定後 blocking 化）:
   - `cargo +nightly miri test`（対象を並列モジュールに絞る）
   - `RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test`（ThreadSanitizer）
   - `loom` による並行性モデル検査（対象を絞った専用テスト）
3. `parallel.rs` への変更を独立レビュー必須とする（→ R1-4）。

**受け入れ条件**
- `-` safe rewrite を採った場合: `#![allow(unsafe_code)]` が crate から消え、
  `#![deny(unsafe_code)]` が例外なく成立する。性能回帰は
  `scripts/compare-perf.sh` の基準内。
- `-` sanitizer 導入を採った場合: Miri または TSan の job が CI に存在し、
  並列経路のテストを実際に実行している（skip されていない）ことを
  ログで確認できる。

### R1-4 — 高リスク変更に独立レビューを要求する

**目的**: 「AI が実装し、AI が説明を書き、同じ作者が自己レビューし、
同じリポジトリの参照実装で確認する」閉ループを断つ。

**手順**
1. `CODEOWNERS` を新設し、以下のパスに独立レビュアーを割り当てる:
   - `SPECIFICATION.html`
   - `rust/src/types/fraction.rs` ほか数値正規形・比較系
   - `rust/src/interpreter/word_identity.rs`
   - `rust/src/interpreter/parallel.rs`
   - `rust/src/types/value_protocol.rs` ほかシリアライズ形式
   - capability 境界（`rust/src/interpreter/host.rs` ほか）
   - 最適化経路（`rust/src/elastic/`、`compiled_plan.rs` ほか）
2. branch protection で当該パスへの変更に review 必須を設定する。
3. 独立レビュアーが確保できない場合の代替として、**同一 PR 内での
   自己承認を禁止し、最低 24 時間の cooling period を置く**運用を
   本文書に明記する。実効性は劣るが、無レビューよりは良い。

**受け入れ条件**
- `-` `CODEOWNERS` がコミットされ、branch protection と連動している。
- `-` 代替運用を採る場合、その内容が本文書に追記されている。

### R2-1 — "algebraic closure" 表現を是正する

**目的**: 正典内部の矛盾（`SPECIFICATION.html:240` と `:684`）を解消する。

**手順**
1. `SPECIFICATION.html:240` の "the algebraic closure of `SQRT` under field
   arithmetic" を、正確な表現に置換する。推奨:
   *the multiquadratic field generated over ℚ by square roots of non-negative
   rationals*（日本語版があれば「非負有理数の平方根で ℚ 上に生成される
   多重二次体」）。
2. `README.md:11` と `README.md:54` の同一表現を同様に置換する。
3. `SPECIFICATION.html:672,684` の記述は既に正確なので変更しない。
4. `docs/` 配下の他文書に同表現がないか `rg 'algebraic closure'` で確認する。

**受け入れ条件**
- `-` `rg -i 'algebraic closure'` の結果が空、または「この語を避ける理由」を
  説明する箇所のみになる。
- `-` `npm run check:semantic-firewall` が通過する。

### R2-2 — 契約解析の主張範囲を正確に記述する

**目的**: 契約機構を「実行前検証」ではなく「限定された構文範囲に対する
保守的な部分検証」として説明する。

**手順**
1. `rust/src/interpreter/word_space.rs:89-118` が `Unbounded, false` へ退避する
   構成（高階実行、`COND`、`EXEC`/`EVAL`、`SPAWN` 系、未解決依存）を列挙し、
   「解析可能な範囲」と「note に退避する範囲」を明示した表を作る。
2. その表を `docs/dev/space-contract-design.md` に追加する。
3. `README.md` および `SPECIFICATION.html` の契約関連記述で、
   「検証済み」「保証」に相当する語を「保守的に推論できた範囲で」に
   相当する表現へ改める。**保守的 fallback 自体は正しい設計であり、
   変更しない**。変えるのは説明のみ。

**受け入れ条件**
- `-` 退避条件の表が実コードの match 分岐と 1:1 で対応している。
- `-` 対外文書に「実行前に完全検証される」と読める記述が残っていない。

### R2-3 — Tier 2 の記述を現在／将来に分離する

**手順**
1. `README.md` と `SPECIFICATION.html` の数値関連記述を
   「現在: 有理数＋多重二次体。比較は常に決着し、budget も `UNKNOWN` も
   関与しない」と「将来: 一般計算可能実数では `UNKNOWN` の可能性」に
   節レベルで分離する。
2. 比較水位・観測 budget の説明を「将来」節へ移す。現行語彙から到達不能である
   ことを明記する（`SPECIFICATION.html:684` が既に根拠）。
3. `docs/dev/tier2-vocabulary-phase7-design.md` は設計文書として維持する。

**受け入れ条件**
- `-` 初学者向け経路（README → SKILL.md）に Tier 2 由来の概念が現れない。

### R3-1 — span 付き AST と型付き IR を導入する

**着手条件**: R0 と R1 の完了。

**目的**: `Token` 列の再解釈による解析を廃し、実行器と静的解析器が同一の
中間表現を共有する構造へ移行する。

**段階**
1. **Phase A — span の付与のみ**。`Token` に `span: (usize, usize)` 相当を
   持たせ、`tokenizer.rs` が付与する。この段階では意味論を一切変えず、
   診断のソース位置精度だけが上がる。回帰検出は既存 995 テストで足りる。
2. **Phase B — AST の導入**。`ExecutionLine`（`rust/src/types/mod.rs:614-617`）の
   `Arc<[Token]>` を構造化 AST に置換する。既存の実行経路は AST から
   Token 列を再生成する互換層を通す。
3. **Phase C — 型付き IR と transfer function の共有**。契約解析
   （`word_space.rs` / `word_contract.rs` / `cli/contract_*.rs`）を IR 上の
   transfer function に書き直し、実行器と同じ関数を参照させる。
   ここで初めて「第二のインタプリタ」問題が解消する。

**受け入れ条件（各 Phase 共通）**
- `-` `compare.py --conformance` が通過する（Python 参照実装との差分ゼロ）。
- `-` `scripts/compare-perf.sh` が基準内。
- `-` `npm run check:file-size` が通過する（§14.1 の 500 行予算）。

**注意**: Phase C は契約解析の対外的な能力を変えうる。R2-2 で正確化した
主張範囲を、Phase C 完了後に再度更新すること。

### R3-2 — 組み込み語メタデータを単一スキーマへ統合する

**目的**: 「重複を残してテストで同期する」構造を「単一宣言から生成する」構造へ
置き換え、整合性テストを生成器の検査に集約する。

**手順**
1. 単一宣言スキーマを定義する。1 語あたり最低限: canonical name, aliases,
   executor key, stack effect / mass, purity, determinism, NIL policy,
   capability, space class, documentation, examples。
2. 生成対象: `builtin_word_definitions.rs` / `builtin_word_details.rs` /
   `builtin_word_types.rs` / `builtin_word_lookup_docs.rs` /
   `coreword_registry.rs` / `core_word_aliases.rs` /
   `modules/module_word_docs.rs` / `modules/module_word_types.rs` /
   `docs/word-manifest.json` / dispatch skeleton。
3. 既存の整合性テスト（`builtin_word_details_tests.rs` ほか）は、
   生成物同士の突き合わせから **生成器そのものの検査** へ移す。
4. 語を 1 つ追加する作業が「スキーマに 1 エントリ追加 → 生成 → dispatch 本体を
   書く」の 3 手で完了することを確認する。

**受け入れ条件**
- `-` `npm run word:manifest:check` が生成器経由で通過する。
- `-` 語の追加漏れ（`SUPERVISE` 事例）が構造的に起こり得ないこと
  —— 単一スキーマに無い語は生成物にも現れない —— を実演する。

**非目標**
- 語の意味論変更。純粋に定義の所在を移すリファクタリングとする。

### R3-3 — conformance corpus を実装から独立させる

**目的**: 参照実装と本実装が同じ誤解を共有するリスクを下げる。

**手順**
1. `tests/conformance/index.html` のケース群を、実装リポジトリから独立して
   参照できる **バージョン固定された成果物** として切り出す
   （最低でも別ディレクトリ＋バージョンタグ、望ましくは別リポジトリ）。
2. 各ケースに `SPECIFICATION.html` の節番号を紐付ける
   （`docs/quality/TRACEABILITY_MATRIX.md` の枠組みを流用可能）。
3. ランダムな適格プログラム生成器を追加し、`compare.py` の入力に加える。
4. 代数的 metamorphic test（可換性・結合性・恒等元など。
   `rust/tests/algebraic_laws.rs` の資産を流用）を差分比較に組み込む。
5. native / WASM / Python の三者比較へ拡張する
   （現在は native CLI と Python の二者）。

**受け入れ条件**
- `-` conformance corpus が実装コミットとは独立にバージョン付けされている。
- `-` 三者比較が CI で blocking として走る。

---

## 7. 当面凍結するもの

R0–R2 が完了するまで、以下を凍結する。レビューの「当面抑制すべきもの」を
そのまま採用する。

- 新しい中心概念・新しい比喩の導入
- 新しい契約軸の追加（現在の 4 軸で止める）
- 新しい Coreword の追加
- Tier 2 の先行実装

例外は、R0–R3 の受け入れ条件を満たすために必要な変更のみとする。

---

## 8. 不採用・保留とした指摘

| 指摘 | 判断 | 理由 |
| --- | --- | --- |
| 文字列リテラルを escape / raw string / 長さ付きへ移行（L-3） | **保留** | 指摘自体は妥当（`tokenizer.rs:379-397`）だが、これは表層構文の破壊的変更であり、既存の全 conformance ケース・例・`SKILL.md` に波及する。R3-1 Phase A（span 付与）完了後に、formatter とエディタ支援の要求と併せて再検討する |
| word identity の「cryptographic」を仕様から削除する（P0-1 の選択肢 2） | **不採用** | §4.3 の通り、content store 経由で誤実行に至る経路があるため、文言の後退では閉じない。選択肢 1 を採る |
| BLAKE3 の採用 | **採用**（初版の不採用を撤回） | 初版は「依存が重い」として退けたが、実測すると実行時依存は `blake3`（5 クレート）＜ `sha2`（6 クレート）であり、前提が誤っていた。§6 R0-1 参照 |
| Minimal Core 概念の撤回 | **不採用** | レビューは「監査面積を縮めていない」と正しく指摘するが、言語同一性の定義としては有効である。撤回ではなく、`docs/dev/trusted-core-size-assessment.md` の「懸念は実務的に解消された」という主張を「言語同一性の核は 47 語だが、メモリ安全・値正しさの監査面積は 76,277 行のままである」と改める（R2 に含める） |
| SBOM とリリース成果物への dependency digest | **保留** | R1-1（lockfile コミット）が前提。完了後に別途起票 |

---

## 9. 改訂履歴

| 日付 | 内容 |
| --- | --- |
| 2026-07-24 | 初版。ChatGPT 批判的レビュー最新版に対する妥当性評価と R0–R3 の改修指示を記載 |
| 2026-07-24 | R0-1 の採用アルゴリズムを SHA-256 から **BLAKE3** に変更。初版の「BLAKE3 は依存が重い」という不採用理由は実測（`blake3` 実行時依存 5 < `sha2` 6）により誤りと判明したため撤回。`pure` feature が `unsafe` を除去しないこと、および `no_sse*`/`no_avx*` を併用すれば `portable.rs` のみ（`unsafe` 0）になることを追記 |
