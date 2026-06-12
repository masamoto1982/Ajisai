# AI-First Competitive Upgrade Instructions（改修指示書）

Status: work order (non-canonical). Authority: `SPECIFICATION.html` only.
この文書は実装作業の指示書であり、Ajisai の意味論を定義しない。
本指示書と `SPECIFICATION.html` が矛盾する場合は `SPECIFICATION.html` に従う。

---

## 0. 目的と戦略

### 0.1 競争環境

AI-first 言語の競争が激化している（Zero / MoonBit / NanoLang / Vera ほか）。
外部の比較検証は共通して次の方法を取る:

1. AI エージェントに仕様だけ（または薄い文法ガイドだけ）を渡してコードを書かせる
2. 「1回で通るか」「修正回数」「トークン数」「エラーメッセージの質」「自己解決できるか」を測る
3. すべて CLI 上のループ（書く → 実行 → エラーを読む → 直す）で行われる

### 0.2 Ajisai の現状診断

| 領域 | 状態 |
|---|---|
| 意味体系（連分数 / NIL / UNKNOWN / 契約メタデータ） | 強い。実装済み |
| AI 向け診断素材（`DebugDiagnosis` / `AiDiagnosticPayload` / `nextChecks`） | 強い。実装済みだが WASM/JS 経由でしか見えない |
| VTU 観測カウンタ（17 個、`RuntimeMetrics`） | 実装済み。ただし集約スコアなし |
| **エージェントが headless で実行する手段（CLI）** | **存在しない。最大のボトルネック** |
| AI が最初に読む薄いガイド（SKILL.md 相当） | 存在しない |
| 省電力性の実証形式 | 存在しない（主張もまだしないこと） |

### 0.3 勝ち筋（ポジショニング）

「AI が初見で書きやすい言語」競争で NanoLang 等と正面衝突しない。
Ajisai が独占すべき象限は:

> **正しさを証明してから、速く・省電力に実行する AI-first 言語**
> （検証派 × 省電力派。既存言語のどれも占めていない）

ただしこの主張が成立する順序は固定である:
**(1) CLI → (2) 生成された SKILL.md → (3) energy proxy の CI 強制 → (4) 検証付きローダウン**。
1〜3 は既存コードの再配線でほぼ実現できる。4 のみが研究課題。

### 0.4 省電力主張に関する誠実性ルール（重要）

連分数 + BigInt の演算は、f64/i64 のハードウェア演算より 1 演算あたり高コストである。
この矛盾に正面から答えない限り「省電力」を看板にしてはならない。答えは二段構え:

- **ライフサイクル省電力**: AI 開発の総エネルギー = 実行コスト × 試行回数。
  強い意味体系と質の高い診断は再試行・再検証回数を減らす（修正回数が代理指標）。
- **実行省電力 = 検証付きローダウン（Phase 5）**: exactness が保存されることを
  証明できたブロックだけを機械語幅（i64/SIMD）に落とす。

実測（joule）するまで README / SPECIFICATION で「省電力である」と断定しない。
「省電力化に効く構造的指標を観測・強制する」とだけ言う。
カウンタ名・スコア名に `energy_saved` のような結果を断定する動詞を使わない
（`docs/dev/virtual-tensor-unit-design.md` の既存方針を維持）。

---

## 1. 全フェーズ共通の不変条件

1. **意味論を変えない。** すべての改修は observational / additive であること。
   既存プログラムの出力・NIL/UNKNOWN の挙動・Fraction の exactness を変えない。
2. `SPECIFICATION.html` は肥大化させない。新規文書は `docs/dev/`（設計・指示）
   または `docs/quality/`（品質規律）に置く。
3. WASM ビルド（`npm run build:wasm`）と Tauri ビルドを壊さない。
   新規 native 専用コードは `#[cfg(feature = "std")]` 等で隔離する。
4. 新規 Rust 実装ファイルは 500 行以下（テストファイルは例外）。
5. 既存検証をすべて通す:
   - `cargo test`（rust/ 配下）
   - `node scripts/check-formalization-coverage.mjs`
   - `scripts/check-semantic-firewall.sh`
   - manifest 生成スクリプトの再実行で差分が出ないこと
6. 各フェーズは独立した PR にする。1 PR に複数フェーズを混ぜない。

---

## 2. Phase 1 — `ajisai` CLI（最優先・他のすべての前提）

### 2.1 目的

AI エージェントが「書く → 実行 → 構造化エラーを読む → 直す」ループを
ターミナルだけで回せるようにする。これがないと外部ベンチマークの土俵に上がれない。

### 2.2 実装

- `rust/Cargo.toml` に `[[bin]] name = "ajisai"` を追加する。
  crate は既に `rlib` を含むので同一 crate 内 bin で良い。
  bin は `std` + `hosted`（default features）でビルドできること。`wasm` feature に依存しない。
- エントリポイント: `rust/src/bin/ajisai.rs`（500 行以下。必要なら
  `rust/src/cli/` モジュールに分割）。
- 依存を増やさない方針: 引数パースは手書きで足りる（clap 等は導入しない。
  Core dependency-light 方針を維持）。JSON 出力は既存の `serde_json` を使う。

### 2.3 コマンド仕様

```
ajisai run <file.ajisai> [--json]
ajisai check <file.ajisai> [--json]      # tokenize + parse + resolve のみ。実行しない
ajisai version [--json]
```

- 終了コード: 0 = 成功、1 = 言語エラー（diagnosis あり）、2 = CLI 使用法エラー。
- `--json` なしの場合は人間向けの簡潔なテキスト（最終スタックとエラーサマリ）。

### 2.4 `--json` 出力契約（最重要成果物）

トップレベル形状（camelCase。WASM バインディングの `diagnosis_to_js` と同じ命名規約）:

```json
{
  "schemaVersion": 1,
  "status": "ok | error",
  "stack": [ ... ],
  "diagnosis": {
    "when": "...", "why": "...", "summary": "...",
    "where": { "kind": "...", "word": "...", "module": "...", "dictionary": "..." },
    "evidence": [ "..." ],
    "nextChecks": [ ... ],
    "agreedPrefix": null
  },
  "errorFlowTrace": [ ... ],
  "aiDiagnostic": { "kind": "...", "recoverability": "...", "semanticArea": "...",
                    "semanticRole": "...", "algebraicFamily": "...", "nextChecks": [ ... ] },
  "runtimeMetrics": { "vtu": { ... 既存 17 カウンタ ... } }
}
```

- シリアライズ対象は既存の `DebugDiagnosis` / `AiDiagnosticPayload` /
  `ErrorFlowTrace` / `RuntimeMetrics`。**新しい診断概念を発明しない。**
  既存構造体に `serde::Serialize` を derive する（まだ無ければ）。
- 契約文書を新規作成: `docs/dev/agent-cli-output-contract.md`。
  各フィールドの意味、`schemaVersion` の更新規則、後方互換ポリシー
  （フィールド追加は minor、削除・改名は version bump）を明記する。

### 2.5 ホスト適合

- ネイティブ実行は `cargo test` で既に動いているので、CLI の仕事は
  ファイル入力 + 既存構造体の JSON 化 + ホストアダプタの提供のみ。
- `PRINT` 等の出力 → stdout（`--json` 時は `"output": [ ... ]` 配列に収集して
  JSON に含め、stdout を汚さない。stdout には JSON のみを書く）。
- Audio / Serial / GUI 依存の hosted word → panic せず、明確な diagnosis
  （`why: Environment`、recoverability: `fixCapabilityOrForce` 系）を返す。

### 2.6 受け入れ基準

- [ ] `cargo build --bin ajisai` が成功する
- [ ] `examples/*.ajisai` のうち audio/GUI 非依存の全ファイルが `ajisai run` で実行できる
- [ ] エラーを含むコードで `--json` 出力が valid JSON かつ `nextChecks` を含む
- [ ] `--json` 時、stdout に JSON 以外が混ざらない（パイプ安全）
- [ ] 終了コードが仕様どおり
- [ ] `docs/dev/agent-cli-output-contract.md` が存在し、出力例を含む
- [ ] WASM ビルドが引き続き成功する

---

## 3. Phase 2 — SKILL.md（手書き禁止・生成すること）

### 3.1 目的

AI エージェントが「これだけ読めば書ける」薄い実用層を提供する。
ただし手書きすると仕様との乖離が必ず起きるため、**既存の manifest 生成
パイプラインの延長として生成する**。これ自体が他言語の手書き SKILL.md に対する
差別化になる（「AI 向けガイドが仕様から機械的に導出され、CI で鮮度検証される」）。

### 3.2 実装

- 新規スクリプト: `scripts/generate-skill-md.mjs`
  - 入力: `docs/word-manifest.json`、`examples/*.ajisai`、
    およびスクリプト内に保持するキュレーション・データ
    （頻出エラー → 修正方法の対応表、禁止パターン、canonical examples）
  - 出力: リポジトリルートの `SKILL.md`
- `package.json` に `generate:skill` と `check:skill`（再生成して差分が出たら fail）
  を追加。既存の manifest チェックと同列に CI へ組み込む。

### 3.3 SKILL.md の内容要件

順序も含めて固定する。**仕様の要約ではなく、書くためのプロトコル**にする:

1. 最小実行方法（`ajisai run file.ajisai --json` と JSON のどこを読むか）
2. 最小構文: スタック規律、vector literal `[ ... ]`、code block `'...'`、
   user word 定義、コメント
3. 制御とイテレーション: COND 系、MAP / FILTER / FOLD（quoted block の渡し方）
4. NIL の扱い方（発生条件と fallback パターン）
5. UNKNOWN / 三値論理の扱い方（`agreedPrefix` の読み方を含む）
6. canonical examples 20 個程度（examples/ から自動抽出 + キュレーション。
   各例は「コード → 期待されるスタック」のペア）
7. よくあるエラー 10 個程度: 実際のエラー JSON 断片 → 修正方法
8. 禁止パターン（やりがちな誤り。他言語の構文の持ち込み等）
9. word 早見表（word-manifest.json から生成: surface / category / 1 行説明）

トークン予算: §1〜§8 で約 3,000 トークン以内を目標。§9 の早見表は末尾に置き、
冒頭に「迷ったら §9 を grep せよ」と書く。

### 3.4 受け入れ基準

- [ ] `SKILL.md` がルートに存在し、`node scripts/generate-skill-md.mjs` で再生成できる
- [ ] `check:skill` が CI（既存のチェック群と同じ場所）に組み込まれている
- [ ] SKILL.md 内の全コード例が `ajisai run` で実際に通る
      （生成スクリプトが CLI を呼んで検証する。Phase 1 への依存はここ）
- [ ] 仕様にない word・構文が SKILL.md に現れない（manifest 由来であることで担保）

---

## 4. Phase 3 — energyProxyScore と「エネルギー回帰テスト」

### 4.1 目的

VTU の 17 カウンタを単一の決定的スコアに集約し、
「同じ意味のまま proxy cost が増えたら CI が落ちる」規律を作る。
これにより「省電力志向」が口先でなく機械的に強制された性質になる。

### 4.2 実装

- 新規モジュール: `rust/src/interpreter/energy_proxy.rs`（500 行以下）
  - `RuntimeMetrics` から `energyProxyScore: u64` を決定的に計算する純関数
  - 重みは整数係数の重み付き和。初期値の例（実装時に調整可、ただし文書化必須）:
    flatten/rebuild された要素数と allocated_elements を重く、
    SIMD / bulk kernel / projected broadcast の使用は控除（または別軸で報告）
  - スコアとともに `suggestions: Vec<String>`（fusion 候補の分断、不要 rebuild 等、
    既存カウンタから機械的に導ける助言）を返す
- 重みの定義と「これは joule ではない」という明示を
  `docs/quality/energy-proxy-score.md` に置く。重みを変えたら
  `proxyVersion` をインクリメントする規則を書く。
- CLI の `--json` 出力 `runtimeMetrics.vtu` に `energyProxyScore` /
  `proxyVersion` / `suggestions` を追加（Phase 1 の schemaVersion 規則に従う）。
- 回帰テスト: `rust/src/interpreter/energy_proxy_regression_tests.rs`
  - 固定プログラム 10 本程度（vector map chain / tensor broadcast /
    same-shape arithmetic / projected broadcast / sparse candidate / NIL 混在 tensor 等）
  - 各プログラムの baseline スコアをテスト内に記録し、
    「出力が同一」かつ「スコアが baseline を超えない」ことを assert
  - 既存 `perf_regression_tests.rs` と同じ流儀に合わせる

### 4.3 受け入れ基準

- [ ] 同一プログラム・同一入力でスコアが決定的（複数回実行で不変）
- [ ] 回帰テストが `cargo test` に含まれ、意図的に劣化させると fail することを確認済み
- [ ] `docs/quality/energy-proxy-score.md` に重み・proxyVersion 規則・
      「proxy であって joule ではない」断り書きがある
- [ ] CLI JSON にスコアと suggestions が出る

---

## 5. Phase 4 — エージェント・ベンチマークハーネス

### 5.1 目的

外部記事と同一手法のベンチマークを自前で再現し、結果（負けを含む）を公開する。
次の比較記事に「省電力派」枠で載るための素材を作る。

### 5.2 実装

- 新規ディレクトリ: `bench/agent-suite/`
  - `tasks/json-parser.md`、`tasks/bank-account.md`: 外部記事と同等の仕様書
    （言語非依存の要求仕様 + 受け入れテストケース 22 / 14 件相当）
  - Ajisai が有利な追加題材: `tasks/exact-rational-calculator.md`、
    `tasks/three-valued-logic.md`（UNKNOWN を含む）、`tasks/nil-fallback-pipeline.md`、
    `tasks/energy-refactor.md`（同一出力のまま energyProxyScore を下げる）
  - `protocol.md`: 計測手順の固定。
    A: SKILL.md なし（SPECIFICATION のみ）/ B: SKILL.md あり。
    各試行は独立したエージェントセッション。記録項目は
    1 回で pass / 修正回数 / 最終行数 / トークン数 / 自己解決 / エラーの質 /
    energyProxyScore。
  - `results/`: 結果記録テンプレート（`results/TEMPLATE.md`）
- 検証ランナー: 各 task に `verify.sh`（`ajisai run --json` の出力を
  期待値と突き合わせる。jq か node ワンライナーで良い）

### 5.3 厳守事項

- **結果を捏造しない。** ハーネスと task と検証スクリプトだけを整備する。
  実測は独立セッションのエージェントで行い、結果はそのまま記録する。
- 負けた項目も削除せず公開する。

### 5.4 受け入れ基準

- [ ] 全 task に検証スクリプトがあり、人手の判定なしで pass/fail が出る
- [ ] `protocol.md` だけ読めば第三者が同条件で再実施できる

---

## 6. Phase 5 — 検証付きローダウン（設計のみ。実装は別途レビュー後）

### 6.1 目的

VTU の最終形。「exactness を捨てて速くする」のではなく
「**exactness が保存されることを証明してから機械語幅に落とす**」層を設計する。
これが Ajisai だけの研究的差別化（検証派 × 省電力派の融合）になる。

### 6.2 このフェーズでやること（実装しないこと）

設計文書 `docs/dev/vtu-verified-lowering-design.md` の作成のみ。含めるべき内容:

1. 対象: `QuantizedBlock` のうち `infer_vtu_hint` が StrongCandidate の純粋ブロック
2. 証明義務の定義: ブロック内の全中間値が i64（または固定ビット幅）に収まり、
   結果が Fraction/CF 経路と bit-exact に一致する条件
   （値域解析 + 分母 1 保証 + オーバーフロー解析）
3. 失敗時の手順: 証明できなければ無条件に既存 exact 経路へフォールバック。
   推測的ローダウン（実行してから検算）は採用しない
4. 観測: ローダウンされたブロック数 / 棄却理由をカウンタとして追加する案
5. 既存の `trace-quant` / `force-no-quant` feature との関係
6. 段階的ロールアウト計画と、各段階で energyProxyScore および
   criterion ベンチでどう効果を測るか

### 6.3 厳守事項

- この設計がレビューされ承認されるまで、ローダウン本体を実装しない。
- 設計文書にも「結果の数値が 1 bit でも変わる最適化は採用しない」と明記する。

---

## 7. Phase 6（任意）— Ajisai MCP サーバー

CLI（Phase 1）の薄いラッパーとして `tools/mcp-server/` に Node 製 MCP サーバーを置く。

- ツール 3 つのみ: `run`（= `ajisai run --json`）、
  `explain_word`（word-manifest.json を引く）、`skill`（SKILL.md を返す）
- 依存は MCP SDK のみ。ロジックを持たせない（すべて CLI と manifest に委譲）
- README に Claude Code / 他エージェントからの接続手順を 5 行で書く

受け入れ基準: ローカルで Claude Code に接続し、3 ツールが動くこと。

---

## 8. 実施順序と PR 分割

| PR | 内容 | 依存 |
|---|---|---|
| 1 | Phase 1: CLI + 出力契約文書 | なし |
| 2 | Phase 2: SKILL.md 生成 + CI チェック | PR 1 |
| 3 | Phase 3: energyProxyScore + 回帰テスト | PR 1 |
| 4 | Phase 4: ベンチマークハーネス | PR 1, 2, 3 |
| 5 | Phase 5: ローダウン設計文書 | なし（並行可） |
| 6 | Phase 6: MCP サーバー | PR 1, 2 |

各 PR の完了条件は §1 の不変条件 + 各フェーズの受け入れ基準のすべて。

## 9. やってはいけないこと（再掲・要約)

- SPECIFICATION.html を肥大化させること
- SKILL.md を手書きすること
- 実測なしに「省電力である」と断定する文言を README 等へ入れること
- 既存プログラムの出力・exactness・NIL/UNKNOWN 挙動を変えること
- ベンチマーク結果の捏造・不利な結果の非公開
- Phase 5 のローダウンを設計レビュー前に実装すること
- Core への不要な依存追加（clap 等）
