# MCP 日本語抑制ガイダンス強化 引き継ぎ書

作成日: 2026-08-17
対象セッション: 独立した 1 セッション（新規ブランチで着手する）
関連: [`mcp-claude-code-handoff.md`](./mcp-claude-code-handoff.md) /
[`mcp-host-profiles.md`](./mcp-host-profiles.md)

## 0. この文書の位置づけ

非正典。ツール説明文（`tools/mcp-server/index.js` / `mcp-quickstart.md`）の
文面調整に関する作業指示であり、`SPECIFICATION.html` の意味論には触れない。

**この文書は単独で読めるように書いてある。** 前セッションの会話履歴を
引き継がずに着手してよい。逆に、本書に書いていない判断を前セッションの
記憶から補わないこと。

## 1. 背景（実測済み・再現可能）

2026-08-17、`claude-opus-5` に対して実モデルでの2種類のライブ評価を実施し、
結果をコミット済み:

- `tools/mcp-server/eval/traces/claude-opus-5-full-corpus.json`
  （79 ケース × en/ja = 158 トレース、`capture-traces.js` で採取）
- `tools/mcp-server/eval/traces/claude-opus-5-repairs-full-corpus.json`
  （4 ケース × en/ja × 2 ターン = 8 トレース、`capture-repairs.js` で採取）

採点結果（`node score-traces.js eval/traces/claude-opus-5-full-corpus.json`）:

| 指標 | en | ja | 差 |
|---|---:|---:|---:|
| `irrelevantToolRate`（低いほど良い） | 0.10 (2/20) | 0.25 (5/20) | **+0.15** |
| `toolSelectionAccuracy` | 0.861 | 0.848 | -0.013 |
| `semanticSuccessRate` | 0.810 | 0.747 | -0.063 |

自己修正能力（`score-repairs.js`）は言語差ゼロ
（`diagnosisDrivenRepairRate` en/ja とも 0.75）で、健全。**本作業の対象外。**

## 2. 決まったこと

**本セッションの作業範囲は `irrelevantToolRate` の言語差の是正のみ。**
以下は前セッションで検討済み・優先度低と判断されており、蒸し返さない:

- **自己修正能力（capture-repairs）** — 言語差ゼロで健全。触らない。
- **`check` と `compute` の選択曖昧性**（`repair-stack-underflow` 1 件）—
  言語非依存（en/ja 両方で同じ揺れ）。本作業の対象外。
- **`irrelevant-weekday` と `irrelevant-explain-rpn` の restraint 失敗** —
  **en/ja 両方で失敗しており、言語差ではない。** 本作業のスコープ外。
  下記 §3 の3ケースと混同しないこと。

## 3. 証拠: 言語差を作っている3ケース

`node score-traces.js eval/traces/claude-opus-5-full-corpus.json` の出力で
`PASS ... [en] restraint` かつ `FAIL ... [ja] restraint` なのはこの3件のみ:

| ケース ID | en | ja | プロンプト（ja） |
|---|---|---|---|
| `irrelevant-currency` | PASS | FAIL | 100 米ドルを日本円に換算してください。 |
| `irrelevant-estimate` | PASS | FAIL | 500 ページの小説を読むのに、だいたいどれくらいかかりますか。 |
| `irrelevant-debug` | PASS | FAIL | JavaScript の `for (i=0;i<n;i++)` で最後の要素が飛ばされるのはなぜですか。 |

いずれも `eval/cases.json` で `expectedTool: null`（ツールを呼ばないのが
正解）と定義されている「Ajisai の管轄外」の質問。実際の ja トレースでの
`compute` 呼び出し引数（トレースファイルから抽出、要約せず原文のまま）:

```
irrelevant-currency [ja]:
  source: "[ 140 145 150 155 160 ] 100 MUL"
  → 為替レートを 140〜160 円/ドルの範囲ででっち上げ、Ajisai に掛け算させて
    「厳密な答え」であるかのような体裁を作っている。

irrelevant-estimate [ja]:
  source: "( 文庫1ページ約600字 → 500ページ = 300000字 ) ... { 60 DIV } MAP"
  → 1ページあたりの文字数・読字速度をでっち上げて計算している。

irrelevant-debug [ja]:
  source: "[ 0 5 ] RANGE\n[ 0 4 ] RANGE\n[ 0 5 ] RANGE LENGTH ..."
  → JavaScript のループ意味論という質問に対し、無関係な Ajisai の
    RANGE/LENGTH を実行して答えを構成しようとしている。
```

**パターン**: 実世界の事実（為替レート・読書速度・他言語の構文意味論）を
問う質問に対し、Ajisai の管轄外だと判断せず、値をでっち上げて Ajisai に
計算させ、厳密演算エンジンを通したことで答えに正確さの体裁を与えている。
en では同じ3ケースとも `(no tool)` で正しく restraint できている
（トレースファイルの `[en]` 側を参照すれば確認できる）。

## 4. 現状のツール説明文（何が足りないか）

- `tools/mcp-server/index.js` の `compute` ツール説明（158〜179行目付近）は
  末尾で "Out of domain: transcendentals, floats, I/O, and general-purpose
  programming." とだけ述べている。**実世界の事実・データを Ajisai に
  持ち込んで計算してはいけない、という言及が一切ない。**
- `tools/mcp-server/mcp-quickstart.md` 40行目の "Out of domain" 箇条書きも
  同様に一般的な範囲外事項の列挙のみで、「入力値そのものを捏造するな」
  という具体的な禁止が無い。
- **両ファイルとも英語のみで、言語別の文面分岐は存在しない。** つまり
  現状のガイダンスは en/ja のどちらのプロンプトに対しても同一のテキストが
  渡っている。にもかかわらず restraint の成否に言語差が出ている ——
  これは「日本語向けの追記が無いから」ではなく、**同一の（弱い）ガイダンス
  に対するモデルの反応が言語によって異なる**ことを意味する。
  したがって「日本語の一文を追加すれば直る」という単純な仮説を無条件に
  信頼しないこと。§5 の作業項目でこの点を検証すること。

## 5. 作業項目

優先順。

1. **`compute` の説明文と `mcp-quickstart.md` の "Out of domain"節に、
   実世界の事実・データの補完を禁じる一文を追加する。**
   例（文面はそのまま採用しなくてよい、内容の要件のみ）:
   「Ajisai has no external or real-world reference data — no exchange
   rates, no calendars, no natural-language semantics of other languages,
   no reading speeds. Do not invent plausible-looking numbers to answer a
   real-world-fact question through `compute`; if the question is not
   about a value already given or computable from first principles inside
   Ajisai's domain, answer directly without a tool call or say you don't
   know.」
   `index.js` の既存コメント（159〜168行目）の流儀に倣い、**この文書の
   §3 の実測（3ケース・具体的な捏造引数）を根拠として明記すること。**
   数値的根拠を書かない当てずっぽうの文面変更は、前回のレビューで
   問題視された「値だけ変えて根拠を書かない」パターンと同じ失敗になる。

2. **言語差そのものの仮説を最低限検証する。**
   §4 で述べたとおり、現状ガイダンスは言語非依存のテキストである。
   追記後に en 側の3ケースが（既に PASS なので）壊れていないか、
   ja 側の3ケースが PASS に転じるかを両方確認すること。ja だけ直って
   en が壊れる、あるいはどちらも変化しない場合は、文面の強さの問題ではなく
   モデル側のツール選択方針が言語によって異なる可能性を示唆する ——
   その場合は本書の範囲を超えるので、`docs/dev/` に新しい記録を残した上で
   一旦立ち止まり、ユーザーに次の方針を確認すること。

3. **`mcp-quickstart.md` を編集した場合は `sync-assets.js` で
   `assets/quickstart.md` を再生成する。** 手編集しないこと
   （`mcp-claude-code-handoff.md` §9 の既知の罠と同じ）。

4. **再度ライブモデル評価を行い、変更前後を比較する。**
   `capture-traces.js` は `ANTHROPIC_API_KEY`（または `ANTHROPIC_AUTH_TOKEN`
   / `ant auth login`）を Anthropic SDK の通常の解決順で必要とする。
   前セッションで一時発行された APIキー（3時間で失効する前提のもの）は
   本書作成時点で失効しているとみなすこと。再測定が必要な場合は
   ユーザーに新しいキーの発行を依頼すること。
   **キーの扱い**: Bash 呼び出し内の環境変数としてのみ使う
   （`export ANTHROPIC_API_KEY="..."`）。ファイルに書かない、コミットしない、
   自分の出力に一切エコーしない。この規律は前セッションで確立されたもので
   継続すること。

   手順:
   ```sh
   cd tools/mcp-server
   export ANTHROPIC_API_KEY="..."   # ユーザー提供、出力に出さない
   node capture-traces.js           # eval/traces/claude-opus-5-<timestamp>.json を生成
   node score-traces.js eval/traces/claude-opus-5-<timestamp>.json
   ```
   `eval:traces` npm script は `eval/reference-traces.json`
   （スコアラー自己検証用の固定フィクスチャ、実モデルではない）に
   パスが固定されているため、実測トレースの採点には使えない。
   `score-traces.js <path>` を直接呼ぶこと。

   比較対象は本書 §1 の表（`claude-opus-5-full-corpus.json` の値）。
   改善が確認できたら、採取したファイルを
   `tools/mcp-server/eval/traces/` 配下に意味の分かる名前
   （例: `claude-opus-5-after-restraint-guidance.json`。タイムスタンプの
   ままコミットしない — 既存ファイルの命名規則に合わせる）でコミットする。

5. **`README.md`（`tools/mcp-server/README.md`）の記述が古くなっていないか
   確認する。** 前セッションで「モデルベースラインが無い」という一文が
   実際には古いことを確認済み（追跡済みファイルと矛盾）。今回の追記でさらに
   古くなる場合は更新すること。

## 6. 触らないもの

- **`RuntimeLimits` / `host_profile_defaults.rs` の資源上限。**
  本作業とは無関係（別の完了済み作業、PR #1520 でマージ済み）。
  数値上限の話ではなく、ツール説明文というプロンプト面の話である。
- **英語側の restraint を弱める変更。** en は既に 0.10 で、追記後に
  悪化させないこと（§5-2 の検証で確認）。
- **`irrelevant-weekday` / `irrelevant-explain-rpn`。** 言語非依存の別問題。
  同じ変更で偶然直ることはあり得るが、これらを直すことを目的に文面を
  設計しないこと（目的がぼやけると根拠が書けなくなる）。
- **`eval/cases.json` のケース定義。** 追加・変更が必要だと判断した場合は
  このセッション内で完結させず、理由を記録した上でユーザーに確認すること。

## 7. コミット前の必須チェック

`mcp-claude-code-handoff.md` §7 と同じ一式（ツール説明文のみの変更でも
`npm run check:mcp-assets` と `npm run test:mcp` は必ず通すこと。前者は
`sync-assets.js` の出力が `assets/quickstart.md` と一致するかを検査する）。

```sh
cd tools/mcp-server
npm run check:mcp-assets
npm test
npm run eval:mcp
```

リポジトリルートの `npm run check` / `npm run lint` / `npm test` も
変更範囲に応じて実行すること。
