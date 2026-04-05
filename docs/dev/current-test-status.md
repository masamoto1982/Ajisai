# Current Test Status

- **Date**: 2026-04-05
- **Branch**: claude/fix-ajisai-tests-tDzBl
- **Commit**: HEAD (pre-fix)

## npm run check

**Result**: PASS (no TypeScript errors)

## cargo test

**Result**: FAILED (2 failures out of 77 tests)

### Failing Tests

| Test | File | Error |
|------|------|-------|
| `test_def_with_branch_guard` | `rust/tests/gui-interpreter-test-cases.rs:654` | `Unknown word: $` |
| `test_loop_guard` | `rust/tests/gui-interpreter-test-cases.rs:673` | `Unknown word: &` |

### Root Cause

Both tests use legacy syntax symbols (`$` for branch guard, `&` for loop guard) that were removed during the grammar migration to `COND`-based control flow. The tokenizer passes these symbols through as unknown words, and the interpreter has no handler for them.

### Resolution Plan

- **Plan A (chosen)**: Rewrite tests to use current `COND` syntax
- `$` (branch guard) → `COND` with guard/body pairs
- `&` (loop guard) → Replace with `COND`-based test (no loop construct in current spec)
- Add regression tests to verify old symbols produce explicit errors
