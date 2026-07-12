# Ajisai GUI構成整理 改修指示書（改訂版）

## 1. 妥当性評価サマリ

元の指示書は、課題認識・非スコープ・段階的方針が明確で、**リファクタリング計画としての方向性は妥当**です。  
一方で、実装リスクを上げる点があるため、以下を補正します。

### 主な問題点

1. **Phase番号と内容の不整合**
   - 本文中で「Phase 7」が `window.ajisaiInterpreter` 依存削減とWASM隔離の両方に見える箇所があり、実施順序が曖昧。
2. **「A/B優先度」の定義不足**
   - 「優先度AおよびBまで」とあるが、各項目にA/Bタグがない。
3. **受け入れ条件が“機能維持”中心で、差分安全性の客観指標が弱い**
   - 例: ファイル移動後の import 破断検知、CSS回帰検知の具体策が不足。
4. **リンク修正方針の実装依存が不明瞭**
   - `SPECIFICATION.md` を直接リンクする案は配信環境で崩れる可能性がある。
5. **大規模移動時のPR分割ルールが未定義**
   - 一度に行うとレビュー不能化しやすい。

---

## 2. 改訂方針（変更点）

- Phaseを**P0〜P8**に再採番し、依存関係を明確化。
- 全タスクを **Priority A / B** で明示。
- 各Phaseに「必須チェック」と「回帰観点」を追加。
- `docs/index.html` は **public/docs/index.html を必須採用**（404回避を優先）。
- 実装は **小PR分割（1PR=1Phase原則）** を明文化。

---

## 3. 改訂版スコープ

## Priority A（今回必須）

A1. `js/` → `src/` への移行（最小破壊）  
A2. `entry-web.ts` / `entry-tauri.ts` 重複整理  
A3. `GUIElements` 実行時検証導入  
A4. `gui-application.ts` からイベント登録分離  
A5. `gui-application.ts` からレイアウト制御分離（状態入口一本化）  
A6. `docs/index.html` リンク修正（404解消）

## Priority B（今回実施、ただし段階的）

B1. CSS責務分割（`src/styles/` 集約）  
B2. `window.ajisaiInterpreter` 直接依存の縮退  
B3. WASM生成物隔離（`src/wasm/generated/`）

## 非スコープ（据え置き）

- UIフレームワーク導入
- 見た目刷新
- Rustインタプリタ仕様変更
- WASM生成手順の全面刷新
- Tauri新機能追加

---

## 4. 実施フェーズ（改訂版）

## P0. ベースライン固定（事前）

- 目的: 比較可能な状態を固定。
- 実施:
  - `npm run check`
  - `npm test`
  - `npm run build:web`
  - 可能なら `npm run build:tauri-frontend`
- 成果物:
  - 失敗中テストがある場合は「既知失敗」として記録。

## P1 (A1). `js/` → `src/` 移行

- 変更:
  - ディレクトリ rename。
  - `tsconfig.json`, `vite.config.ts`, `index.html`, importパス更新。
- ルール:
  - このPhaseでは**ロジック変更禁止**（パス修正のみ）。

## P2 (B1). CSS再分割

- 変更:
  - `src/styles/{tokens,base,layout,components,responsive,debug}.css`。
  - `public/ajisai-base.css` と `app-interface.css` の責務を移管。
- ルール:
  - セレクタ名変更は原則禁止。
  - CSSロード方式は「index.html直リンク」または「TS import」のどちらか一方に統一。

## P3 (A2). Entry重複整理

- 変更:
  - `entry-bootstrap.ts` + `entry-common.ts`（名称は同等構造なら可）。
- ルール:
  - Web/Tauri差分は `platform` 側へ寄せる。

## P4 (A3). GUI DOM実行時検証

- 変更:
  - `requireElement` 導入、`as HTML...` の無検証キャスト削減。
- ルール:
  - エラーメッセージに element ID を必ず含める。

## P5 (A4). イベント登録分離

- 変更:
  - `gui-event-bindings.ts` へ移管。
- ルール:
  - 初回は移動中心、ロジック再設計はしない。

## P6 (A5). レイアウト状態一本化

- 変更:
  - `layout-model/controller/renderer`（配置名は同等責務なら可）。
- ルール:
  - 状態更新とDOM反映を分離。
  - 既存 `mobile-view-switcher.ts` は段階吸収可。

## P7 (B2+B3). Interpreter依存縮退 + WASM生成物隔離

- 変更:
  - `interpreter-client` を追加し `window.ajisaiInterpreter` 参照を集約。
  - `src/wasm/generated/` へ生成物移動。
  - 参照パス・Vite設定・除外設定更新。
- ルール:
  - Worker経由実行を壊さない。

## P8 (A6). Referenceリンク修正 + 文書更新

- 変更:
  - `public/docs/index.html` を追加し `index.html` の `docs/index.html` 導線を維持。
  - READMEまたは既存dev docs更新。
- ルール:
  - ルートMarkdownへの相対リンクは避け、必要ならGitHub URLを使う。

---

## 5. 受け入れ条件（改訂版）

## 共通必須（各Phase完了時）

- `npm run check` 成功
- `npm test` 成功
- `npm run build:web` 成功

## 終了時必須

- `js/` がソースルートとして残っていない
- GUI TypeScriptが `src/` 配下
- CSSが `src/styles/` に責務分割
- `gui-application.ts` が初期化調停中心
- イベント登録が分離済み
- レイアウト状態入口が単一化
- DOM取得に実行時検証
- Entry重複解消
- WASM生成物隔離
- `Reference` リンクが404にならない

## 推奨追加確認

- 可能なら `npm run build:tauri-frontend`
- 可能なら `cd rust && cargo test`
- 主要UIのスモーク確認（実行/ステップ/辞書切替/モバイル切替）

---

## 6. 実装ルール（改訂版）

1. **1PR=1Phase原則**（大きくても2Phaseまで）。  
2. 各PR説明に「非機能変更（リファクタリング）」であることを明記。  
3. 互換レイヤーは一時許容。ただし削除計画（TODO or issue）を残す。  
4. `public/` は静的配布物のみ。GUIロジック/CSS本体は `src/`。  
5. DOM ID/CSSクラス名変更は回帰根拠がある場合のみ。  

---

## 7. 改訂後の推奨最終構成

```text
src/
  entry/
    entry-bootstrap.ts
    entry-common.ts
  gui/
    gui-application.ts
    gui-dom-cache.ts
    gui-event-bindings.ts
    layout/
      layout-model.ts
      layout-controller.ts
      layout-renderer.ts
    interpreter/
      interpreter-client.ts
  wasm/
    wasm-module-loader.ts
    wasm-interpreter-types.ts
    wasm-interpreter-compat.ts
    generated/
  styles/
    tokens.css
    base.css
    layout.css
    components.css
    responsive.css
    debug.css
public/
  images/
  docs/
    index.html
```

以上を、元指示書の「意図（挙動維持での構成整理）」を保ちながら、実行可能性とレビュー容易性を高めた改訂版とする。
