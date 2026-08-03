# Ajisai β版改修 — Claude Code引継書

更新日: 2026-08-03  
対象Issue: GitHub Issue #1429  
基準文書: `docs/dev/beta-freeze-2026-08.md`  
正典: `spec/`配下および生成物`SPECIFICATION.html`

## 1. 引継ぎ時点のGit状態

- repository: `/workspace/Ajisai`
- branch: `work`
- α基準commit: `ebb66a5f9d14a6c8d6610488724e476e652abc35`
- Phase 1 commit: `86b5c80 feat(vocabulary): establish beta tier contracts`
- quality-gate修正commit: `6fa2d1d fix(ci): rebuild CLI before skill freshness checks`
- 引継書追加前のHEAD: `6fa2d1d`

このbranchは、過去に作成したPhase 1〜4の巨大な一括差分を破棄し、`ebb66a5`からPhase 1だけを作り直したものである。現在のruntime inventoryは意図的に70語のままで、13語の実削除はまだ行っていない。

## 2. 現在実装済みの範囲

### Phase 1 — 契約と分類

以下を実装済み。

1. `docs/dev/beta-freeze-2026-08.md`を追加。
2. `spec/words.json`へ`migration.betaFreezePhase: 1`を追加。
3. β版で残す57語へ`vocabularyTier`を付与。
   - `kernel`: 35語
   - `standard`: 22語
4. Standard Wordsへ`standardKind`を付与。
   - `shorthand`
   - `namedPattern`
   - `algorithm`
   - `operational`
5. `docs/formalization-coverage.json`へ以下を追加。
   - derivable Standard: `standard_relation: "derivable"`
   - operational Standard: `standard_relation: "operational"`
   - operational Standardの`native_retention_reason`
6. `scripts/check-minimal-core.mjs`を35/22構造の検査へ変更。
7. Phase 1に限り、削除予定13語が未分類で残る70語inventoryを明示的に許可。
8. `scripts/check-word-schema-migration.mjs`へPhase 1 metadata検査を追加。
9. `spec/words.schema.json`へ`vocabularyTier`、`standardKind`、`betaFreezePhase`を追加。

### Quality Gateの追加修正

`scripts/generate-skill-md.mjs`は従来、`rust/target/debug/ajisai`が存在するとbuildを省略していた。branchの巻き戻し後に古いCLIを再利用し、正常なexampleを失敗と判定する事象をローカルで再現したため、明示的な`AJISAI_BIN`がない場合は必ず次を実行するよう変更した。

```bash
cargo build --bin ajisai
```

Cargoのincremental buildを利用するため、binaryが最新なら短時間で完了する。

## 3. 現在の重要な移行状態

`spec/words.json`はまだ70 canonical Wordsを含む。内訳は次の通り。

- `vocabularyTier=kernel`: 35
- `vocabularyTier=standard`: 22
- Phase 2で削除予定の未分類Words: 13

削除予定13語:

```text
CEIL SIGN
INSERT REPLACE REMOVE SPLIT REORDER UNIQUE CONTAINS
STARTS-WITH? ENDS-WITH? CHR
EAT
```

したがって、このbranchはβ release candidateではない。`check-minimal-core`成功時にも次を出力する。

```text
[minimal-core] phase 1: 13 alpha Words remain explicitly pending removal.
```

## 4. 検証済みコマンド

以下は引継ぎ前に成功済み。

```bash
node scripts/check-minimal-core.mjs
npm run word-schema:check
npm run check:formalization-coverage
npm run check:semantic-firewall
npm run check:file-size
npm run check:reading-surfaces
npm run specification:check
npm run semantic-kernel:check
npm run word:reference:check
npm run core-word-docs:check
npm run word-registry:check
npm run word:manifest:check
npm run check:runtime-metadata
npm run check
npm run lint
npm test
cargo fmt --manifest-path rust/Cargo.toml --check
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml --all-targets --quiet
```

Rust test結果は832 passed、1 ignored。Vitestは13 files、231 tests passed。

`npm run check:skill`は次の2経路で成功を確認済み。

1. CLIを削除してからのrebuild path。
2. 直後のincremental path。

## 5. 環境上の制約

### Remote mainを取得できない

このcheckoutにはGit remoteが設定されていない。さらに次のfetchはproxyのHTTP 403で失敗した。

```bash
git fetch https://github.com/masamoto1982/Ajisai.git main
```

したがって、最新のGitHub `main`を取得したrebaseおよびGitHub上で報告された実コンフリクトの再現は、この環境では行えていない。Claude Code側では最初にremoteを設定・fetchし、最新`main`へrebaseすること。

### WASM target / wasm-pack

この環境では`wasm32-unknown-unknown` targetの取得と`wasm-pack`利用を完了できなかった。WASM gateはClaude Code側またはCIで必ず再実行すること。

```bash
rustup target add wasm32-unknown-unknown
cargo check --manifest-path rust/Cargo.toml --features wasm --target wasm32-unknown-unknown
npm run build:wasm
```

### PR作成

API上のPR metadata記録は試行したが、ユーザーから「PRが作成できない」と報告されている。実GitHub PRの存在を前提にせず、Claude Code側でbranch pushとPR作成を行うこと。

## 6. Claude Codeで最初に行う手順

```bash
cd /workspace/Ajisai
git status --short --branch
git log --oneline --decorate -5

# remoteがなければ設定
git remote add origin https://github.com/masamoto1982/Ajisai.git
git fetch origin main

# 専用branchへ移すことを推奨
git switch -c beta-freeze-phase1
git rebase origin/main
```

rebase conflictでは、最新`main`側ですでに同等変更が入っていないかを必ず確認する。特に競合可能性が高いファイルは次の通り。

- `spec/words.json`
- `spec/words.schema.json`
- `docs/formalization-coverage.json`
- `scripts/check-minimal-core.mjs`
- `scripts/check-word-schema-migration.mjs`
- `scripts/generate-skill-md.mjs`

競合解消後は、生成物を手編集せずgenerator/check commandを使うこと。

## 7. Phase 1 PRの推奨scope

Phase 1 PRでは現在の6つの契約・metadataファイルとquality-gate修正だけを扱う。次を混入させない。

- 13語のruntime削除
- HostProtocolV1削除
- persistence migration削除
- version変更
- 記号糖の再設計
- Word Manifest schema変更

PR title案:

```text
feat(vocabulary): establish beta tier contracts
```

## 8. Phase 1 merge後の実装順序

### Phase 2 — 13語削除

別branch / 別PRで実施する。正典entryだけを削除してexecutorを残すことは禁止。

各Wordについて同一PR内で行うこと:

1. `spec/words.json` entry削除。
2. generated `WordId` / registry再生成。
3. runtime dispatch arm削除。
4. compiled path削除。
5. 専用executor/helper/module削除。
6. alias/canonicalization path削除。
7. hover/LOOKUP/REFLECT到達不能化。
8. law/unit/integration/conformance test更新。
9. examplesと公開docs更新。
10. 未使用error messageとmigration削除。

### Phase 3 — Standard契約

- 16 derivable WordsへKernel-only executable witnessを追加。
- 6 operational Wordsへ訪問順、短絡、effect、ERROR復元、resource ceiling、原子性のlaw testを追加。
- 全22語のconformance、runtime/compiled一致、REFLECT name一致を確認。

### Phase 4 — 互換層削除

- HostProtocolV1 schema/golden/test/adapter削除。
- GUI live pathをcurrent protocolへ接続。
- no-op module/import API削除。
- execution-mode/hedged-trace互換削除。
- 旧snapshot/import/export reader削除。

### Phase 5 — 公開面とversion

- 全生成物再生成。
- `57 canonical Words / 35-Word Semantic Kernel`へ表記統一。
- package/crate/Tauriを`0.2.0-beta.1`へ同期。
- α基準commitとβ境界commitをREADME/Specificationへ記録。

## 9. 注意事項

- `core_tier`と`vocabularyTier`は別軸。統合・renameしない。
- Standard Wordsをprelude、compatibility bundle、二級Wordとして扱わない。
- aliasをcanonical countへ加えない。
- 新しい記号糖を本改修へ混入させない。
- `scripts/generate-skill-md.mjs`のCLI build修正をrebase時に落とさない。
- `npm run primitive:test-map`はchecked-in `docs/primitive-test-map.json`を書き換えるため、実行後の差分を確認する。
- 中間Phaseをβ版としてreleaseしない。

## 10. 完了判定

Claude CodeはPhaseごとにPRを分け、最終的に`docs/dev/beta-freeze-2026-08.md`の品質gateと完了条件をすべて満たしたcommitだけをβ境界とすること。
