# Implementation Quickstart

開発者が最短で作業開始できるための最小ガイド。

## 正典ドキュメント

| ドキュメント | 役割 | いつ見るか |
|---|---|---|
| `SPECIFICATION.md` | 言語仕様の唯一の正 | 仕様確認・実装判断時 |
| `CLAUDE.md` | AI支援開発のコンテキスト | Claude Code 使用時（自動参照） |
| `README.md` | プロジェクト概要・導入 | 初回のみ |

## 日常開発で見る最小セット

1. **実装対象の仕様セクション** → `SPECIFICATION.md` の該当セクションID（例: `#CONTROL-COND`）
2. **コマンド一覧** → `CLAUDE.md` の Commands テーブル
3. **ファイル配置** → `CLAUDE.md` の Architecture セクション

## よく使うコマンド

```bash
npm run check              # TypeScript 型チェック
cd rust && cargo test --lib  # Rust ユニットテスト
cd rust && cargo test        # 全テスト（統合テスト含む）
```

## 変更時チェックリスト

### 新しい組み込みワードを追加する場合
- [ ] `rust/src/builtins/builtin-word-definitions.rs` に登録
- [ ] `rust/src/interpreter/execute-builtin.rs` の eval ループに分岐追加
- [ ] ワードの実装ファイルを作成（`interpreter/` 配下）
- [ ] `SPECIFICATION.md` に仕様記載
- [ ] テスト追加（`rust/tests/gui-interpreter-test-cases.rs` or inline）

### 値・型を変更する場合
- [ ] `Value`/`ValueData` にメタデータを追加しない（`SemanticRegistry` に入れる）
- [ ] `DisplayHint` は Semantic Plane に配置

### テスト修正・追加する場合
- [ ] `cargo test` 全通過を確認
- [ ] `npm run check` でTS型エラーゼロを確認

## 仕様変更時に必ず更新するファイル一覧

| 変更内容 | 更新対象ファイル |
|---|---|
| ワード追加/変更 | `SPECIFICATION.md`, `builtin-word-definitions.rs`, `CLAUDE.md` |
| 構文変更 | `SPECIFICATION.md`, `tokenizer.rs`, `CLAUDE.md` |
| エラー追加 | `SPECIFICATION.md`, `error.rs` |
| モジュール追加 | `SPECIFICATION.md`, `modules.rs`, `CLAUDE.md` |
| ファイル名変更 | `docs/migration-file-renaming-inventory.md` |

## 禁止事項（CLAUDE.md Critical Rules より）

- `>`, `>=` 演算子の追加禁止
- DUP/SWAP/ROT/OVER 禁止
- `MAX_CALL_DEPTH`(4) / 次元上限(10) の引き上げ禁止
- 後方互換 shim / feature flag 禁止
