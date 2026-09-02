---
name: spec-impl-alignment
description: Run Ajisai's 4-phase spec/implementation alignment pass — check spec/ for internal contradictions, check the implementation for internal contradictions, reconcile any remaining spec-vs-implementation disagreement toward whichever side has the better synergy, then delete whatever turns out to be spec with no implementation or implementation with no spec (including the docs/dev/ history that led there). Use when asked to audit, reconcile, or clean up drift between Ajisai's specification and its implementation, or to continue/repeat that effort.
---

# Ajisai spec/implementation alignment

Full method: `docs/dev/spec-impl-alignment-methodology.md` (non-canonical,
but the citable rationale for every step below — read it before a first
run). This file is the operational checklist; that one is the reasoning.

Canonical spec lives in `spec/` (5 sources: `language-semantics.md`,
`words.json`+`words.schema.json`, `semantic-families.json`,
`gui-semantics.md`, `host-protocol.schema.json`) and is regenerated into
`SPECIFICATION.html` via `npm run specification:generate`. Nothing else
defines Ajisai semantics — not `docs/dev/`, not `CLAUDE.md`.

Work one phase to a clean, tested, committed state before starting the
next. Each phase's fixes get their own PR (small, verifiable diffs beat
one large one — see PRs #1604-#1608 for the shape this takes in practice).

## Phase 1 — spec-internal consistency

Read the 5 `spec/` sources against **each other**, without looking at
`rust/` or `src/`. Existing gates (`npm run specification:check`,
`semantic-kernel:check`, `word-schema:check`, `word:manifest:check`,
`word-registry:check`, `word:reference:check`, `core-word-docs:check`,
`check:formalization-coverage`, `check:minimal-core`,
`check:unreachable-contract`) verify generation round-trips and structural
cross-references, but not whether one source's *prose* claims something a
sibling source's *schema* doesn't actually define. That gap is what a human
(or an agent) reading pass finds.

When two sources disagree, resolve toward whichever is **more clearly the
data of record** for that fact — narrow prose to match a schema's actual
declared shape, rather than widening the schema to match aspirational
prose (widening a schema is how host-protocol-v2 happened: see Phase 4).

Verify: regenerate (`npm run specification:generate`), then run every
`npm run *:check` script above plus `cargo test --all-targets` — a
spec-only change can still break a Rust test that reads `spec/` at
compile/test time (e.g. `include_str!`-based conformance or golden tests).

## Phase 2 — implementation-internal consistency

Ignore `spec/` entirely. Look for the same fact implemented independently
in more than one place — `rust/src`, `src/`, `tools/mcp-server/` — where
nothing actually cross-checks the two copies against each other. Grep for
tells: a comment claiming "mirrors X" or "same as Y" where X/Y is a
different file; a claimed shared test file that doesn't exist; one copy
whose `git log` is stale relative to the other's.

For each real finding: confirm which copy is actually live (called from
production code, not just from its own tests) by tracing callers, not by
reading doc comments — comments claim parity more often than code
delivers it. Fix the bug in the live copy if the two disagree on behavior;
delete the dead copy if one turns out to be unreferenced (grep the whole
repo for its filename/export before deleting — `src/gui/value-formatter.ts`
was dead and buggy; word-candidates.js was live and merely unguarded).
Add the missing cross-check test so the same drift can't ship silently
again.

Verify: `npm run check` (tsc), `npm run lint`, `npm run deadcode`, full
`npm test` (vitest), `cargo fmt --check`, `cargo clippy --all-targets --
-D warnings`, `cargo test --all-targets`.

## Phase 3 — spec vs. implementation

Only now compare the two. For each observed disagreement between what
`spec/` says and what a real build actually does (confirm with a live
program run — `ajisai agent compute - --json`, or the WASM/GUI
equivalent — never trust a doc comment's claim about behavior),
apply the suite-arbitration rule:

- The behavior is **pinned by `tests/conformance/index.html`**
  (`LANG.CONFORMANCE.CORPUS`) → intentional design. Amend `spec/` to
  document it (spec catches up to impl).
- **The suite is silent** on it, and it demonstrably violates spec
  prose (checked against `LANG.CONFORMANCE.FAMILIES` law tests or
  `docs/dev/ajisai-mathematical-formalization.md`'s formal oracle) →
  implementation bug candidate. Fix the implementation to match spec
  (impl catches up to spec), and add a conformance case pinning the
  corrected behavior.
- **`spec/` disagrees with itself** on the point → that's actually a
  Phase 1 finding surfacing late; resolve by authority order
  (`LANG.AUTHORITY.SOURCES`) and `git blame` provenance, with a human
  decision where genuinely ambiguous.

Never resolve a Phase 3 finding by expanding a schema to describe
something no consumer reads yet (see host-protocol-v2's postmortem in
`docs/dev/spec-impl-alignment-methodology.md` §2) — verify liveness with
a real build first (`grep -rl` for the module's own name/type across the
whole repo; a serializer nothing calls is not "the protocol" no matter
how complete its schema is).

## Phase 4 — delete what turns out to be one-sided

Once Phase 3's reconciliation lands and every remaining gate is green:

1. **Spec with no implementation** (a schema, a declared field, a
   documented behavior nothing in `rust/`, `src/`, or `tools/mcp-server/`
   ever produces or reads) — delete the spec text/schema/fixture. Do not
   leave it as a "future target": an unimplemented spec is noise, and if
   the feature is wanted later it gets written from the implementation
   that actually exists then.
2. **Implementation with no spec** (real, reachable code implementing a
   behavior no `spec/` source declares, and Phase 3 confirmed it's not
   simply an undocumented-but-intended feature) — delete the code, its
   tests, and its fixtures.
3. **The `docs/dev/` history that led to the now-resolved finding** —
   once Phase 1-3 have folded a memo's conclusions into the canonical
   spec or the implementation, the memo describing the investigation is
   noise for anyone opening `docs/dev/` next. Before deleting a memo:
   - `grep -rl '<filename>' .` across the whole repo (including other
     `docs/dev/` files and `tests/conformance/index.html` — they cite
     each other and the corpus more than you'd expect).
   - For every hit, either rewrite the citing passage to be
     self-contained (point at a stable `LANG.*` clause ID or the current
     file that actually carries the fact now, not the retiring memo) or
     remove the row from `docs/dev/INDEX.md` alongside the file.
   - Re-run `npm run check:docs-dev-drift` after — it only catches a
     `[設計根拠]`/`[方針記録]` memo's stale Rust-identifier claims, not a
     dangling filename citation, so the grep sweep above is not optional
     even when this check stays green.
   - A memo tagged `[執筆規約]` (writing convention) or one whose
     conclusions are still actively used (referenced by a CI script, or
     cited from a live Rust/TS source comment as design rationale) is
     *not* history — most of `docs/dev/` turns out to be this even
     though it looks like a one-off report at a glance. Check references
     before assuming a memo is safe to delete.

Verify the full matrix again after deleting: every `npm run *:check`
script, `cargo fmt`/`clippy`/`test --all-targets`, `npm run check`/`lint`,
full `npm test`.
