# Reverse-dependency index reads (`collect_dependents`)

Status: **implemented**. This is a value-model-neutral runtime change: it does
not alter any observable Ajisai result, only the cost of one internal query.
The canonical language definition is `SPECIFICATION.html`.

## Motivation

When Ajisai runs, it is continuously *searching* its dictionaries — resolving a
name to a definition, and (at edit time) asking "who depends on this word?".
The second query, `collect_dependents`, backed every `DEF` and `DEL`:

- `DEF` of an existing word asks for its dependents to decide whether to block
  the redefinition (or warn) and to update reverse edges
  (`execute_def.rs`).
- `DEL` asks the same to decide whether to block the deletion
  (`execute_del.rs`).

The previous implementation answered it by **rescanning every word in every
user dictionary** and testing `def.dependencies.contains(word_name)` — an O(N)
walk over the whole corpus per call, where N is the total number of user words.

This is the inverted-index lesson from full-text search engines, applied to the
dictionary: the runtime *already maintains* the inverted index this query wants.
`Interpreter::dependents: HashMap<word → {dependent words}>` is a
term→posting-list structure, kept in sync incrementally by `DEF`/`DEL` and
rebuilt wholesale by `rebuild_dependencies`. The scan recomputed, from the
forward edges, a reverse mapping that was already sitting in memory.

## Change

`collect_dependents(word_name)` now reads the maintained index directly:

```rust
self.dependents.get(word_name).cloned().unwrap_or_default()
```

turning an O(N)-corpus walk into an O(1) map probe (plus the clone of the
result set). A new `collect_transitive_dependents(word_name)` returns the
breadth-first closure over the same index — the full impact set of a
redefinition or deletion — which a later cache-invalidation stage will use as
its scope.

The change is confined to read paths. The index is still *written* exactly as
before, in `rebuild_dependencies`, `op_def_inner`, and `op_del`; nothing about
when or how reverse edges are maintained changed.

## Why it is safe

The maintained index and the full scan are two computations of the same set, so
correctness reduces to: *does the incrementally-maintained `dependents` ever
drift from a from-scratch scan?* Two mechanisms pin this:

1. **In-call cross-check.** `collect_dependents` carries a `debug_assert_eq!`
   comparing the index read against `collect_dependents_by_scan` (the retained
   full scan) on every call. Debug assertions are compiled out of release
   builds — the hot path is the bare index probe — but are active in every test
   build. Because the entire test suite (1400+ tests) exercises `DEF`/`DEL`
   through `collect_dependents`, every one of those calls now also asserts that
   the index equals ground truth. Any drift, in any scenario, fails a test.

2. **Targeted tests.** `dependents_index_tests.rs` pins the returned *values*
   across direct dependence, multiple dependents, transitive chains, redefinition
   that drops a stale edge, deletion that clears an edge, and the empty/unknown
   case.

Observable semantics are unchanged: the same dependent sets drive the same
redefinition/deletion warnings and blocks, so the Section 8.6 behaviour and the
semantic firewall are untouched (`npm run check:semantic-firewall` passes).

## Scope and non-goals

`collect_dependents` runs at **edit time** (`DEF`/`DEL`), not on the program
execution hot path, so this does not move the `tail_call_bench` per-iteration
number — that axis belongs to the value-model fast paths (`scalar-fastpath-d1.md`
and the arithmetic-kernel `i128` work). This change improves the
define/edit-cycle cost on large dictionaries and removes a redundant
recomputation of state the runtime already holds.

`collect_transitive_dependents` is introduced here but not yet wired into any
invalidation path; it is the foundation for dependency-scoped cache
invalidation (a separate, later step).
