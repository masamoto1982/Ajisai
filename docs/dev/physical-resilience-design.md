# Physical Resilience — Semantic Integrity (design note, non-canonical)

> Canonical semantics live in `SPECIFICATION.html`. This note describes an
> internal robustness mechanism and makes no language-level guarantees beyond
> what it states here.

## Scope and honest claims

Ajisai does not claim to prevent hardware faults or physical attacks (thermal
runaway, power instability, memory corruption, cold-boot extraction) by language
machinery alone. What it does claim is narrower and enforceable:

> Ajisai does not silently commit a result that has become semantically
> unverifiable. When an optimized execution path and the reference path
> disagree on the observable result, the optimized result is not adopted.

A user never configures this. Programs receive the protection simply by running.

## What ships in this unit

The runtime already runs a *shadow validation*: for the first execution of a
word (per epoch, for bounded input sizes) it runs both the compiled (optimized)
path and the plain (reference) path and compares them
(`rust/src/interpreter/shadow_validation.rs`). This unit closes two gaps in that
comparison and removes one unsafe adoption.

### 1. The comparison was too shallow

Previously the paths were considered to agree iff `fast_stack == plain_stack`.
`Value`'s `PartialEq` compares `data` and `hint` but **deliberately ignores
`absence`** (so language-level `EQ` is unaffected — `types/mod.rs`). It also did
not compare emitted **host effects**, even though both effect列 were already
captured. Consequences:

- Two stacks that print identically but carry different absence reasons
  (e.g. one `NIL` from division-by-zero, another from index-out-of-bounds) were
  treated as equal — a silently-different *meaning*.
- A path that emitted a different (or extra, or missing) `HostEffect` while
  leaving the stack unchanged passed validation — a silently-different
  *observable behavior*.

The comparison is now: stacks agree on `data` + `hint` + the **semantic core of
absence** (`reason`, `origin`, `recoverability` — the human/AI `diagnosis` prose
is intentionally excluded as brittle), **and** the two host-effect sequences are
identical.

### 2. A success the reference path never produced was being committed

When the compiled path returned `Ok` but the reference path returned `Err`, the
old code silently adopted the compiled `Ok`. That is precisely the
"broken-meaning returned as a valid value" case. It is now treated as a
divergence.

### Reaction policy — `IntegrityMode`

`ValidationPolicy.integrity_mode` (internal, default `Fallback`):

| Mode       | On disagreement                                              |
|------------|-------------------------------------------------------------|
| `Off`      | Legacy shallow check; adopt compiled. Benchmark-only.       |
| `Observe`  | Full comparison, count the mismatch, still adopt compiled.  |
| `Fallback` | **Default.** Prefer the reference path (its result wins).   |
| `Strict`   | Refuse the result; surface an integrity failure.            |

Every genuine divergence increments
`RuntimeMetrics.shadow_validation_integrity_mismatch_count`, independent of how
the mode then resolves it, so the signal is observable even in `Observe`.

`Strict` currently surfaces a self-describing recoverable error. A dedicated
`ErrorCategory::IntegrityFailure` / `NilReason::IntegrityFailure` /
`AbsenceOrigin::IntegrityCheck` taxonomy is a planned follow-up so the failure
flows through the standard diagnosis path rather than a `Custom` string.

## Deliberately out of scope (this unit)

- A separate `SemanticFingerprint` hash type. Not needed yet: the enriched
  direct comparison already covers stack value, hint, absence core, and host
  effects. A cheap (non-`String`) fingerprint is the natural next step when
  epoch / tensor-mask coverage is added.
- Secrecy labels, persistence redaction, and host environmental signals. These
  are a separate concern with no current producer in the language and are not
  built here.

## Verification

`rust/src/interpreter/shadow_validation.rs` (module `integrity_comparison_tests`)
covers: identical paths agree; differing / missing host effects are divergences;
two `NIL`s that are `Value`-equal but carry different absence reasons are caught;
matching absence reasons agree; absence-core comparison ignores diagnosis text.
The full library suite passes with `Fallback` as the default.
