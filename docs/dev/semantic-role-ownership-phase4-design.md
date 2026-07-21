# Phase 4: semantic role ownership design memo

Status: `[実施済み・候補案 A 採用]`。この文書は `docs/dev/` の設計メモであり、Ajisai の意味論を定義しない。正典は `SPECIFICATION.html` のみである。実施結果は `semantic-role-ownership-phase4-migration-handoff.md` の「完了記録」を参照。

> **残移行（4C–4G）の具体的な調査結果・設計・段階手順・検証ゲートは
> `docs/dev/semantic-role-ownership-phase4-migration-handoff.md`（作業引継書）に集約した。**
> 本メモは候補案 A/B/C の比較と段階計画の根拠を残す。着手時は引継書を先に読むこと。

## 実施フェーズ

Phase 4: 意味役割の二重管理解消。

このメモは Phase 4 の着手条件である「最初から `Stack = Vec<StackSlot>` への全面置換を開始せず、まず設計メモを作り、複数案を比較する」を満たすためのものである。この変更単位では Rust 実行経路、CLI JSON、WASM wire format、GUI 表示、Ajisai 表層構文を変更しない。

## 目的

現行実装では、トップレベルのスタック位置に対応する意味役割が主に二つの場所で扱われている。

- `Value.hint`: 値が構築時に持つ既定役割、および Vector、Tensor、Record などネスト構造の leaf role を保持する。
- `SemanticRegistry.stack_hints`: 実行中のトップレベルスタック位置ごとの役割列を保持する。

Module word 実行後には `semantic_sync.rs` が fingerprint によって変更スロットを再同期しており、どちらがトップレベル役割の権威かを読み手が推測しなければならない箇所が残っている。Phase 4 の目標は、トップレベルスタックスロットの役割に唯一の権威を与え、同期専用の fingerprint 経路を不要にすることである。

## 着手前調査の要約

`rg` による横断確認では、Phase 4 の主な依存点は次の通りである。

- `rust/src/types/mod.rs`: `Value.hint` と `SemanticRegistry.stack_hints` が存在する。
- `rust/src/interpreter/interpreter_core.rs`: interpreter 本体が `SemanticRegistry` を保持し、`set_stack_hints` と `collect_stack_hints` を公開している。
- `rust/src/interpreter/modules/semantic_sync.rs`: Module word 実行後の `Value.hint` と `stack_hints` の再同期を担当している。
- `rust/src/interpreter/control_cond.rs`、`rust/src/interpreter/shadow_validation.rs`、`rust/src/interpreter/higher_order/`: サブ実行、fallback、HOF で `stack_hints` を退避・復元している。
- `rust/src/cli/report.rs`、`rust/src/wasm_interpreter_bindings/`: wire/protocol 生成時に stack value と role を組み合わせている。

既存テストは `json_semantic_role_tests`、role rendering 関連テスト、COND/HOF/Shadow Validation 周辺テストに分散している。Phase 4 のコード移行前に、下記の「必須回帰テスト」を追加または既存テストへ対応付ける必要がある。

## 正典から維持する不変条件

SPEC §12 の意味を、実装構造上の制約として次のように読む。ただし、この文書は仕様解釈を確定しない。

- スタック上の各位置は data と role の組として観測される。
- 表示は `(data, role)` の純粋関数であり、role は計算結果そのものを変えない。
- semantic plane は data plane から分離される。
- role は明示的な semantic boundary でのみ適用され、render 時に値内容から推論し直さない。
- `Unassigned` は richer inference を意味せず、構築時に role が決まらなかったことを表す。
- stack projection と output projection は区別されるが、role と data の対応を失ってはならない。
- Vector、Tensor、Record 内部の leaf role は、トップレベル stack slot の role とは別に保持されなければならない。
- NIL passthrough は absence reason、origin、recoverability と role を落としてはならない。

## 候補案 A: StackSlot 方式

```rust
pub struct StackSlot {
    pub value: Value,
    pub role: Interpretation,
}

pub type Stack = Vec<StackSlot>;
```

### 判断

Phase 4 の最終目標として最も適している。

### 理由

- SPEC §12 の「stack is a sequence of `(data, role)` pairs」という観測モデルを直接表せる。
- トップレベル role の唯一の権威を `StackSlot.role` に置ける。
- `Vec<Value>` と `Vec<Interpretation>` の長さ不一致を構造的に作りにくい。
- push、pop、truncate、extend、snapshot、Shadow Validation の比較対象を一つの単位へ寄せられる。
- `Value` は構築時既定役割やネスト構造の leaf role を保持しつつ、トップレベル位置依存 role は `StackSlot` が所有できる。

### リスク

- 現行コードは `Vec<Value>` を前提にする箇所が多く、初手で全面置換すると大規模差分になる。
- CLI/WASM wire format、Shadow Validation、HOF、COND、子ランタイムに同時影響しやすい。
- Module word の既存 executor が `Value` を直接 push/pop するため、移行アダプタが必要になる。

## 候補案 B: SemanticStack 方式

```rust
pub struct SemanticStack {
    values: Vec<Value>,
    roles: Vec<Interpretation>,
}
```

`values` と `roles` は private にし、push、pop、truncate、extend、snapshot などを唯一の操作口にする。

### 判断

段階移行の中間抽象として有効である。

### 理由

- 外部 API を一気に変更せず、既存の `Vec<Value>` 前提コードを段階的に閉じ込められる。
- 長さ不一致を public API から作れないようにできる。
- `collect_stack_hints` や report/WASM adapter を互換層として残しつつ、内部 mutation を集約できる。
- StackSlot 方式へ後で内部表現だけを差し替えやすい。

### リスク

- 内部表現として `values` と `roles` が分かれているため、private 境界が破られると二重管理が温存される。
- `Value.hint` と `roles` の責務分離を文書化しないと、`semantic_sync.rs` が残り続ける可能性がある。
- 最終形として採用する場合も、StackSlot と同等の invariant test が必要である。

## 候補案 C: Value 所有方式

トップレベル role を `Value.hint` へ統合し、`SemanticRegistry.stack_hints` を廃止する。

### 判断

Phase 4 の採用案から除外する。

### 理由

- role が stack position に属するという観測モデルを、clone 可能な data object へ寄せてしまう。
- `>CF` のような位置依存の再解釈と、値構築時の既定 role の境界が曖昧になる。
- KEEP、COND、HOF、Shadow Validation で「同じ data を別 role で観測する」ケースを扱いにくい。
- Vector、Tensor、Record 内部の leaf role とトップレベル role の区別が壊れやすい。
- wire format 互換を維持する adapter が結局必要になり、責務削減の効果が薄い。

## 推奨する段階移行

Phase 4 では、候補 A を最終目標に置きつつ、候補 B の private façade を経由して移行する。

1. Phase 4A: 現状挙動を固定する回帰テストを追加する。
2. Phase 4B: `SemanticStack` façade を導入し、push、pop、truncate、extend、snapshot、role update を集約する。
3. Phase 4C: Coreword、Module word、COND、HOF、子ランタイム、Shadow Validation の順に façade 経由へ移行する。
4. Phase 4D: CLI と WASM の adapter を façade から生成し、wire/protocol 表現を既存と一致させる。
5. Phase 4E: call site が集約できた時点で、内部表現を `Vec<StackSlot>` に変更するか、`SemanticStack` が同等 invariant を満たすならそのまま最終形にするかを判断する。
6. Phase 4F: Module word の戻り値・stack mutation が新抽象経由になった後、`semantic_sync.rs` と fingerprint 再同期を削除する。
7. Phase 4G: 旧 `stack_hints` 直接操作 API と、トップレベル role を `Value.hint` から推測する互換経路を削除する。

この順序により、Phase 4 の高リスクな全置換を避けつつ、最終的にトップレベル role の権威を一箇所へ収束させる。

## 必須回帰テスト

コード移行前に、少なくとも次の振る舞いを固定する。

- `>CF` 後の表示が変わらない。
- `>CF` 対象外スロットの role が変わらない。
- KEEP による値保持で role が失われない。
- NIL passthrough で absence reason、origin、recoverability、role が失われない。
- 論理的 UNKNOWN が TruthValue role として観測される。
- JSON parse 後に入力 Text role が parse 結果へ漏れない。
- Text、Boolean、Timestamp、Interval の表示 role が既存通りである。
- Vector と Tensor の leaf role が保持される。
- Record の構築と直列化で role/protocol が既存通りである。
- COND guard 実行前後で stack role が復元される。
- MAP、FILTER、FOLD 前後で callback と外側 stack の role が混線しない。
- Shadow Validation の stack 比較が同じ観測を比較する。
- WASM と CLI の protocol 表現が一致する。
- 新しい stack 抽象の public API から stack と role の長さ不一致を作れない。

## 非対象

この設計メモの変更単位では次を行わない。

- Ajisai 表層構文の変更。
- `Value.hint`、`SemanticRegistry.stack_hints`、`semantic_sync.rs` の削除。
- CLI JSON または WASM wire format の破壊的変更。
- role 表示規則、NIL、UNKNOWN、exact number の意味変更。
- Shadow Validation の観測対象削減。
- Module word executor の一括置換。

## 互換性方針

- 表層構文: 変更しない。
- CLI JSON: 変更しない。Phase 4 の実装段階でも additive 変更が不要な限り避ける。
- WASM: wire/protocol を維持する。内部 stack 抽象を公開しない。
- GUI: 表示結果を既存テストで固定し、内部 role ownership を GUI contract にしない。
- conformance: SPEC §12 の既存観測を固定するテストを優先する。
- reference interpreter: Rust 実装の都合で正典意味論を変更しない。

## 仕様上の未解決点

現時点では、Phase 4 のコード移行を止める仕様穴は確定していない。ただし、次の点で SPEC §12 と既存実装の対応付けが曖昧になった場合は、実装を進めず停止する。

- トップレベル role と `Value` の構築時既定 role のどちらを nested container の外側 role として扱うか。
- Module word が内部で構築した `Value.hint` を、トップレベル stack slot role へ自動昇格してよい範囲。
- NIL passthrough 時に absence payload と role をどちらの抽象が所有するか。

このメモでは、これらを新しい正典意味論として確定せず、移行時の調査・テスト対象として扱う。
