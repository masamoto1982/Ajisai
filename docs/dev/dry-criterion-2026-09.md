# DRY 原則の見直し（2026-09）— 「重複」を形ではなく知識で測る

> Status: **Non-canonical / 方針記録（`[方針記録]`）.** 本書は Ajisai の意味論を
> 一切定義しない。正典は `spec/` 配下の各ソースと、そこから生成される
> `SPECIFICATION.html` のみ。本書は実装工学の判断基準と、その基準で行った
> 一回の点検結果を記録する。

## 0. 契機

外部記事「DRY原則の致命的な誤解。一次情報から読み解く「あえて共通化しない」が
正解になる理由」（<https://zenn.dev/kouji0705/articles/075390df55cdf8>）を受けた
見直し。記事が依拠する一次情報は `The Pragmatic Programmer` の DRY 原則本文
——*every piece of knowledge must have a single, unambiguous, authoritative
representation within a system*——であり、20周年版で著者自身が「DRY は
**知識と意図**の重複を指す。コードの見た目が同じであることではない」と補足した
点が論旨の中心である。

本リポジトリの `docs/dev/specification-implementation-rules.md` は、この見直しの
前まで Advisory に「Prefer small helper extraction for duplicated control
scaffolding」と書いていた。これは重複を**形**で測る書き方で、記事が誤解として
挙げる側の表現だった。同ファイルに **The DRY criterion** 節を追加し、この行を
「同じ決定を二箇所が持っているときに抽出する」に置き換えた。

## 1. 判断基準

採用した問いは三つ。詳細は
`docs/dev/specification-implementation-rules.md` の **The DRY criterion**。

1. 一方が変わるとき、他方も**同じ理由で同じコミットで**変わらなければならないか。
   → はい：知識は一つ。表現も一つにする。いいえ：たまたま今一致している別々の
   知識。分けたままにする。
2. どちらが正典か、読み手に分かるか。分からなければ、現時点で一致していても
   すでに乖離している。
3. 二つ目の写しは何のためにあるか。生成された射影（`spec/*.json` → レジストリ・
   ドキュメント・Specification）は二つ目の表現ではない。手で保つ言い直しは
   二つ目の表現である。

両方向にコストがある。値・規則・出力形式の**言い直し**は黙って乖離する。
無関係な二つの決定に**共通ヘルパを被せる**と決定同士が結合し、片方の変更が
もう片方に引数として現れる。

## 2. 点検結果

### 2.1 正しく効いている側（変更なし）

- **`spec/` からの生成**。語彙・結果空間・文法・停止性は `spec/*.json` を単一の
  源とし、`scripts/generate-*.mjs` が Rust レジストリ・`docs/word-reference.md`・
  `SKILL.md`・`SPECIFICATION.html` へ射影する。`--check` 付きの同一スクリプトが
  乖離を落とす。これは基準 3 の「射影」に当たり、重複ではない。
- **やむを得ない二重表現に門を立てている例**。`src/gui/core-word-name.ts` は
  GUI が実行時に spec を読めないため canonical 名の文法を
  `spec/words.schema.json` から言い直しているが、`core-word-name.test.ts` が
  `spec/words.json` 自体に対して述語を突き合わせる。理由もコメントに書かれている。
- **あえて共通化しない判断を記録している例**。
  `rust/src/interpreter/word_outcome_vocabulary.rs` は冒頭で、`word_contract.rs`
  の契約推論と**意図的に独立**であること（状態を共有しない並行実装であること）を
  宣言している。記事の「あえて共通化しない」がそのまま実践され、しかも読み手が
  見落としと誤解できない形で残されている。
- **同じ値だが別の知識**。`tools/mcp-server/index.js` の
  `LIMITS.executionSteps: 100_000` は、インタプリタが以前使っていた既定値と
  同じ数値だが、MCP ホスト自身の方針であり言語側の写しではない。統合しては
  ならない側。

### 2.2 見つかった知識の重複（本見直しで修正）

**(a) 既定ステップ予算の言い直しが実際に乖離していた。**
権威ある表現は `rust/src/interpreter/host_profile_defaults.rs` の
`DEFAULT_MAX_EXECUTION_STEPS` で、ホスト時間予算から**導出**される
（`DEFAULT_HOST_TIME_BUDGET_MS` × 実測フロアレート）。定数自身の doc comment が
「以前は 100,000 だった」と述べているとおり値は動いている。にもかかわらず、
値を数字で書き写した箇所が wasm 境界の両側に 5 件残り、すべて旧値
100,000 を主張し続けていた。

- `rust/src/wasm_interpreter_bindings/wasm_interpreter_state.rs`
  （およびそこから射影される `src/wasm/generated/ajisai_core.d.ts`）
- `src/platform/platform-adapter.ts`
- `src/workers/interpreter-snapshot.ts`
- `src/gui/interpreter-execution-utils.ts`
- `src/wasm-interpreter-types.ts`

修正は同期ではなく**二つ目の表現の削除**。各所は値ではなく定数名を指す。数字を
書かなければ乖離できない。

なお wasm バンドルは `src/wasm/generated/` と `tools/mcp-server/wasm/generated/`
の二つがコミットされており（web ターゲットと nodejs ターゲット）、どちらも同じ
Rust doc comment を写している。これは基準 3 の射影であり重複ではない——ただし
コミット済みの射影なので、Rust 側の文言変更に合わせて両方を更新した（CI は
再生成して差分が出れば warning を出す）。

**(b) 診断の読み取り形式が二つあった。**
`[DIAGNOSIS]` 見出し・`Q1 when:` / `Q2 where:` / `Q3 why:`・`next:` 行という
形式は提示の契約——読み手が一度覚えれば全ての拒否を同じ手順で読める——であり、
一つの知識である。これが二箇所にあった。

- `src/gui/execution-controller.ts`：プロトコルの `ProtocolDiagnosis` から描画。
- `src/gui/interpreter-execution-utils.ts`：実時間停止（ワーカーを外から停める
  ため診断が結果に乗ってこない唯一の拒否）用に、同じ形式を文字列リテラルで
  手書き。

見た目が似ているだけの偶然ではない。`Q3 why:` の改名や第四の問いの追加は一方を
動かして他方を残す。修正として `src/gui/diagnosis-report.ts`
（`renderDiagnosisReport`）を単一の描画器として切り出し、実時間停止側は
他と同じく `ProtocolDiagnosis` を組んで通す。ホスト固有の天井行だけは
`extraLines` として渡す——観測値を持たず、帰属させる Word もない、本当に
その拒否だけの知識だからである。副産物として、手書きの英語行では持てなかった
`ja` 側が四つの next-step すべてに付いた。形式の断定は
`src/gui/diagnosis-report.test.ts` の一箇所に集約した（旧リテラルとの出力一致も
そこで固定）。

**(c) 永続化レコードの形が同じディレクトリで二度書かれていた。**
`src/platform/web/web-persistence.ts` が `TableData` / `InterpreterState` を
ローカル `interface` で定義していたが、これは同ファイルが既に import して
`exportAll` でキャストにも使っている `ExportData` の該当部分を一字ずつ言い直した
ものだった。写しの側は `readonly` 修飾を落としており、`ExportData` に項目が
増えても書き込み側は**そのまま通り、黙って項目を落とす**。隣の
`src/platform/tauri/tauri-persistence.ts` は既に `ExportData['interpreterState']`
から導出していた。`ExportData['tables'][number]` /
`NonNullable<ExportData['interpreterState']>` に置き換えた。

### 2.3 見つかったが本見直しで手を付けなかったもの

`scripts/` 配下の 30 余りのゲートで、リポジトリルートの解決方法が二通り混在する
（`resolve(repoRoot, path)` と、cwd 依存の裸の相対パス `'spec/words.json'`）。
「ルートがどこか」は一つの知識で、答えが二つある状態。ただし現状 CI と
`package.json` からは常にルートで起動されるため観測される故障はなく、修正は
ゲート全ファイルへの機械的な変更になる。形式の統一だけを目的とした変更は
Mandatory の「意味変更と構造整理を分離する」に従い、別変更として扱う。

## 3. 再発防止

(a) の類は「値を書き写さない」で構造的に閉じる（書いていないものは乖離しない）。
(b) は形式の断定を `diagnosis-report.test.ts` の一箇所に集約したので、形式変更は
そこで一度落ちる。(c) は型が導出になったため `tsc` が検出する。

いずれも新しいゲートスクリプトを追加していない。本見直しで実際に乖離していた
のは「値を書き写した」件だけであり、これは書き写しをやめれば消える種類の問題で、
検査を増やして監視する対象ではない。
