# 外部評価への応答と開発方針

Status: `[提案・未実施]`。この文書は `docs/dev/` の設計メモであり、Ajisai の
意味論・互換性方針を定義しない。正典は `SPECIFICATION.html` のみである。記述が
`SPECIFICATION.html` と食い違う場合は正典に従う。

## 背景

外部の大規模言語モデル（ChatGPT）に Ajisai を評価させ、その内容を実コードと
突き合わせて妥当性を検証した。本文書は (1) 検証結果、(2) 検証から導いた
根本原因分析、(3) それに基づく開発方針を記録する。方針の優先順位付けが目的で
あり、個々の実装可否は各フェーズ着手時に別途裁定する。

外部評価そのものは非正典の一次資料であり、本文書はその主張のうち **コードで
再現・確認できたものだけ** を根拠として採用する。

## 1. 検証結果（妥当性判断）

外部評価は表面的な感想ではなく、同梱 WASM を実行してバグを行番号まで追跡して
いる点で信頼度が高い。主要な主張を実コードと照合した結果を以下にまとめる。

### 1.1 コードで確認できた指摘

| 指摘 | 検証結果 | 根拠 |
| --- | --- | --- |
| CodeBlock が保存→復元で NIL になる | 正確 | `rust/src/types/value_protocol.rs:238` が `CodeBlock(_) => ("nil", Null)`。復元側 `rust/src/wasm_interpreter_bindings/wasm_value_conversion.rs:143` が `"nil" => Value::nil()` |
| √2（ExactScalar）が保存→復元で有理近似に化ける | 正確。数学的な値が変わる | 保存: `value_protocol.rs:189-201` が `best_rational_approximation` → `type:"number"`。復元: `wasm_value_conversion.rs:98-102` が `parse_js_fraction` で厳密有理数として戻す |
| これは GUI セッション保存の経路上の実バグである | 正確 | 保存 `src/gui/interpreter-state-persistence.ts:77`、復元 `:306`。worker snapshot も `src/workers/interpreter-snapshot.ts:60` |
| コード行数（Rust≈77.9k / TS≈9.8k / Python≈6.1k） | ほぼ一致 | Rust 実測 77,947 行 |
| VENT `^` の「字句構造依存」批判 | 妥当 | 仕様自身が `1 ^ 2 3 ADD → 4`（`2` だけスキップ）を明記（`SPECIFICATION.html` VENT 節） |
| ユーザー語の content-address 化は Unison 類似 | 正確 | `rust/src/interpreter/word_identity.rs`、`SPECIFICATION.html` §8.6 |
| 「exact-real」の呼称は現状より過大 | 妥当 | 実装領域は有理数＋多重二次体（multiquadratic）中心。π・e・一般代数的数は対象外 |

### 1.2 外部評価が不正確／補足を要する点

- 外部評価は「approximate 情報が消える」と述べるが、`approximate: true` マーカーは
  **保存段階では出力されている**（`wasm_value_conversion.rs:274-278`
  `value_semantics_to_js`）。正しい原因は「**復元側 `js_value_to_value` が
  `semantics` ブロックを一切読まず `type`/`value` しか参照しない**」ことである
  （`wasm_value_conversion.rs:90-156`）。欠陥は復元経路にある。
- したがって「判別共用体（discriminated union）と往復プロパティテストを導入せよ」
  という推奨は方向として正しい。現行ワイヤ形式には復元に必要な情報
  （CodeBlock のソーストークン／ExactScalar の厳密表現）がそもそも含まれて
  いないためである。

### 1.3 総合判断

外部評価は妥当。特に往復バグは実在の重大な正しさ問題であり、最優先で修正すべき
という結論に同意する。現状の主要課題は機能不足ではなく、
**ホスト境界の無損失化・表層構文の単純化・実装済み範囲を正確に表す対外表現** の
三点である、という評価の位置づけも支持する。

## 2. 根本原因分析（往復バグ）

往復バグの本質は個別の変換ミスではなく、**「観測用プロトコル」と「永続化用
プロトコル」を同一シリアライザ（`value_to_js` / `value_to_protocol`）で兼用して
いること** にある。両者は要求が正反対である。

- 観測用（GUI 表示・CLI JSON・AI 検査）: ExactScalar を「≈有理近似 ＋ approximate
  マーカー」として見せるのは **意図的な設計** である。`SPECIFICATION.html` §2.3 の
  「隠れた切り捨てをしない（no hidden truncation）」ファイアウォールに従い、あえて
  近似であることを観測可能にしている。CodeBlock を `nil` として観測面から隠すのも
  同様の観測面の判断である。
- 永続化用（`collect_stack` → `restore_stack`）: セッションの往復であり、
  **無損失（`restore(collect(v)) == v`）** でなければならない。

`collect_stack`（`wasm_interpreter_state.rs:70-81`）が観測用フォーマットを
そのまま永続化に流用した結果、「正しい観測用表示」が「壊れた保存」になっている。
`value_to_protocol` の該当分岐（`value_protocol.rs:182-241`）は観測面としては
正しく、変更すべきではない。修正は **永続化面を観測面から分離する** ことである。

## 3. 開発方針

外部評価の P0〜P4 を土台に、優先順位と論拠を再構成する。上位ほど優先度が高い。

### P0 — 意味を壊さないホスト境界（最優先・即着手可）

1. **永続化プロトコルを観測プロトコルから分離する。** `collect_stack` /
   `restore_stack` 専用の判別共用体ワイヤ形式を新設する。観測用
   `value_to_protocol` は現状維持。
   - CodeBlock: `{"type":"codeBlock","source":"..."}`（ソーストークンを保持）
   - ExactScalar: `{"type":"exactAlgebraic","representation":{...}}`
     （多重二次体表現をそのまま保持）
2. **全 `ValueData` に対する往復同値テスト** `restore(collect(v)) == v` を追加する。
   重点対象: CodeBlock / ExactScalar / UNKNOWN / NIL 理由情報 / Record / Handle。
   これにより同種バグの再発を仕組みで防ぐ。テストは既存の
   `rust/src/types/value_protocol_tests.rs` の native 検証方針に沿わせる。

> この修正は `SPECIFICATION.html` §2.3 の「観測は近似で見せる」設計を壊さない。
> 観測面は現状維持、永続化面だけを無損失にする分離だからである。永続化面の
> 追加はワイヤ契約の拡張であり、`docs/dev/agent-cli-output-contract.md` の
> 観測用 `--json` 契約とは独立に扱う。

### P1 — 表層構文の認知負荷を下げる

- **VENT `^` の「次のソース単位をスキップ」規則の見直し。** `1 ^ 2 3 ADD → 4` の
  ような字句依存の挙動は、ブロックを引数に取る明示語（例: `OR-ELSE { ... }`）へ
  寄せると値ベースになり、空白・グルーピング・リファクタリングに対して頑健になる。
  正典変更を伴うため §2.2 の互換性方針に従って段階導入する。
- ベクトルリテラル内の名前解決（辞書状態により同一トークンがコードにもデータにも
  なる問題）を **原則データ化** へ統一する。`vector-nesting-role-redefinition.md`
  の役割固定方針と整合させる。
- **20〜30 語程度の "Core Profile"** を定義し、初学者の入口を用意する。
  `ajisai-minimal-core-identity.md` の最小コア議論と接続する。

### P2 — 契約推論と静的シグネチャの接続

既存の契約基盤（`rust/src/interpreter/word_contract.rs`、`check --contract`）を
活かし、任意（opt-in）の宣言を実行前検査に接続する。

- 例: `ADD1 : (Scalar -- Scalar) pure nil-free`、
  `NORMALIZE : (Vector<n> -- Vector<n>) may-nil`
- 目的: スカラー／ベクトル、NIL 可能性、純粋性・副作用を実行前に拒否できるように
  する。動的契約と静的型のあいだの空白を、既存基盤を壊さず埋める。

### P3 — 周辺環境

LSP、フォーマッタと構文診断、パッケージ名前空間、推移的依存、FFI/WASI、
再現可能なプロジェクトロック。優先度は中。P0/P1 の後に着手する。

### P4 — 対外的ポジショニング

「AI-first 汎用言語」より、**監査可能で厳密なベクトル計算を機械と人間の双方が
実行前に検査できる契約駆動言語** として表現する。

- 実装済みの強み（厳密数値 ＋ 三値失敗論 NIL/UNKNOWN/エラー ＋ 機械可読契約）と、
  将来構想（一般 exact-real）を明確に区別する。
- 呼称を「exact-real」から「exact rational & multiquadratic」（または
  「exact-by-default numeric with an extensible exact-real architecture」）へ是正する。
- `ajisai-use-language-identity.md` の言語アイデンティティ記録と整合させる。

## 4. 優先順位の考え方

機能追加ではなく、**①ホスト境界の無損失化（P0）→ ②表層の単純化（P1）→
③静的安全性（P2）** の順に投資する。外部評価が指摘するとおり現状の弱点は
「盛り込みすぎ」であって独創性不足ではない。したがって当面の投資先は
新機能ではなく **絞り込みと堅牢化** である。

## 参照

- `SPECIFICATION.html`（正典） §2.2 互換性方針、§2.3 no-hidden-truncation
  ファイアウォール、§8.6 content identity、VENT 節
- `rust/src/types/value_protocol.rs`（観測用ワイヤ形式の単一の真実）
- `rust/src/wasm_interpreter_bindings/wasm_value_conversion.rs`（境界変換）
- `rust/src/wasm_interpreter_bindings/wasm_interpreter_state.rs`
  （`collect_stack` / `restore_stack`）
- `src/gui/interpreter-state-persistence.ts`（永続化経路）
- `rust/src/interpreter/word_contract.rs`（契約推論、P2 の基盤）
- `docs/dev/agent-cli-output-contract.md`（観測用 `--json` 契約）
