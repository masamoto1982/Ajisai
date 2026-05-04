# Three-Layer Documentation Implementation — Handover

最終更新: 2026-05-04
引き継ぎ元セッション: PR #850（マージ済み、`docs/dev/three-layer-documentation-model.md` を main に追加）
作業ブランチ: `claude/ajisai-documentation-framework-MQlGZ`

---

## 1. 目的

`docs/dev/three-layer-documentation-model.md` で合意された三層モデル（Reference / LOOKUP / Hover）を実装する。**仕様策定はすでに完了している**。本書は実装に必要な現状把握と着手手順だけを記す。

**まず読むべきもの**:
1. `docs/dev/three-layer-documentation-model.md` — 仕様（必読）
2. `docs/dev/reference-writing-style.md` — 既存規約（フェーズ3で改訂対象）
3. `SPECIFICATION.md` §6.5（sugar）と §7.14（contract metadata）

仕様書の §3.5（Role vs Behavior）、§4.3（hover_syntax の sugar 優先ルール）、§5.1（フィールド定義）、§8（フェーズ計画）が実装の基準。これらと矛盾する判断はしない。仕様に齟齬を見つけたら勝手に直さず、ユーザに上げる。

---

## 2. フェーズ計画（仕様書 §8 を実装に翻訳）

### Phase 1 — Hover の配線とフィールド名整理

**Rust 側**

- ファイル: `rust/src/builtins/builtin-word-definitions.rs`
- 現状の `BuiltinSpec`:
  ```rust
  pub struct BuiltinSpec {
      pub name: &'static str,
      pub category: &'static str,
      pub short_description: &'static str,   // ← rename to hover_summary
      #[allow(dead_code)]
      pub syntax: &'static str,              // ← rename to hover_syntax, drop #[allow(dead_code)]
      pub signature_type: &'static str,
      #[allow(dead_code)]
      pub detail_group: BuiltinDetailGroup,
      pub executor_key: Option<BuiltinExecutorKey>,
  }
  ```
- `builtin_spec!` マクロも追従して引数名を改名する。
- 既存の全 `BUILTIN_SPECS` エントリを仕様書 §4.3 の表に従って `hover_summary` / `hover_syntax` で書き直す。**注意**: 現行の `syntax` 値の多くは `". ADD → apply ADD to stack top"` のような説明文混じりで、仕様書が要求する「最短の使用例だけ」になっていない。全件レビューが必要。

**WASM 境界**

- 現状: `collect_builtin_definitions` と `collect_core_builtin_definitions` がタプル `(name, description, syntax, signature_type)` を返すが、3番目は `BUILTIN_SYNTAX_PLACEHOLDER`（コミット `6a21818` で per-word の syntax 例が剥がされた）。
- フェーズ1では **プレースホルダではなく `spec.hover_syntax` を返すように戻す**。そうしないと TS 側が hover_syntax を受け取れない。
- 関連ファイル: `rust/src/builtins/mod.rs:18` 周辺、`rust/src/wasm-interpreter-state.rs:145` のタプル形状コメント。

**TypeScript 側**

- `src/gui/vocabulary-state-controller.ts:254-271` — `wordData[1]`（description）が `button.title` に流れる経路、`wordData[2]`（syntaxExample）が `renderWordInfo(elements.builtInWordInfo, ...)` に流れる経路。タプル位置は変えず、Rust 側の値を `hover_summary` / `hover_syntax` に切り替えるだけで済む。
- `src/gui/dictionary-element-builders.ts:81-106` — `createWordButtonElement` の `title` 引数が `button.title` を直接設定している。変更不要。
- `wasm-interpreter-types.ts` / `wasm-interpreter-compat.ts` の型定義もタプル要素3を「hover syntax example, max ~40 chars」のコメントに更新。

**完了条件**: 各 built-in word をマウスオーバーすると、`button.title` に「`WORD — short verb phrase`」が、wordInfo 領域に最短の使用例（仕様書 §4.3 の表）が出る。

### Phase 2 — LOOKUP 本体

- 現状: `rust/src/builtins/builtin-word-details.rs::lookup_builtin_detail` はプレースホルダ文字列を返すだけ（32行）。`rust/src/interpreter/execute-lookup.rs::op_lookup` の built-in 分岐がここを呼ぶ。
- 仕様書 §3.4 のテンプレートを生成するレンダラに置き換える。
- そのために `BuiltinSpec` に以下を追加（仕様書 §5.1）:
  ```rust
  pub summary: &'static str,
  pub role: Option<&'static str>,
  pub syntax_forms: &'static [BuiltinSyntaxDoc],
  pub stack_effect: &'static str,
  pub behavior: &'static str,
  pub examples: &'static [BuiltinExampleDoc],
  pub failure: Option<&'static str>,
  pub side_effects: &'static [&'static str],
  pub modifier_interaction: Option<&'static str>,
  pub related: &'static [&'static str],
  pub stability: &'static str,
  ```
- `BuiltinSyntaxDoc` / `BuiltinExampleDoc` も同ファイルに新設。
- 既存全エントリにこれらフィールドを埋める。**英語のみ・ASCII・80列以内・プレーンテキスト**（仕様書 §3.3）。
- Role と Behavior は仕様書 §3.5 の規則で書き分ける。Role と Behavior が同じことを言いそうになったら立ち止まる。
- ユーザWord の `original_source` 経路（`execute-lookup.rs:26-43`）には**触らない**（仕様書 §3.2）。
- `stability` と SPECIFICATION.md §7.14 の整合性テストを `rust/src/interpreter/` 配下に追加（仕様書 §5.3）。`partiality` / `nil_policy` / `safety_level` のレジストリがどこにあるかは未調査。先にそれを探す。

**完了条件**: built-in を `?` LOOKUP すると §3.4 テンプレートに沿った構造化テキストがエディタに挿入される。プレースホルダ文言が消える。整合性テストが通る。

### Phase 3 — Reference サイト

- 現状: `public/docs/index.html` は GitHub への外部リンクだけのスタブ（23行）。
- `BuiltinSpec` に `concept: Option<&'static str>` を追加（仕様書 §5.1）。
- ビルド時に `BuiltinSpec` から `public/docs/words/<NAME>.html` を生成する仕組みを作る（言語選択は別途）。
- 概念ガイド・モジュールリファレンス・開発者ノートは手書き Markdown / HTML で `public/docs/` 配下に作る。`public/docs/index.html` をハブとして整理。
- `index.html:37-42` の Reference ボタンは既に `docs/index.html` を指しているので変更不要。
- **`docs/dev/reference-writing-style.md` を改訂する**（仕様書 §7）:
  - LOOKUP built-in 出力 → ASCII 英語プレーンテキスト
  - Reference サイト → Markdown、翻訳可
  - User-word LOOKUP → 据え置き
  - 「`detail-lookup-*.rs` の raw string literal」という記述は、`BuiltinSpec` フィールドからの生成方式に書き換える。

各フェーズは独立。Phase 1 だけでもリリース可。

### Phase 4 — Module words (将来)

Phase 1〜3 が対象とするのは canonical core words（`BUILTIN_SPECS` の登録語）のみ。
**module word（`'IO' IMPORT` 等で取り込まれる words）は当面三層モデルの対象外**。
理由はモジュールワードのラインナップが現在も流動的で、ドキュメント化のコストに対してリターンが低いため。

将来的にモジュールワードのラインナップが安定した時点で、本モデルをモジュールワードへも拡張する。具体的には:

- モジュール側のメタデータ（`rust/src/interpreter/modules/module_registry.rs`、`module_samples.rs`、`module_builtins.rs::module_word_description` 周辺）に `hover_summary` / `hover_syntax` および LOOKUP-tier フィールドを追加。
- `lookup_builtin_detail` と同等の renderer をモジュールワードにも適用し、`?` LOOKUP が built-in と同じ §3.4 テンプレートを返すようにする。
- Reference サイトのモジュールページ（仕様書 §2.1 項4）で同データを再利用する。

最終ゴールは「すべてのワード（canonical core / core-listed module / 一般 module）が三層モデルで統一的に扱われる」状態。

---

## 3. 既存コードの座標（実装前に頭に入れておく）

| 関心事 | ファイル / シンボル |
|---|---|
| ビルトイン仕様データ | `rust/src/builtins/builtin-word-definitions.rs::BuiltinSpec` / `BUILTIN_SPECS` |
| LOOKUP 本体（ビルトイン） | `rust/src/builtins/builtin-word-details.rs::lookup_builtin_detail` |
| LOOKUP ディスパッチ | `rust/src/interpreter/execute-lookup.rs::op_lookup` |
| WASM 越しの語彙列挙 | `rust/src/builtins/mod.rs`、`rust/src/wasm-interpreter-state.rs:145` |
| Hover ボタン生成 | `src/gui/dictionary-element-builders.ts::createWordButtonElement` |
| 語彙パネル描画 | `src/gui/vocabulary-state-controller.ts:240-336` |
| Reference ボタン | `index.html:37-42`（`docs/index.html` を開く） |
| Reference スタブ | `public/docs/index.html` |
| 既存執筆規約 | `docs/dev/reference-writing-style.md` |
| 三層モデル仕様 | `docs/dev/three-layer-documentation-model.md` |

---

## 4. 落とし穴・要注意

- **`syntax` のリネームは破壊的**。`#[allow(dead_code)]` が付いている＝Rust 内部では未使用だが、マクロ呼び出し側は全 BUILTIN_SPECS で引数を渡しているため、リネーム時は機械的に全置換が必要。
- **WASM タプル順は維持する**（`(name, description, syntax, signature_type)`）。タプル位置を変えると TS 側の `wordData[N]` を全部触ることになる。中身（プレースホルダ → 実値）だけ差し替える。
- **`hover_syntax` は必ず実例を入れる**（operands 込み）。仕様書 §4.3 で「real example, not a bare word」と明記。`DEF` のように sugar が無い語は canonical のままでよい（仕様書 §4.3 表）。
- **DEF / IMPORT のように sugar が無い語**は `Shorthand:` ヘッダ自体を出さない（仕様書 §3.6）。空の `Shorthand:` を残さないこと。
- **modifier_interaction** は標準動作の語では Optional を `None` にする。全語に書こうとしないこと（仕様書 §3.4 と書き手の判断）。
- **stability の整合性テスト**: §7.14 の contract metadata がどのファイル / 関数で実体化されているか先に把握すること。テストは「BuiltinSpec.stability と contract が矛盾しないこと」を assert する形が望ましい。
- **`原文ママ書き戻し` 警戒**: フェーズ1で `BUILTIN_SPECS` の `syntax` 列を一斉に書き換えるとき、現在の文字列（`. ADD → apply ADD to stack top` など）を流用してはいけない。仕様書 §4.3 の表に基づき作り直す。
- **言語ポリシー転換の宣言**: 既存の `reference-writing-style.md` は日本語Markdown許容。LOOKUP出力を英語ASCIIに変えるのは方針転換であり、フェーズ3で同文書を改訂しない限り規約間矛盾が残る。フェーズ2の成果物が「英語化された LOOKUP テキスト」になる時点で、`reference-writing-style.md` が嘘をつくことになる点をユーザに改めて確認してから本格着手するのが安全。
- **CI**: `cargo test` の `builtin_specs_*` 系テスト（`builtin-word-definitions.rs:807-839`）はリネーム後も通るはずだが必ず確認。

---

## 5. 着手手順（推奨）

1. ブランチ `claude/ajisai-documentation-framework-MQlGZ` をチェックアウトし `git pull` で最新化。
2. 仕様書 §8 のフェーズ1を、Rustリネーム → `BUILTIN_SPECS` 全件書き換え → WASM境界の値差し替え → TS型コメント更新、の順に小さなコミットで進める。
3. `cargo test` と `npm run typecheck`（あれば）を都度回す。
4. ブラウザで `npm run dev` 等を立ち上げ、辞書パネルで built-in をホバーして表示を目視確認（仕様書 §4 の通り）。
5. PR を draft で出し、ユーザに Phase 1 単独で取り込むか、Phase 2 まで含めて出すかを尋ねる。

ユーザは大きな変更は段階的に取り込みたい意向。**フェーズをまたぐ単一PRは作らない**。
