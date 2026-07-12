# 改修指示書: content-first な辞書名解決の実装

## 0. この指示書について

- **対象**: 新しいセッションで Ajisai のランタイム辞書解決を **content-first** 化する改修を行う。
- **正典**: `SPECIFICATION.html` のみ（§2.1）。本指示書は非正典の作業指示。
- **前提分析**: `docs/dev/archive/ajisai-dictionary-word-relationships-analysis.md`（同 PR）を必ず先に読むこと。
  本書はその §6〜§7 の設計判断を実装に落とすためのもの。
- **開発ブランチ**: 指示された feature ブランチで作業し、コミット・プッシュ後にドラフト PR を作る。
- **スコープ順序**: **まず content-first の仕組みを作る**。Reference（学習向け派生ドキュメント）への
  反映は**その後でよい**。ただし下記 §5 の「最小限の正典追記」は仕組みと同時に行う（正典は派生
  ドキュメントとは別で、挙動を定義する以上は conformance 上必須のため）。

---

## 1. 目的（何を直すか）

User Word 層で、ユーザーが名前の扱いに悩む2つの状況を解消する。

- **懸念A**: 複数 User 辞書に同名（例 `XXX@TEST` と `YYY@TEST`）があると、裸名 `TEST` が
  **黙って最古登録の辞書に解決**される（順番依存・無警告）。
- **懸念B**: Example をリネームせず再インポートすると、辞書内の名前→identity マップが
  **黙って張り替わる**。

これらの根因は「モデルは content-addressed（§8.6）なのに、ランタイム解決層が
名前 + `registration_order` だけで引いている（name-first）」という乖離。
解決層を identity 参照に拡張して content-first に揃える。

---

## 2. 現状の確証（コードの所在）

- `rust/src/interpreter/resolve_word.rs`
  - `resolve_short_name`（67–113 行）: 裸名解決の唯一の経路。順序は
    Core → インポート済み Module → 所有辞書（`owning_dictionary_context`, §8.6） →
    その他 User 辞書。最後の段（95–110 行）で同名複数なら `registration_order` 昇順で
    **最古を `Some` で返す**。← 懸念A の発生点。
  - `check_ambiguity`（115–134 行）: 「Ambiguous word ... Use a qualified path」用のパスを
    返すが、呼ばれるのは解決が `None` のときだけ（下記）。User 辞書に同名があれば必ず
    `Some` が返るため **事実上発火しない死にコード**。
  - `resolve_word_entry`（216–249 行）: `resolve_cache` 経由の高速路あり。曖昧名を
    単一解決でキャッシュしてはならない（§4 で対処）。
- `rust/src/interpreter/execute_builtin.rs`（67–81 行付近）: `execute_word_core_inner` が
  `resolve_word_entry` の `None` 時に `check_ambiguity` を見てエラー文を組む唯一の場所。
- `rust/src/interpreter/word_identity.rs`: §8.6 の content-addressing を**実装済み**。
  - `recompute_word_identities`（308 行）→ `self.word_identities: HashMap<fq名, id>` を構築。
  - `word_identity(fq_name)`（224 行）アクセサ。`body_store` が同一 body を `Arc` 共有。
  - 再計算は DEF（`execute_def.rs:207`）・DEL（`execute_del.rs:134`）・
    `rebuild_dependencies`（`resolve_word.rs:343`）後に走る → **quiescent 点で identity は新鮮**。
- `rust/src/wasm_interpreter_bindings/wasm_interpreter_state.rs`
  - `restore_user_words`（560 行）: バルク import。`defer_identity_recompute` で遅延し、
    最後に `rebuild_dependencies` で identity を一括再計算 → import 後も identity は新鮮。
  - `define_restored_words`（582 行）: 辞書名は `word.dictionary`（ファイル名由来）、
    既定 `EXAMPLE`。**現状 EXAMPLE の特別扱い（予約保護）は無い**。

---

## 3. 設計判断（実装の指針）

### 3.1 裸名の辞書間衝突は identity で判定する

最後の「その他 User 辞書」フォールバックで、同名マッチを集めて identity でグルーピングする。

- マッチが**単一の distinct identity**に収れんする（＝中身が同じ語） → **曖昧ではない。解決する**。
  表示名は安定のため最小 `registration_order` の fq 名を採用。
- distinct identity が **2 つ以上** → **真の曖昧**。黙って1つ返さず、曖昧エラーにする。
- すなわち **ambiguous = 名前の一致ではなく「内容の相違」**。

**保持すべき既存挙動（変更しない）**:

- Core / Module が裸名で勝つ順序（懸念の対象外。built-in は決して shadow されない）。
- `owning_dictionary_context` による自辞書優先（§8.6）。所有文脈が当該語を持つなら
  それで確定し、曖昧判定には進まない。識別グルーピングは**最終フォールバック段のみ**に適用。

### 3.2 EXAMPLE 予約保護は実装しない（重要）

content-first により再インポートは非破壊（旧 identity は `body_store` に残存・依存者は pin・
最悪でも名前 map の張り替えのみ）。破壊的上書きの危険が消えるため、予約保護の動機が無い。
むしろ「EXAMPLE だけ特別」は "名前より内容" の原則に逆行する。

→ **全 User 辞書を一様に扱う。EXAMPLE の特別扱い・リネーム必須化・ハードエラーは入れない。**
import は identity マージ（同一 body は `body_store` で dedup 済み）のまま。

任意（必須ではない）: 既存辞書名へ取り込んだ際に「N 件の名前を再ポイントした」程度の
**非ブロッキングな通知**を出してもよいが、EXAMPLE 固有にはしないこと。

### 3.3 確認事項（実装前に検証）

- バンドル Example が起動時に再シード可能か（`src/gui/example-words.ts` 系）。
  もし永続化のみで再シードされないなら、名前 map 張り替え後の復元手段（identity 経由 or
  再シード）が残るかを確認。content store に旧 identity は残るのでデータ消失ではないが、
  UX 上の復元経路を一応確かめる。ブロッカーではない。

---

## 4. 実装プラン

### 推奨アプローチ（最小変更・既存エラー文を再利用）

1. **共有ヘルパーを切り出す**（drift 防止）。`resolve_word.rs` に
   `resolve_user_bare(&self, upper: &str) -> UserBareOutcome` を新設し、
   `resolve_short_name` の最終フォールバックと `check_ambiguity` の両方から使う。
   ```text
   enum UserBareOutcome {
       None,                                   // どの User 辞書にも無い
       Unique(fq_name, Arc<WordDefinition>),   // 単一 identity に収れん
       Ambiguous(Vec<fq_name>),                // distinct identity が2つ以上
   }
   ```
   - 各マッチの identity は `self.word_identity(&fq)` で取得。
   - identity が `None`（未計算など）のマッチが混ざる場合は保守的に「distinct とみなさない
     /従来の最古登録で代表」にフォールバックし、その判断をコメントで明記。通常 quiescent 点
     では新鮮なので実害は出ない想定。
2. `resolve_short_name` の最終段を `resolve_user_bare` の結果で分岐：
   - `Unique` → `Some(...)` を返す。
   - `Ambiguous` → `None` を返す（解決失敗扱い）。曖昧の通知は呼び出し側のエラー路で行う。
   - `None` → 従来どおり `None`。
3. `check_ambiguity` を `resolve_user_bare` ベースに書き換え、`Ambiguous(paths)` のときのみ
   `paths` を返す。Core/Module ヒット時は空（従来どおり）。これで `execute_word_core_inner`
   の `None` 路で正しい曖昧エラー文が出る。
4. **resolve_cache の整合**:
   - 曖昧名（`Ambiguous`）は**キャッシュしない**こと（`store_resolve_cache` を呼ばない）。
   - `lookup_resolve_cache` の高速路（222–242 行）が、後から divergent な同名辞書が追加された
     ケースで stale な単一解決を復活させないことを確認。識別/依存の再計算（DEF/DEL/import 後の
     `rebuild_dependencies`）時に resolve_cache が無効化されるかを確認し、されていなければ
     同点で無効化する。
5. **呼び出し側の棚卸し**: `resolve_word`, `resolve_word_entry`, `resolve_word_entry_readonly`,
   `word_exists` の各呼び出し元を確認。曖昧名が `None`（＝未解決）として扱われても、
   ユーザー実行路では §step3 の曖昧エラーが出ること、内部判定（依存解決など）で誤って
   「存在しない」と誤認しないことを確認する。問題が出る箇所があれば下記の代替へ。

### 代替アプローチ（呼び出し側で漏れが出る場合）

`Option` ではなく `Resolution { Found, Ambiguous, NotFound }` を解決系の戻り値に導入して
明示的に伝播させる。変更点は増えるが曖昧の扱いが厳密になる。推奨路で call-site 監査が
重いと判明した場合に切り替える。

---

## 5. 正典（SPECIFICATION.html）への最小追記（必須・仕組みと同時）

挙動を定義する規則のため、Reference とは別に正典へ最小限追記する。場所は §8.6 の
「Name resolution for user words」付近、または §7.14 末尾の bare-name 解決規則の付近。

追記内容（趣旨）:

> 裸名が複数の User 辞書のエントリに一致する場合、それらが**同一の content identity** を
> 共有するなら同一の語として解決する。identity が相違するなら曖昧であり、修飾名
> （`DICT@WORD`）での指定を要する。Core / Module 語の解決順序はこれに優先し、影響を受けない。

> （補足）User 辞書は名前→identity マップであり、import は identity 単位のマージである。
> 同一定義は自動 dedup され、相違定義は新 identity として併存する。既存名への取り込みは
> 当該名の指す identity の張り替えであって、過去の identity・その依存者を破壊しない。

文体・節番号は周辺に合わせる。新規の語・型・プロトコルフィールドは増やさない。

---

## 6. テスト

### 追加（`rust/src/interpreter/dictionary_resolution_tests.rs`）

- **同一内容の辞書間衝突**: `XXX@TEST` と `YYY@TEST` を同一 body で定義 → 裸 `TEST` が
  エラーなく解決する（identity 一致）。
- **相違内容の辞書間衝突**: 異なる body で同名定義 → 裸 `TEST` は曖昧エラー（メッセージに
  両 fq 名）。`XXX@TEST` / `YYY@TEST` 修飾は解決する。
- 既存 `test_user_word_short_name_wins_*` 等が引き続き通る（単一 User 語の解決不変）。

### 追加/確認（`rust/src/interpreter/dictionary_operation_tests.rs`）

- import の identity dedup（同一定義は重複しない）。
- 同名辞書へ編集版を再 import → 名前は新 identity を指すが、旧 identity が
  `word_identity`/content store 上に残る（非破壊）ことを確認。

### 確認（`rust/tests/naming_resolution_laws.rs`）

- `core_word_is_not_shadowed_by_import` / `imported_module_shadows_user_word` が不変。

---

## 7. 検証コマンド

```
# Rust コア（解決ロジック・identity）
cargo test --manifest-path rust/Cargo.toml

# WASM 再ビルド（resolve 変更を GUI へ反映する場合）
npm run build:wasm

# TS 型・lint・JS テスト
npm run check
npm run lint
npm run test
```

`.githooks` と CI が要求する provenance/manifest 系（`npm run provenance:check`,
`word:manifest:check` 等）も、該当物を触った場合は走らせる。

## 8. 受け入れ基準

- 裸名の辞書間衝突: **内容一致 → 解決 / 内容相違 → 曖昧エラー（修飾を促す）**。
- `check_ambiguity` が identity ベースで実際に発火する（死にコードでなくなる）。
- import は非破壊・identity マージ。**EXAMPLE 特別扱いを追加していない**。
- 既存テスト全通過 + 上記新規テスト追加。
- 正典に §5 の最小追記。Reference 学習ドキュメントは未着手で可（別タスク）。

## 9. スコープ外（このタスクでやらない）

- Reference（`public/docs/` 等の学習向け派生ドキュメント）の整備。
- 用語の脱・多重定義（"Coreword" 改名など）。
- presentation 層（boundary 4 分類・category）の tag モデル化。
- `Resolution` enum への全面移行（推奨路で足りる限り）。
