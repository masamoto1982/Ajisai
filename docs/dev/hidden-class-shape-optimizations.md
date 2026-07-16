# Hidden-class-style shape optimizations

Status: prototype (non-canonical design note). No surface syntax or
value-semantics change; the canonical definition remains `SPECIFICATION.html`.
End users need no knowledge of, and have no interaction with, any mechanism
described here — everything is default-on, self-managing, and invisible except
as speed.

## Motivation

JavaScript engines attach a shared, interned *hidden class* (shape) to objects
so that (a) same-layout objects share one layout description instead of each
carrying its own, and (b) call sites can cache "what shape did I see here?"
and jump straight to a specialized route guarded by one cheap comparison.

Ajisai is not property-access-bound the way JavaScript is, so the technique
does not port literally. But both halves of the idea have exact Ajisai
analogues, and both compose with the existing epoch-guard discipline:

* the dictionary already has a "code-side hidden class" — `dictionary_epoch` /
  `module_epoch` guard every compiled plan and the resolve cache;
* this note adds the *data side*: interned Record layouts, and a per-call-site
  shape cache for compiled builtin calls.

Three mechanisms, all invisible to users:

1. **Record layout interning** (`rust/src/types/record_shape.rs`)
2. **Compile-time builtin call specialization** (`CompiledCall` in
   `rust/src/interpreter/compiled_plan.rs`)
3. **Call-site shape inline cache** (`rust/src/interpreter/shape_ic.rs`)

## 1. Record layout interning

`ValueData::Record` previously carried a per-instance
`index: HashMap<String, usize>`. Every record clone copied the map (every key
`String` included), and record equality compared two maps.

The key→slot mapping is layout, not data. It now lives in an interned
`Arc<RecordShape>` shared by every same-layout record:

```
Record { pairs: Arc<Vec<Value>>, shape: Arc<RecordShape> }
```

* The intern table is global, mutex-guarded, and bucketed by an
  order-independent layout hash, so the lookup path materializes no sorted
  canonical key and clones no strings. Bucket collisions resolve by comparing
  the mappings.
* The table is bounded (`INTERN_CAP`): past the cap, new layouts get private
  (unshared) shapes that behave identically. A JSON stream of
  all-unique-layout objects therefore cannot grow the table without bound.
* Because Ajisai values are immutable, a shape never changes after
  construction. The infamous hidden-class *transition chains* (layout
  mutation on property add) do not exist here at all: one intern at build
  time, nothing afterwards.
* Equality semantics are unchanged: `Arc::ptr_eq` is only a shortcut, with
  the old mapping comparison as the fallback across the interned/uninterned
  boundary.

Measured (release, `record_shape_bench`): cloning a 32-field record Value
dropped from ~1509 ns to ~41 ns per clone (~36x), since the layout copy became
a pointer bump. The mixed JSON parse/SET/GET/EQ workload is neutral (within
noise): what interning costs at construction it earns back on clone/equality.

## 2. Compile-time builtin call specialization

`CompiledOp::CallBuiltin` used to carry the surface name and re-do, on every
execution: alias canonicalization (linear scan of `CORE_WORD_ALIASES`), a
linear scan of all 99 `BUILTIN_SPECS` entries, the force-flag classification,
and — in `post_call_cleanup` — an `is_mode_preserving_word` check that
allocated `name.to_uppercase()` per call.

All of that depends only on static tables, never on dictionary state, so
`CompiledCall::resolve` now computes it once at plan-compile time:

```
CallBuiltin(Arc<CompiledCall {
    name,               // canonical
    key,                // pre-resolved executor
    resets_force_flag,  // canonical != DEF/DEL/FORC
    mode_preserving,    // precomputed cleanup decision
    ic_op, shape_ic,    // see §3
}>)
```

No epoch guard is needed precisely because nothing here reads mutable state —
this is the same argument that lets the resolve cache validate by epoch, taken
to the degenerate case of "depends on nothing mutable".

On the guarded countdown loop (`tail_call_bench`'s idiom) this is
performance-neutral in isolation: at ~7.5 µs/iteration the loop is dominated
by clause-stack and value work, not dispatch scans. The change removes real
per-call work (two linear string scans and one `String` allocation per
compiled builtin call) and is kept for that reason, with honest expectations.

## 3. Call-site shape inline cache

Each `CompiledCall` for a word with a D1 scalar fast path (`+ - * / < <= > >=
= !=`) owns a `ShapeIc` — one `AtomicU8` with three states:

* `UNSEEN` → probe the scalar fast path first;
* `SCALAR` → every execution so far completed on the scalar fast path; keep
  probing it first (monomorphic site);
* `GENERIC` → the site has seen non-fast-path operands; skip the probe and go
  straight to the generic executor.

The cache is *routing state only*. Every route revalidates its operands
(`scalar_fast_operand` re-checks shapes before producing anything), so a stale
or racing entry can only pick which equivalent route runs, never change a
value. This is also why sharing one `ShapeIc` across plan clones and hedged
races is safe: `AtomicU8`, any value valid.

Skipping the generic route's NIL-passthrough pre-check on a completed probe is
sound because the scalar fast path only accepts bare scalars and singleton
numeric wrappers — values that can never be operational NIL. On any rejection
the generic executor runs from its very beginning, NIL check included, and the
site demotes to `GENERIC`.

Observability: `RuntimeMetrics::shape_ic_hit_count` / `shape_ic_miss_count`.
On the countdown loop the IC engages fully (2 hits/iteration, 0 misses) and
timing is neutral — the probe it skips was already cheap. Its value is
structural: sites that are *not* scalar-shaped stop paying the probe at all
(mixed/tensor sites demote once), and the state machine is the natural place
for future monomorphic classes (e.g. cached singleton-wrap results) without
touching dispatch again.

## Switches and A/B harnesses

Following the D1/HOF-memo convention, everything is default-on with paired
kill switches for measurement only (never needed for correctness):

| Mechanism | Env switch | In-process setter |
| --- | --- | --- |
| Shape IC | `AJISAI_NO_SHAPE_IC` | `set_shape_ic_enabled(bool)` |
| (scalar fast path, reused) | `AJISAI_NO_SCALAR_FASTPATH` | `set_scalar_fastpath_enabled(bool)` |

Record interning and call specialization have no switch: they are
representation changes with no alternate route to A/B in-process (compare
across revisions instead, e.g. with `record_shape_bench`).

Benches: `cargo run --release --example shape_ic_bench` (loop + IC counters),
`cargo run --release --example record_shape_bench` (record construction /
clone). Differential tests: `rust/src/interpreter/shape_ic_tests.rs` asserts
stack values, rendered forms, and hints are identical with the IC on and off
across hits, misses, NIL demotion, KEEP mode, division-by-zero, and the
recursive-loop idiom; `rust/src/types/record_shape.rs` carries the interning
unit tests.

## Why it is safe

* **Same value.** The IC only reorders which equivalent route computes a
  result; the fast paths themselves are the untouched D1 implementations, and
  shadow validation continues to compare compiled against plain execution.
* **Same layout semantics.** `RecordShape` preserves the exact key→slot
  mapping and the exact equality relation; interning changes identity of the
  metadata allocation, which was never observable.
* **Bounded state.** The intern table is capped; the IC is one byte per
  compiled builtin call site.
