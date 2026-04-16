# Ajisai DisplayHint 改修指示書（Arena + NodeId / SoA 採用）

本書は、Ajisai の値表現を **Arena + NodeId（木本体）** と **SoA（hint 配列）** に移行するための、Codex 向け実装指示書である。  
目的は、無制限ネストに対して DisplayHint を安定的・高速に扱えるアーキテクチャへ更新すること。

---

## 0. 背景と目的

- 現行は `Value` ノードに `DisplayHint` を保持しつつ、`SemanticRegistry.stack_hints` も併用している。
- 今後の主戦略は、**値木と hint を分離**し、`NodeId` による安定参照でネスト深度に依存しない表現を実現すること。
- 目標:
  1. DisplayHint の決定を「推論依存」から「明示情報優先」へ。
  2. 深いネストでもヒント整合性が崩れないこと。
  3. 将来的な SIMD/最適化・キャッシュ戦略に耐える内部表現にすること。

---

## 1. 完了条件（Definition of Done）

以下を満たしたら完了:

1. 値木の主表現が `NodeId` 参照（Arena 管理）である。
2. `DisplayHint` は `Vec<DisplayHint>` 等の SoA 領域で管理され、ノードと同じ index（NodeId）で参照される。
3. wasm 変換・JSON 変換・主要演算経路が NodeId ベース API を使用する。
4. 既存テストが通る（必要に応じて更新）＋ 新規回帰テストが追加される。
5. 既存の再現ケース（数値ベクターが `'...'` 表示される誤判定）が再発しない。

---

## 2. 非目標（このスコープでやらない）

- UI/UX の見た目変更（Stack 表示デザイン変更など）
- 全演算の最適化完了（まずは正しさ優先）
- Arena の高度メモリ最適化（free-list 圧縮など）は後続フェーズ

---

## 3. 新データモデル（提案）

> 命名は実装時に調整可。意味を優先すること。

### 3.1 Core 型

```rust
pub type NodeId = u32;

pub enum NodeKind {
    Nil,
    Scalar(Fraction),
    Vector { children: Vec<NodeId> },
    Record { pairs: Vec<NodeId>, index: HashMap<String, usize> },
    CodeBlock(Vec<Token>),
    ProcessHandle(u64),
    SupervisorHandle(u64),
}

pub struct ValueArena {
    pub nodes: Vec<NodeKind>,          // AoS: 木本体
    pub hints: Vec<DisplayHint>,       // SoA: display hint
    // 必要なら将来拡張:
    // pub flags: Vec<NodeFlags>,
}
```

### 3.2 重要不変条件（Invariant）

1. `nodes.len() == hints.len()`
2. 任意 NodeId `id` について `id < nodes.len()`
3. `Vector.children` / `Record.pairs` の全要素が有効 NodeId
4. `hints[id] == Auto` は「未知」ではなく「自動解決対象」の意味

---

## 4. 実装フェーズ

### Phase 1: 基盤導入（並行稼働）

1. `types` 配下に `arena.rs`（仮）を追加し、`NodeId` / `NodeKind` / `ValueArena` を定義。
2. 生成 API を用意:
   - `alloc_scalar(f, hint) -> NodeId`
   - `alloc_vector(children, hint) -> NodeId`
   - `alloc_string(&str) -> NodeId`（内部は scalar codepoint の vector + String hint）
3. 走査 API を用意:
   - `kind(id) -> &NodeKind`
   - `hint(id) -> DisplayHint`
   - `children(id) -> &[NodeId]`（vector/record用）
4. 現行 `Value` API はこのフェーズでは残し、相互変換関数を追加:
   - `value_to_arena(root: &Value) -> (ValueArena, NodeId)`
   - `arena_to_value(arena: &ValueArena, root: NodeId) -> Value`

### Phase 2: 表示・変換経路の切替

1. wasm 出力 (`value_to_js_value_with_hint` 相当) を arena 版に追加:
   - `arena_node_to_js(arena, root_id, external_hint_opt)`
2. JSON serialize/deserialize を arena 版に追加:
   - `json_to_arena_node(...) -> NodeId`
   - `arena_node_to_json(...) -> serde_json::Value`
3. 再現ケースの回帰テストを arena API で実装:
   - 入力: `[ [ [ 88 ] [ 99 ] [ 100 ] ] [ [ 50 ] [ 32 ] [ 44 ] 22 ] ]`
   - 期待: 数値ベクターが文字列化されないこと

### Phase 3: Interpreter 主要経路の移行

1. Stack を `Vec<Value>` から段階的に `Vec<NodeId>` へ置換（必要に応じて中間層導入）。
2. `SemanticRegistry.stack_hints` 依存ロジックを縮退し、ノード hint 参照へ寄せる。
3. 主要演算（算術、vector 操作、cast、json、wasm 境界）を NodeId 入出力へ移行。

### Phase 4: 旧実装の縮退と削除

1. `Value` 直保持経路を deprecate。
2. 重複した hint 管理（stack-level のみで持つ仕組み）を整理。
3. 変換ブリッジを最小化し、最終的に内部統一表現を Arena に一本化。

---

## 5. テスト計画

### 5.1 必須回帰テスト

1. **Nested Vector DisplayHint 回帰**
   - 数値ベクターが `'X'` 形式へ誤変換されない。
2. **String literal 保持**
   - 明示文字列のみ string hint として表示される。
3. **深いネスト**
   - 既存次元制限撤廃前提で 10+ 深度でも hint が崩れない。

### 5.2 互換性テスト

1. `value_to_arena` -> `arena_to_value` roundtrip
2. JSON roundtrip（現行仕様との整合）
3. wasm 境界 roundtrip（型と displayHint の一致）

### 5.3 性能計測（任意だが推奨）

- 既存ベンチ比較:
  - ネスト深い vector 表示
  - JSON parse/stringify
  - map/fold 系での hint 参照頻度

---

## 6. 移行時の注意点

1. **NodeId の安定性**
   - 削除を伴う場合は tombstone または free-list 戦略を明示する。
2. **hint のデフォルト規約**
   - `Scalar(Number)` でも `Boolean` を許す点を仕様化する（値と表示意味は分離）。
3. **Record の key 保証**
   - 既存の string-like key 前提を崩さないように、record key の hint 規約を定義する。
4. **境界層の一本化**
   - wasm/JSON は必ず arena API 経由で変換し、独自推論コードを分散させない。

---

## 7. Codex 実行手順（推奨）

1. Phase 1 の型追加と相互変換までを 1 PR。
2. Phase 2（wasm + json + 回帰テスト）を 1 PR。
3. Phase 3（interpreter 主経路）を複数 PR に分割。
4. Phase 4（削除・整理）を最終 PR。

各 PR は以下を必須化:
- 変更理由
- 影響範囲
- 回帰テスト結果
- 既知制約（あれば）

---

## 8. 受け入れチェックリスト

- [ ] `ValueArena` が導入され、NodeId で木を参照できる
- [ ] SoA hint 配列がノードと同一 index で管理される
- [ ] wasm / json / display の主要経路が arena を利用
- [ ] 再現ケースの回帰テストが追加・成功
- [ ] 既存主要テストが成功
- [ ] ドキュメント（本書）と実装が同期

