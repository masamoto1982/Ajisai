# Compiling COND clause guards and bodies

Status: prototype (non-canonical design note). Final step of the internal-GOTO
series (`internal-goto-tail-call.md`, `internal-goto-cond-dispatch.md`,
`internal-goto-literal-vectors.md`). No surface syntax or value-semantics
change; the canonical definition remains `SPECIFICATION.html`.

## Motivation

The earlier steps compiled the recursion loop's *scaffolding* — the backward
jump, the precomputed clause dispatch — and made literal vectors compilable. But
the loop's actual work still ran interpreted: `op_cond` evaluated each clause's
guard and ran the winning body through `execute_section_core`, walking the token
stream (`[ 0 ] >`, `[ 1 ] - DOWN`) every iteration. That interpretation was the
dominant per-iteration cost the previous PRs deliberately left untouched.

## Mechanism

`CondClause` now carries optional compiled sub-plans:

```
pub struct CondClause {
    pub guard: Arc<[Token]>,
    pub body:  Arc<[Token]>,
    pub guard_plan: Option<Arc<CompiledPlan>>,
    pub body_plan:  Option<Arc<CompiledPlan>>,
}
```

When `lower_cond_dispatch` builds a `CondDispatch` (and `compiled_clause_enabled`
is set), it compiles each clause's guard and body with `compile_clause_plan`,
which runs the section through the same `compile_one_line` + lowering used for
word bodies — so literal vectors, nested `COND`s, and word calls inside the
clause all compile. Compilation only sticks if the section compiles to more than
fallbacks (`plan_is_all_fallback` ⇒ `None`, keeping the interpreter path).

At runtime, `evaluate_guard_isolated` and `execute_cond_body` run the sub-plan
via `execute_compiled_plan` when present, else interpret the tokens. Both produce
the same result value on the isolated clause stack, so the rest of `op_cond`
(guard truthiness, `IDLE` else, body result) is unchanged.

### Tail-call through a compiled body

The trampoline previously relied on `execute_section_core` recognizing the
guarded tail self-call. With the body compiled, that deferral moves into the
compiled executor: `execute_compiled_line`, when the tail op of a body run in
tail position (`in_tail_context`) is a call to `tail_self_word`
(`is_self_tail_call`), raises `tail_jump_pending` and skips the call — exactly
the interpreter's deferral. The residual single value flows out as the clause
result and the trampoline loops.

This reuses the existing `in_tail_context` gate, so the boundary is preserved:
a word's *own* body plan runs with `in_tail_context = false`, so a bare tail
self-call there (`{ REC }`) still recurses to the depth-limit error; only a
self-call inside a tail `COND` clause body trampolines.

## Why it is safe

- **Same value.** A compiled sub-plan of a token section is equivalent to
  interpreting that section (the property shadow validation already enforces for
  word bodies); the guard/body run on an isolated stack exactly as before.
- **Conservative.** A clause that does not fully compile keeps `None` and the
  interpreter path. Sub-plans share the enclosing word plan's epoch lifecycle,
  so dictionary changes invalidate them together — no separate validity check.
- **Boundary intact.** The `in_tail_context` gate keeps unguarded/own-body
  recursion native and depth-limited; `compiled_clause_tests` pins this.

Toggle with `AJISAI_NO_COMPILED_CLAUSE` or `set_compiled_clause_enabled`.

## Measured effect

`cargo run --release --example tail_call_bench` (countdown loop, depth 250,
tail-call + dispatch on):

```
-- Compiled clause body: interpreted vs compiled --
  clause OFF (interpreted): ~12.1 us/iter
  clause ON  (compiled):    ~ 9.8 us/iter
  speedup: ~1.24x
```

A **~1.24×** (~20%) per-iteration speedup on the recursion loop — the loop body
now runs compiled. This is the win the earlier steps set up: combined with
tail-call elimination (unbounded depth) and literal-vector compilation, the
guarded recursive loop is now compiled end to end except for the exact-rational
arithmetic itself.

## Tests

`rust/src/interpreter/compiled_clause_tests.rs`: the compiled path fires; a
guarded tail self-call inside a compiled body still trampolines past the depth
limit; ON vs OFF agree across clause shapes (single/multi-clause, numeric
guards, arithmetic bodies); the toggle disables it; unguarded recursion is
unaffected.
