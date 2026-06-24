# Internal GOTO: precompiled COND clause dispatch

Status: prototype (non-canonical design note). Builds on
`internal-goto-tail-call.md`. No surface syntax or value-semantics change; the
canonical definition remains `SPECIFICATION.html`.

## Motivation

`COND` is the loop condition in Ajisai's recursion idiom, so it runs on every
iteration. The dynamic implementation rebuilds its dispatch structure from the
stack each call (`control_cond.rs`):

1. pop every consecutive code block off the stack (**deep-cloning** each block's
   token vector),
2. classify clause style (`{guard}{body}` pairs vs `{guard | body}`),
3. **re-split** each block on `|` into a guard and a body.

The clause blocks are compile-time literals (`PushCodeBlock` ops emitted right
before the `COND`), so this is the same work every iteration. This is the second
internal-GOTO opportunity from the original analysis: a **jump table** — select
a clause by dispatching on a structure computed once, not rebuilt per call.

## Mechanism

A new compiled op carries the split-once clause table:

```
CompiledOp::CondDispatch(Arc<[CondClause]>)   // CondClause { guard: Arc<[Token]>, body: Arc<[Token]> }
```

At compile time (`lower_cond_dispatch` in `compiled_plan.rs`), each
`CallBuiltin("COND")` whose preceding contiguous `PushCodeBlock` run is
statically known is replaced by a `CondDispatch` carrying the precomputed,
`Arc`-shared clauses. The split reuses the exact dynamic splitter
(`split_clause_blocks`), so a malformed clause set fails to split and is simply
left as the dynamic `COND` (its error still surfaces at runtime).

The `PushCodeBlock` ops are **kept**. They still push the clause blocks each
call, which:

- preserves stack discipline (the blocks are consumed, as before), and
- gives `op_cond_dispatch` a cheap, exact safety check: it collects the top
  code blocks (now **moving** their token vectors out — no clone) and, when
  their count matches the precomputed table, dispatches on the precomputed
  clauses; otherwise (an unexpected extra block reached the stack) it falls back
  to the dynamic split of the actual blocks. Either way the result is identical
  to dynamic `COND`.

Both paths converge on one shared core (`run_cond_core`) that pops the target,
evaluates guards, and runs the winning clause — so dynamic and compiled `COND`
are behaviorally identical, and shadow validation (`shadow_validation.rs`)
compares them on every non-recursive call.

### Interaction with the tail-call trampoline

The tail-call work keyed the backward jump on `CallBuiltin("COND")` being the
tail op. `CondDispatch` is now recognized as a COND tail op too
(`is_cond_tail_op`), so a guarded tail self-call inside a precompiled clause
body is still eliminated. Tail context is set immediately before the op and
consumed on entry, so no restore is needed.

## Invariants preserved

- **Semantics.** Guard evaluation, body execution, `IDLE` else-clauses, Strong
  Kleene `UNKNOWN` guards, numeric-guard fallback, and hedged prefetch all reuse
  the existing functions; only the clause *source* changes (precomputed vs
  re-collected).
- **Errors.** Malformed clause sets are not lowered, so their errors are
  unchanged. Count mismatch at runtime falls back to the dynamic split.
- **Mass / quantization analysis.** `CondDispatch` is treated as data-dependent
  arity (like `COND`); clause purity is still observed through the retained
  `PushCodeBlock` ops.
- **Identity / provenance.** The dispatch is a property of the derived plan, not
  the source.

Toggle with `AJISAI_NO_COND_DISPATCH` or `Interpreter::set_cond_dispatch_enabled`.

## Measured effect

`cargo run --release --example tail_call_bench` (countdown loop, depth 250,
tail-call on):

```
-- COND dispatch: dynamic collect vs precompiled jump table --
  dispatch OFF (dynamic): ~10.2 us/iter
  dispatch ON  (jump tbl): ~ 9.9 us/iter
  speedup: ~1.03x
```

The gain is modest *for this loop* because the dominant per-iteration cost is
interpreting the guard and body token streams (vector construction, exact
arithmetic), not the clause collection the dispatch removes. The dispatch helps
proportionally more where `COND` itself dominates — many clauses and/or simple
guards — and the move-not-clone collection speeds the dynamic path for *every*
`COND` call, not only loops.

The deeper win (running the guard and body **compiled** rather than interpreted)
requires compiling literal vectors and the clause sub-streams; precomputing the
clause table here is the structural prerequisite for that next step.

## Tests

`rust/src/interpreter/cond_dispatch_tests.rs`: the fast path fires, ON vs OFF
agree across clause shapes (pairs, `|`, `IDLE`, multiple clauses, numeric
guards), malformed clauses keep their error, and the toggle disables it. The 22
existing `control_cond_tests` already exercise COND semantics with dispatch on
by default.
