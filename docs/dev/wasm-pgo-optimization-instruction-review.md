# Ajisai WASM最適化（wasm-opt / PGO）指示書レビューと改訂版

## 結論

提示された指示書は、**速度最適化の方向性そのものは妥当**です。ただし、現在のAjisaiリポジトリにそのまま適用すると、CI破損・無効なPGO収集・wasm-optの再導入によるWASM破損のリスクがあります。したがって、以下の方針へ改訂します。

- `wasm-opt = ["-O3"]` への単純変更は、現行の `scripts/rebuild-wasm.sh` が `--no-opt` を指定しているため効果がありません。
- `--no-opt` は過去の `wasm-opt 108` と `wasm-bindgen 0.2.120` 出力の不整合回避として明示されているため、速度優先への変更前に `wasm-opt` バージョン固定とWASMスモークテストを必須にします。
- `cargo test -p ajisai-core --test perf_regression_tests` は現構成では不適切です。`perf_regression_tests` は統合テストではなく、ライブラリ内部の `#[cfg(test)]` モジュールです。
- ネイティブ収集プロファイルをWASMビルドへ流用するPGOは実験的な最適化として扱い、通常のデプロイCIに直結させず、まず専用ジョブまたは手動起動で検証します。
- PGO用ワークロード失敗を無条件に握りつぶすと、空または偏ったプロファイルを使う危険があります。フォールバックは「PGOを無効化して通常ビルドへ戻す」形に限定し、PGO有効時はプロファイル生成失敗を明示します。

## 現行指示書の問題点

### 1. `wasm-opt = ["-O3"]` だけでは現行ビルドに反映されない

`rust/Cargo.toml` には現在 `wasm-opt = ["-Os"]` が定義されていますが、ローカル/CIのWASM生成で使う `scripts/rebuild-wasm.sh` は `wasm-pack build ... --no-opt` を指定しています。このため、Cargo.tomlのwasm-packメタデータを `-O3` に変えても、実際の生成物には適用されません。

また、`--no-opt` は単なるサイズ最適化回避ではなく、既知の破損回避です。コメントには、ビルド環境の `wasm-opt 108` が `wasm-bindgen 0.2.120` 出力を誤コンパイルし、壊れたモジュールを生成したため無効化している、とあります。したがって、`--no-opt` を外す改修は、Binaryen/wasm-optのバージョン固定と生成WASMの実行検証をセットにする必要があります。

### 2. `perf_regression_tests` の呼び出し形式が実態と合っていない

`perf_regression_tests` は `rust/src/interpreter/mod.rs` から `#[path = "perf-regression-tests.rs"]` で取り込まれるライブラリ内部テストです。したがって、`cargo test --test perf_regression_tests` のような統合テスト指定ではなく、次のようにライブラリテストのフィルタとして実行します。

```bash
cd rust
cargo test --lib perf_regression_tests -- --nocapture
```

ベンチマークは `rust/benches/interpreter-performance-benchmarks.rs` にあるため、ベンチを使う場合は次の形式を基準にします。

```bash
cd rust
cargo bench --bench interpreter-performance-benchmarks
```

### 3. `llvm-profdata` の取得方法が曖昧

PGOの `*.profraw` はRustコンパイラが使うLLVMバージョンと整合する `llvm-profdata` でマージするのが安全です。OSパッケージの `llvm-profdata` をそのまま使うと、GitHub Actions上のapt版LLVMとrustc同梱LLVMのバージョン差で失敗する可能性があります。

CIでは `rustup component add llvm-tools-preview` を使い、`rustc --print sysroot` 配下の `llvm-profdata` を優先的に探す方針にします。

### 4. ネイティブPGOをWASMへ使う前提は検証ゲートが必要

ネイティブターゲットで収集したPGOを `wasm32-unknown-unknown` ビルドへ適用する方針は、理論上はLLVM IRレベルの情報を利用できる余地があります。しかし、ターゲット差・コード生成差・シンボル差により、期待どおり効かない、または警告/失敗になる可能性があります。

そのため、最初からデプロイ用ビルドに組み込まず、以下を満たした場合だけ有効化します。

- `profile-use` ビルドが警告なし、または既知の許容警告のみで完了する。
- 生成WASMがブラウザ/Vite環境でスモークテストに通る。
- `wasm-opt -O3` あり/なし、PGOあり/なしの比較ベンチで速度改善が確認できる。
- 生成サイズ増加が許容範囲内である。

### 5. ベンチ失敗時の「継続」は品質上のリスクがある

指示書は、ベンチの一部が失敗してもPGOデータ生成を継続するフォールバックを提案しています。しかし、失敗したワークロードを無視して `profile-use` すると、ホットパスを外したプロファイルや空に近いプロファイルを最適化へ投入する危険があります。

改訂版では次の扱いにします。

- PGO有効ビルドでは、プロファイル収集ワークロード失敗を原則失敗扱いにする。
- 手動検証中のみ、`ALLOW_PGO_FALLBACK=1` のような明示的フラグがある場合に限り、PGOなしの通常WASMビルドへ戻す。
- フォールバックした場合はCIログに明確に表示し、PGOビルド成功とは扱わない。

### 6. `panic = "abort"` は独立した挙動変更として扱う

`panic = "abort"` はサイズ・性能面で有利な可能性がありますが、Rust側のpanic伝播・テスト時の失敗表示・wasm-bindgen境界の挙動へ影響します。「AjisaiのSAFE修飾子があるから不要」と断定せず、リリースWASM限定の独立タスクとして、WASMスモークテストと既存Rustテストを通してから採用判断します。

## 改訂版実装指示書

### 目的

Rustコアロジックには触れず、WASM生成パイプラインだけを変更して、速度最適化の検証基盤を安全に追加する。デフォルトのデプロイCIを壊さず、`wasm-opt -O3` とPGOを段階的に有効化できる状態を作る。

### 変更対象

変更してよいファイルは次に限定する。

- `rust/Cargo.toml`
- `scripts/rebuild-wasm.sh`
- PGO補助スクリプトを新設する場合は `scripts/` 配下
- `.github/workflows/build.yml`
- 検証結果や運用手順を書く場合は `docs/dev/` 配下

`rust/src/**/*.rs` は変更禁止とする。

### Phase 1: `wasm-opt` 速度最適化の安全な再導入

1. `rust/Cargo.toml` のwasm-pack release profileは速度優先候補として次に変更する。

   ```toml
   [package.metadata.wasm-pack.profile.release]
   wasm-opt = ["-O3"]
   ```

2. ただし、`scripts/rebuild-wasm.sh` から `--no-opt` を外す前に次を実施する。

   - CIで使う `wasm-opt` のバージョンを固定または下限チェックする。
   - `wasm-opt --version` をログに出す。
   - 既知の破損が再発しないことを確認するWASMスモークテストを追加する。

3. 安全性が確認できるまで、`--no-opt` はデフォルト維持とし、速度優先ビルドは明示フラグで有効にする。

   例:

   ```bash
   AJISAI_WASM_OPT=1 bash scripts/rebuild-wasm.sh
   ```

4. `AJISAI_WASM_OPT=1` の場合だけ `--no-opt` を外して `wasm-pack` の `wasm-opt = ["-O3"]` を使う。未指定時は現行どおり `--no-opt` で安定ビルドする。

### Phase 2: PGO収集スクリプトの追加

PGOはデフォルトCIに直結せず、専用スクリプトで次のライフサイクルを実装する。

1. プロファイルディレクトリを初期化する。

   ```bash
   PGO_DIR="${TMPDIR:-/tmp}/ajisai-pgo"
   rm -rf "${PGO_DIR}"
   mkdir -p "${PGO_DIR}/raw"
   ```

2. rustc同梱LLVM toolsを優先して `llvm-profdata` を解決する。

   ```bash
   rustup component add llvm-tools-preview
   LLVM_PROFDATA="$(find "$(rustc --print sysroot)" -type f -name llvm-profdata -print -quit)"
   test -n "${LLVM_PROFDATA}"
   ```

3. ネイティブ収集ビルドとワークロードを実行する。

   ```bash
   cd rust
   export RUSTFLAGS="-Cprofile-generate=${PGO_DIR}/raw"
   cargo test --lib perf_regression_tests -- --nocapture
   cargo bench --bench interpreter-performance-benchmarks
   ```

   時間が長すぎる場合は、まず `cargo test --lib perf_regression_tests` だけを必須ワークロードにし、ベンチは手動検証またはnightlyジョブへ分離する。

4. プロファイルをマージする。

   ```bash
   "${LLVM_PROFDATA}" merge -o "${PGO_DIR}/merged.profdata" "${PGO_DIR}/raw"
   test -s "${PGO_DIR}/merged.profdata"
   ```

5. WASMビルドで使う。

   ```bash
   cd ..
   RUSTFLAGS="-Cprofile-use=${PGO_DIR}/merged.profdata" \
     AJISAI_WASM_OPT=1 \
     bash scripts/rebuild-wasm.sh
   ```

6. `profile-use` 時のミスマッチ警告を見落とさないため、CIログに `RUSTFLAGS` と `merged.profdata` のパスを表示する。ただし、プロファイルファイル自体は成果物に含めない。

### Phase 3: GitHub Actionsへの組み込み

1. 通常のPagesデプロイ用ビルドは、まず現行の安定経路を維持する。
2. PGO検証は次のいずれかで分離する。

   - `workflow_dispatch` で手動実行する専用ジョブ
   - `push` 時は非デプロイの検証ジョブとして実行
   - nightly/scheduledジョブ

3. PGOジョブには次のステップを追加する。

   - `rustup component add llvm-tools-preview`
   - `wasm-opt --version` の表示
   - PGOプロファイル収集
   - `llvm-profdata merge`
   - `AJISAI_WASM_OPT=1` と `-Cprofile-use=...` を指定したWASMビルド
   - `npm run build` またはViteビルドによるWASM読み込み確認

4. PGOジョブが安定し、速度改善が実測できた後にだけ、デプロイ用 `Build WASM artifacts` へ昇格する。

### Phase 4: 採用判定基準

以下をすべて満たした場合に、本番デプロイCIで `wasm-opt -O3` / PGOをデフォルト有効にする。

- `bash scripts/rebuild-wasm.sh` の通常ビルドが従来どおり成功する。
- `AJISAI_WASM_OPT=1 bash scripts/rebuild-wasm.sh` が成功する。
- `cd rust && cargo test --lib perf_regression_tests -- --nocapture` が成功する。
- `cd rust && cargo bench --bench interpreter-performance-benchmarks` がPGO収集用途で成功する、または代替ワークロードが明文化されている。
- `npm run build` と `npx vite build` が成功する。
- 生成WASMの実行スモークテストが成功する。
- 速度改善が測定上有意で、サイズ増加が許容範囲に収まる。

## 最小改修案

最初のPRでは、次だけを実装するのが安全です。

1. `scripts/rebuild-wasm.sh` に `AJISAI_WASM_OPT=1` フラグを追加し、未指定時は現行の `--no-opt` を維持する。
2. `rust/Cargo.toml` の `wasm-opt` を `-O3` に変更する。ただし実際に使うのは `AJISAI_WASM_OPT=1` の場合のみとする。
3. PGOは専用スクリプトまたは専用CIジョブとして追加し、デプロイ用ビルドにはまだ適用しない。
4. CIに `wasm-opt --version` とWASMスモークテストを追加する。

この順序なら、既存の安定ビルドを壊さず、速度最適化の検証を進められます。
