# Internal GOTO: tail-call elimination for guarded recursion

Status: prototype (non-canonical design note). The canonical language
definition remains `SPECIFICATION.html`; nothing here adds surface syntax or
changes observable value semantics.

## Motivation

Ajisai expresses iteration as recursion. The idiomatic loop is a user word
whose `COND` selects a clause that ends in a self-call:

```
{
  { [ 0 ] > | [ 1 ] - DOWN }   # recurse while value > 0
  { IDLE    | [ 'done' ] } COND # base case
} 'DOWN' DEF
```

Before this change, every such self-call recursed through
`execute_word_core`, growing `call_depth` and the native (Rust/WASM) stack one
frame per iteration. Two costs followed:

1. **A hard ceiling.** `MAX_USER_WORD_DEPTH` (256) caps the native stack to
   avoid a WASM trap, so `[ 300 ] DOWN` failed with *"recursion limit
   exceeded"* even though it is a constant-space loop.
2. **Per-iteration overhead.** Each iteration paid a full word-call setup:
   depth/budget checks, owning-dictionary save/restore, plan-set lookup, and a
   `call_stack` push/pop.

This is the classic case for **tail-call elimination**: a self-call in tail
position can reuse the current frame instead of allocating a new one — a
*backward jump*. The jump target is an internal program point, never a
source-level label, so the "GOTO considered harmful" critique (which is about
*named* jumps in human-read source) does not apply. The optimization lives
entirely below the semantic plane, like any other internal representation
(`SPECIFICATION.html` §5.2, §4.2.2).

## What is eliminated

Exactly one shape: a **guarded tail self-call** — a call to the word currently
executing, appearing as the last executable token of a `COND` clause body,
where that `COND` is itself the tail of the word's body. This is the
terminating loop idiom. Deliberately *not* eliminated:

- Bare unconditional self-recursion (`{ REC }`): no base case, so it keeps the
  legacy native-recursion path and its depth-limit error. Trampolining it would
  only convert one diagnostic (recursion limit) into another (step limit).
- Self-calls not in tail position (a result is still consumed afterwards).
- Mutual recursion between distinct words.

These keep the existing depth-bounded behavior.

## Mechanism

Four interpreter fields carry the trampoline (`interpreter_core.rs`):

| field | role |
| --- | --- |
| `tail_call_enabled` | master toggle; off via `AJISAI_NO_TAIL_CALL` or `set_tail_call_enabled(false)` |
| `tail_self_word` | resolved name of the frame eligible for self-tail-call elimination |
| `in_tail_context` | true while executing a section in the word's tail position |
| `tail_jump_pending` | raised by the deferral site, consumed by the trampoline loop |

Flow for one trampolined frame (`execute_word_core_inner`):

1. On entry, set `tail_self_word = Some(this word)` and enter a loop.
2. Run the body once. The tail op of the body — `CallBuiltin("COND")` on the
   compiled path (`compiled_plan.rs`) or the `COND` token on the plain path
   (`execute_guard_structure`, gated by `tail_token_is_cond`) — runs with
   `in_tail_context = true`.
3. `op_cond` (`control_cond.rs`) evaluates guards with tail context cleared
   (guards are not tail positions), then runs the winning clause body with the
   inherited tail context.
4. In that body, when `execute_section_core` reaches a final token that
   resolves to `tail_self_word`, it does **not** execute it. It leaves the
   computed arguments on the stack and sets `tail_jump_pending`.
5. `execute_cond_body`'s existing single-value contract carries those arguments
   out as the clause result; the trampoline loop sees `tail_jump_pending`,
   counts one execution step, and re-runs the body — a backward jump that never
   touched `call_depth`.

The residual stack after deferral *is* the next iteration's input, so no
special argument plumbing is needed: the loop idiom already reduces each clause
body to a single value.

## Invariants preserved

- **Termination (water level).** Each backward jump increments
  `execution_step_count`, so an unbounded guarded loop still raises
  `ExecutionLimitExceeded` (`SPECIFICATION.html` §5.3) instead of spinning.
- **Value integrity.** A jump moves no data and creates no value; `NIL`
  (bubble) vs `UNKNOWN` (stagnation) are untouched. The compiled and plain
  paths trampoline identically, so shadow validation (`shadow_validation.rs`)
  compares matching per-step residuals.
- **Word identity / provenance.** Trampolining is a property of the derived
  execution plan, not the source; content-addressed word identity
  (`SPECIFICATION.html` §8.6) is unchanged.
- **Depth guard for the rest.** Non-tail and unguarded recursion still hit
  `MAX_USER_WORD_DEPTH`, catching runaway native recursion before a WASM trap.

A related change: shadow validation is skipped on **recursive re-entry** of the
same word. The body is identical at every level, so validating once at the
outermost entry gives the same divergence coverage while removing a heavy
validation frame from the recursion chain (recovering native-stack headroom and
avoiding a redundant double execution per level).

## Measured effect

`cargo run --release --example tail_call_bench` (countdown loop):

```
-- Reach (deepest guarded tail recursion that completes) --
  OFF (native recursion):        255   (capped by MAX_USER_WORD_DEPTH guard)
  ON  (backward jump):       1000000   (O(1) native stack; step-budget bounded)

-- Per-iteration cost at depth 250 (both complete) --
  OFF: ~14.5 us/iter
  ON : ~12.3 us/iter
  speedup: ~1.18x
```

The headline is reach: guarded tail recursion becomes a real loop, bounded by
the step budget rather than a fixed native-stack depth. The per-iteration
speedup (~15–20%) is the saved word-call setup; the dominant per-iteration cost
remains the `COND` evaluation and exact-rational arithmetic.

## Tests

`rust/src/interpreter/tail_call_tests.rs` pins: depth past the native limit,
value-equality with the legacy path, the A/B depth contrast, the backward-jump
metric, the unguarded-recursion boundary, and step-budget termination.
