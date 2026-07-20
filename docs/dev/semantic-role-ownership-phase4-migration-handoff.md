# Phase 4 残移行 作業引継書（semantic role single authority）

Status: work order（non-canonical）。正典は `SPECIFICATION.html` のみ。この文書は実装作業の指示書であり、
Ajisai の意味論を定義しない。矛盾する場合は `SPECIFICATION.html` に従う。

前提メモ: `docs/dev/semantic-role-ownership-phase4-design.md`（候補案 A/B/C の比較と 4A–4G の段階計画）。
本引継書はその設計メモを踏まえ、**残作業（4C–4G）を新セッションで完遂するための具体的な調査結果・設計・
手順・検証ゲート**をまとめる。前セッションでコード変更は行っていない（調査のみ）。

## 0. これは何のための作業か

Phase 4（意味役割の二重管理解消、handoff §11）の**受け入れ条件のうち 2 つが未達**である。

現状は機能的には正しく、全テスト green（`cargo test --all-targets` 全 binary ok、semantic-firewall pass、
conformance/差分 CI green）。したがって**バグ修正ではなく設計負債の解消**であり、急いで壊す価値はない。
段階的・検証優先で進めること。

### 未達の受け入れ条件（handoff §11.6）

- [ ] `semantic_sync.rs` の fingerprint 比較が削除される。
- [ ] 役割同期のために Arc ポインタ同一性を使わない。

### 既に満たされている（またはほぼ満たされている）条件

- [x] トップレベル role の権威は概ね `SemanticRegistry.stack_hints`（rendering はここを読む）。ただし
      module word 経路だけ fingerprint 再同期に依存しており「どちらが権威か」の曖昧さが残る。
- [x] wire format / 表示は既存テストで固定済み。
- [x] 表層意味論は不変。

前セッションで Codex が実施済み: 4A（回帰テスト）、4B（`SemanticStack`/`StackSlot` façade 導入、
`types/semantic_stack.rs`）、adapters（`interpreter/semantic_stack_adapter.rs`）、4D 一部
（CLI stack rendering を façade 経由へ）。**未実施: 4C（実行経路の façade 移行）・4E（内部表現確定）・
4F（`semantic_sync.rs` 削除）・4G（旧 `stack_hints` 直接 API と Value.hint 推測経路の削除）**。

## 1. 最重要: 調査で確定した制約（ショートカット禁止事項）

新セッションが同じ落とし穴を踏まないよう、確定事項を先に示す。

1. **`Value.hint` と「トップレベル plane role」は別物であり、統合してはならない（候補案 C は不可）。**
   `value_to_interval`（`rust/src/interpreter/interval_ops.rs:10`）は
   `match (&value.data, value.hint)` で **2 要素 Vector を `value.hint == Interpretation::Interval` の
   ときだけ `[lo, hi]` interval として解釈する**。つまり `value.hint` は「値構築時のデータ役割」
   （interval マーカー等）を担い、plane role（stack 位置の役割、`>CF` 等で書き換わる）とは意味が違う。
   両者を同一視すると `MATH@INTERVAL` / `MATH@SQRT` 系が壊れる。SPEC §12 の「role は data plane と分離」
   とも整合する。

2. **plane role を `value.hint` に焼き込む方式は不可（上と同根）。** module 実行前に
   `value.hint = plane_role` と上書きする案は、interval マーカーを潰す/でっち上げるため不可。

3. **fingerprint は load-bearing。** `modules/semantic_sync.rs` の fingerprint 差分は、module word が
   「触っていない下位スロットの plane role（`>CF` override 等）を保持しつつ、作り直したスロットは
   新 `value.hint` を採用する」ために存在する。単純に「全スロットを `value.hint` から再導出」すると、
   `>CF` 後に module word を挟んだ場合に CF role を失う。

4. **value 等価比較で Arc 同一性だけ外す案は中途半端。** criterion 4（Arc 同一性排除）は満たすが、
   (a) module word 前に stack 全 clone が必要で**性能退行**、(b) 「作り直したが構造的に等しい値」の扱いが
   Arc 版と微妙に変わる（稀だが semantic plane の挙動差）。criterion 3（fingerprint 比較の削除）は満たさない。
   → **push 命令の instrumentation（＝ role を Stack が保持する構造化）以外に、両 criterion を正しく満たす道はない。**

## 2. 現行アーキテクチャ（要点）

- `Stack` は **型エイリアス** `pub type Stack = Vec<Value>;`（`rust/src/types/mod.rs:720`）。role は持たない。
- plane role は `SemanticRegistry.stack_hints: Vec<Interpretation>`（`types/mod.rs:531`）が別 Vec で保持。
  `SemanticRegistry` は他に `flow_hints` / `flow_extensions`（**value id キーの nested 拡張。Phase 4 非対象、触るな**）も持つ。
- role 更新の 3 経路:
  - **Core word**: `apply_word_hint_override`（`interpreter/execution_loop.rs`）が語名→role のハードコード表で
    `stack_hints[top]` を更新。
  - **`>CF` 等の位置キャスト**: `stack_hints[i]` を直接書き換え（値は不変）。
  - **Module word**: `execute_module_word`（`interpreter/modules/module_registry.rs`）が
    `snapshot_stack_slots`→executor→`resync_changed_slots`（`modules/semantic_sync.rs`）で
    Arc 同一性 fingerprint により変更スロットのみ `value.hint` を採用。
- rendering / wire は `stack_hints` を権威に読む（`cli/report.rs`、`wasm_interpreter_bindings/`、
  `interpreter/semantic_stack_adapter.rs::semantic_stack_snapshot`）。

## 3. 影響範囲（実測）

- `interp.stack.*` 直接操作（非 test）: **668 箇所**。内訳（mutation）:
  push 481 / pop 172 / clear 68 / extend 25 / drain 20 / truncate 6 / split_off 4 / reverse 1 / remove 1 / insert 1。
  index 代入 `stack[i] = ` は **1 箇所**のみ。
- stack 全体の退避・復元: `mem::swap|take|replace(... .stack ...)` **22 箇所** + `.stack = ` 直接代入 **66 箇所**
  （HOF / COND / shadow validation / child runtime の sub-execution 前後。**現状はここで `stack_hints` を
  別途退避・復元している** → 単一権威化でここを統合する）。
- `stack_hints` / `semantic_registry` 参照（非 test）: **167 箇所 / 約 27 ファイル**。多い順:
  `types/mod.rs`(22, SemanticRegistry 本体) / `comparison.rs`(15) / `math_ops.rs`(14) / `child_runtime.rs`(14) /
  `interval_ops.rs`(12) / `control_cond.rs`(11) / `execution_loop.rs`(9) / `tier2_ops.rs`(7) /
  `interpreter_core.rs`(7) / `time_ops.rs`(6) / `shadow_validation.rs`(6) / `compiled_plan.rs`(5) …。
  ※このうち相当数は `flow_hints`/`flow_extensions`（非対象）。`stack_hints` 系のみを対象化すること。

## 4. 目標設計（候補案 A の最小 churn 実装）

`Stack` を role 同伴の struct にし、**`Stack.roles` をトップレベル role の唯一の権威**とする。
`Vec<Value>` 前提の read 系 668 箇所の大半は `Deref` で無改修のまま通す。

```rust
// types/stack.rs (新規) もしくは types/mod.rs
pub struct Stack {
    values: Vec<Value>,
    roles: Vec<Interpretation>, // values と常に同長（不変条件）
}

impl std::ops::Deref for Stack {         // 読み取り（len/iter/index-read/last/is_empty/get/contains 等）を透過
    type Target = Vec<Value>;
    fn deref(&self) -> &Vec<Value> { &self.values }
}
// DerefMut は実装しない（role desync を型で防ぐ）。

impl Stack {
    // mutation は role を同時維持。push は「構築時 role = value.hint」を採用。
    pub fn push(&mut self, value: Value) { self.roles.push(value.hint); self.values.push(value); }
    pub fn pop(&mut self) -> Option<Value> { self.roles.pop(); self.values.pop() }
    pub fn truncate(&mut self, n: usize) { self.values.truncate(n); self.roles.truncate(n); }
    pub fn clear(&mut self) { self.values.clear(); self.roles.clear(); }
    pub fn extend<I: IntoIterator<Item = Value>>(&mut self, it: I) { for v in it { self.push(v); } }
    pub fn drain(&mut self, range) -> ... { /* values/roles 双方を drain。戻り値は Value のみで可 */ }
    pub fn split_off(&mut self, at: usize) -> Stack { /* 双方を split */ }
    // insert/remove/reverse は該当 1 箇所ずつ。双方に適用。
    // 明示的な plane role 操作（>CF, core override）:
    pub fn set_role(&mut self, i: usize, role: Interpretation) { if i < self.roles.len() { self.roles[i] = role; } }
    pub fn role_at(&self, i: usize) -> Interpretation { self.roles.get(i).copied().unwrap_or(Interpretation::Unassigned) }
    pub fn roles(&self) -> &[Interpretation] { &self.roles }
    // in-place 値変更用（.hint 変更を含まない前提で安全）:
}
impl std::ops::Index<usize> for Stack { ... &self.values[i] }
impl std::ops::IndexMut<usize> for Stack { ... &mut self.values[i] } // 値の in-place 変更用（role は不変）
```

### 権威の移管

- `SemanticRegistry.stack_hints` を**削除**し、その API（`push_hint`/`pop_hint`/`update_hint_at`/
  `lookup_hint_at`/`lookup_last_hint`/`truncate`/`clear`/`normalize_to_stack_len`/`collect_stack_hints`/
  `set_stack_hints`）を **`Stack` 側 role API へ委譲**するか、呼び出し側を書き換える。
  `flow_hints`/`flow_extensions` は `SemanticRegistry` に残す。
- **Module word 経路**: `push` が role=`value.hint` を自動維持するため、`snapshot_stack_slots` /
  `resync_changed_slots` / `SlotFingerprint` は**不要になり、`modules/semantic_sync.rs` を削除**できる
  （criterion 3・4 達成）。触っていない下位スロットは pop されないので role が保存され、作り直したスロットは
  push 時に `value.hint` を採用する（fingerprint と同じ結果を、差分計算なしで得る）。
- **Core word override / `>CF`**: `stack.set_role(i, role)` を使う（`apply_word_hint_override` と CF キャスト）。
- **sub-execution 退避/復元（HOF/COND/shadow/child）**: `Stack` ごと swap/restore すれば role も一緒に動くため、
  **別建ての `stack_hints` 退避/復元を削除**（二重管理解消の本丸）。

## 5. 段階手順（各段で `cargo test` green を維持）

memo の 4C–4G を具体化。各段は独立 PR とし、前段 green を確認してから次段へ。

- **Stage 1（基盤・挙動不変）**: `Stack` を struct 化。`Deref`＋mutation メソッド実装。
  `roles` は当面 `stack_hints` の**同期ミラー**として維持（`SemanticRegistry.stack_hints` はまだ権威）。
  すべての `.stack.push/pop/...` が role を維持することを確認。全テスト green を確認（**ここで観測挙動は不変**）。
- **Stage 2（権威移管・module 経路）**: `execute_module_word` の前後で `stack_hints` を `Stack.roles` から
  同期する形に切替 → **fingerprint を撤去**し `modules/semantic_sync.rs` を削除。
  `json_semantic_role_tests` と後述の新規 `>CF`×module テストで固定。
- **Stage 3（core override / `>CF`）**: `apply_word_hint_override` と CF キャストを `stack.set_role` へ。
- **Stage 4（sub-execution 統合）**: HOF/COND/shadow/child の stack 退避/復元を `Stack` 一括へ統合し、
  `stack_hints` 別建て退避を削除。**shadow validation の比較が同一観測であることを最優先で確認**。
- **Stage 5（権威一本化）**: `SemanticRegistry.stack_hints` を削除。読み手（`cli/report.rs` /
  `wasm_interpreter_bindings` / `semantic_stack_adapter`）を `Stack.roles` へ。互換 API を撤去（4G）。
- **Stage 6（仕上げ）**: 生成物再生成（word-manifest/skill 等は語不変なので原則不変、provenance は要再生成）。
  memo と本引継書の status を更新。

## 6. 検証戦略（必須）

- **回帰前に固定するテスト**（memo「必須回帰テスト」全項）に加え、**現状 fingerprint が守る未カバー経路を新規に固定**:
  - `>CF` でトップ role を CF にした下位スロットが、直後の module word（例: `1 >CF ... DATA@... ` の類）を
    跨いで CF role を保持すること。**この経路は現状テストが薄い**ため Stage 2 前に追加する。
- **shadow validation**: 高速経路と参照経路の stack 比較が「同じ (data, role) 観測」を比較していること。
  Stage 4 で最重要。`interpreter/shadow_validation.rs` の stack 比較を確認。
- **exact-real hot paths**: `comparison.rs` / `interval_ops.rs` / `math_ops.rs` / `tier2_ops.rs` の
  role 依存（特に `Interpretation::Interval` 解釈）を壊さない。`arithmetic_operation_tests` /
  `json_semantic_role_tests` を各段で実行。
- **CI 相当フル**: `cargo test --all-targets`、`npm run check:semantic-firewall`、
  `npm run provenance:*`、`npm run word:manifest:check`、`npm run check:skill`、
  `npm run check:formalization-coverage`、`npm run check`（tsc）、WASM boundary、Python 差分。
- **wire/protocol 不変**: `cli/report.rs` の JSON と WASM protocol が既存テストと byte 一致。

## 7. リスクと非対象

- **リスク**: semantic plane は言語全体の rendering に効くため、微妙な role 取り違えが広範に波及しうる。
  exact-real 観測機構との相互作用に注意。各段で shadow validation と semantic role テストを回すこと。
- **非対象**: 表層構文・CLI JSON・WASM wire の破壊的変更、NIL/UNKNOWN/exact number の意味変更、
  `flow_hints`/`flow_extensions` の変更、Module word executor の一括書き換え（`Deref`＋`push` 自動 role で
  executor は原則無改修）。

## 8. memo が残した仕様上の未解決点（移行中に判断）

- nested container の外側 role として、トップレベル role と `Value` 構築時既定 role のどちらを使うか。
- Module word が内部構築した `Value.hint` をトップレベル role へ自動昇格してよい範囲
  （本設計では push 時に `value.hint` を role とする ＝ 昇格を既定とする。想定外なら Stage 2 のテストで顕在化）。
- NIL passthrough 時に absence payload と role をどちらの抽象が所有するか
  （`Stack.roles` が role、`Value.absence` が absence を所有で分離）。

これらは新しい正典意味論として確定せず、移行時の調査・テスト対象として扱う（SPEC §12 と齟齬が出たら停止）。

## 9. 着手時のクイックスタート

1. `git fetch origin main && git checkout -B <branch> origin/main`。
2. 本引継書と `semantic-role-ownership-phase4-design.md`、SPEC §12 を読む。
3. Stage 1 前に §6 の新規 `>CF`×module テストを追加し、現状挙動を固定。
4. Stage 1 から順に、各段で全テスト green を確認しつつ独立 PR で進める。
