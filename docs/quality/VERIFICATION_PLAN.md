# Verification Plan

## Objective
Define minimum verification evidence required for Ajisai changes.

## Required Baseline Checks (All PRs)
- `cargo fmt --check` (run in `rust/`)
- `cargo clippy --all-targets -- -D warnings` (run in `rust/`)
- `cargo test --all-targets --verbose` (run in `rust/`)
- `npm run check` (run at repository root, if JS/TS affected)

## Enhanced Checks
- `cargo llvm-cov --branch --workspace` when coverage instrumentation is available and relevant to changed Rust behavior.

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
