# Verification Plan

## Objective
Define minimum verification evidence required for Ajisai changes.

## Required Baseline Checks (All PRs)
- `cargo fmt --check` (run in `rust/`)
- `cargo clippy --all-targets -- -D warnings` (run in `rust/`)
- `cargo test --all-targets --verbose` (run in `rust/`)
- `npm run check` (run at repository root, if JS/TS affected)

> Every baseline check is blocking. The `AJISAI_STRICT_QUALITY` repository
> variable that once staged the rollout has been removed: an advisory-by-default
> gate produces green checkmarks that do not certify anything. See the
> `quality-gate` job in `.github/workflows/test.yml`.

## Enhanced Checks
- `cargo +nightly llvm-cov --branch --workspace` when coverage instrumentation is
  available and relevant to changed Rust behavior. The toolchain is not
  incidental: `--branch` expands to `-Z coverage-options=branch`, which only a
  nightly rustc accepts, so the same command on stable fails rather than
  reporting less. Installing it needs
  `rustup toolchain install nightly --component llvm-tools-preview` and
  `cargo install cargo-llvm-cov --locked`.
- No coverage percentage is a merge threshold. The CI step runs the instrumented
  suite with `--no-report`, so what it certifies is that the workspace builds and
  passes under coverage instrumentation, not that any figure was met. Read a
  number by running `cargo +nightly llvm-cov report --branch --summary-only`
  afterwards.

## Level-Based Evidence Expectations
- **QL-A**
  - Baseline checks + targeted semantic/regression tests.
  - MC/DC-like checklist reviewed for modified boolean logic paths.
  - Traceability matrix row updates required.
- **QL-B**
  - Baseline checks + impacted unit/integration tests.
  - Traceability updates for requirement-to-test linkage.
- **QL-C**
  - Baseline checks; focused verification accepted if unaffected stacks are justified.
- **QL-D**
  - Appropriate subset of checks based on file types changed.

## Review Exit Criteria
A PR is merge-ready only when:
1. Relevant checks pass in CI.
2. PR quality checklist is completed.
3. Any open quality issue has explicit disposition.
