# Fintech value-integrity design note (draft)

**Status:** Non-canonical design note. This document proposes two additions to
the Ajisai language and states them as ready-to-lift specification text, but it
is *not* itself normative. `SPECIFICATION.html` remains the single design
authority; nothing here changes Ajisai semantics until it is implemented in the
Rust core, given contract entries and tests, and then lifted into the
specification. Until that round-trip is done, adding these words to the
canonical Coreword tables would be a conformance violation (Section 7.14) and
would break the provenance/conformance CI.

## Why this note exists

The *Fintech Engineering Handbook* (w.pitula.me) distils money-handling systems
to three axioms:

- **No invented data** — money is never created from nowhere (idempotency,
  deduplication, reconciliation).
- **No lost data** — every movement is tracked at full precision (audit trails,
  immutability, event sourcing).
- **No trust** — external providers, internal components, and assumptions are
  all verified; broken assumptions fail loudly.

Ajisai's identity is *value integrity first*, so most of the handbook is already
Ajisai's core rather than a gap: exact continued-fraction reals already give
full precision (Section 4.2), arbitrary-precision coefficients already make
overflow a diagnosed failure rather than wraparound (Section 4.2.6), errors are
never values and propagate loudly with no error-swallowing modifier
(Section 11.4), and content-addressed source provenance already gives an
immutable audit surface (`docs/provenance/`).

Two principles are *not* yet expressed in the language, and both map cleanly
onto existing Ajisai machinery:

1. **Explicit quantization with a visible residual** — the handbook's "round
   only when forced, choose the mode explicitly, and never lose the remainder".
   Ajisai's `ROUND` today rounds to an integer only, with a single fixed tie
   rule, and discards the fractional part invisibly. This is the one place where
   the exact-real core silently loses information.
2. **Value conservation** — the handbook's double-entry invariant
   ($\sum \text{debit} = \sum \text{credit}$), generalised to "a transform that
   claims to preserve a total must preserve it exactly, or fail loudly". Ajisai
   already has a *flow-mass* conservation invariant for stack arity
   (Section 13.1); this adds the orthogonal *value-mass* invariant for numeric
   totals.

Both are proposed below as concrete specification text.

---

## Proposal ① — Explicit quantization and residual

### Placement

New section **§7.13 "Explicit quantization and residual"** (the 7.13 slot is
currently unused, between §7.12 NIL-passthrough words and §7.14 contract
metadata). Supporting edits to §4.2.6, §4.2.7, §7.3, §7.12, and §7.14.

### The gap

`FLOOR`, `CEIL`, and `ROUND` (Section 7.3) each map an exact real to the nearest
integer under a fixed rule. Three fintech-critical capabilities are missing:

- **Arbitrary grid.** Money is quantized to a currency's smallest unit
  (`1/100` for cents, `1/1` for JPY, `1/100000000` for a satoshi), not to `1`.
- **Explicit mode.** The correct tie rule is a *decision*, not a default. Money
  usually wants banker's rounding (nearest, ties to even) to avoid upward bias
  across many transactions; `ROUND` hard-codes ties-away-from-zero.
- **Residual visibility.** The discarded fractional part is exactly the amount
  that must be tracked so that a sum of quantized parts still reconciles to the
  original total. Today it vanishes.

### The word: `QUANTIZE`

Canonical name `QUANTIZE`, no sugar. Core word (same tier as `FLOOR`/`CEIL`/
`ROUND`).

**Stack effect (`TOP`, `EAT`):**

```
x step mode QUANTIZE  ->  q r
```

- `x` — the exact value to quantize (any scalar).
- `step` — a strictly positive rational quantum. The result `q` is an exact
  integer multiple of `step`.
- `mode` — a rounding-mode marker word (below).
- `q` — the quantized value: `q = n * step` for some integer `n`, chosen by
  `mode`.
- `r` — the **exact residual** `r = x - q`.

**Core invariant (the reason `r` exists):**

$$q + r = x \quad\text{exactly, for every well-formed call.}$$

Because `x`, `q`, and `r` are all exact, quantization loses *nothing*: the pair
`( q r )` reconstructs `x`. Discarding the residual is still possible, but it now
requires an explicit stack action (drop `r`), so silent loss is impossible —
this is exactly the handbook's "never lose the remainder" made structural.

### Rounding-mode markers

A closed, extensible family of pure marker words (values in the same sense as
`TRUE`/`FALSE`/`NIL`; they carry no behaviour on their own and are only
interpreted in the `mode` position of `QUANTIZE`). Let
$m = x / \mathit{step}$ be the exact quotient and $n$ the chosen integer, so
$q = n \cdot \mathit{step}$.

| Marker | Rule for choosing $n$ | Notes |
|--------|-----------------------|-------|
| `HALF-EVEN` | nearest integer to $m$; on a tie ($m$ exactly half-integral) choose the even $n$ | **Banker's rounding. Recommended default for currency.** |
| `HALF-AWAY` | nearest; ties choose the $n$ of larger magnitude | Generalises the current `ROUND` (`0.5 -> 1`, `-2.5 -> -3`) to any grid |
| `HALF-TO-ZERO` | nearest; ties choose the $n$ of smaller magnitude | |
| `TO-FLOOR` | largest $n$ with $q \le x$ (toward $-\infty$) | Grid generalisation of `FLOOR` |
| `TO-CEIL` | smallest $n$ with $q \ge x$ (toward $+\infty$) | Grid generalisation of `CEIL` |
| `TO-ZERO` | truncate toward $0$ | |
| `AWAY-ZERO` | away from $0$ | |

The marker set is closed: a `mode` operand that is not one of these markers is
malformed use and raises an error (not a Bubble). The prefixes (`HALF-`,
`TO-`, `AWAY-`) keep the names unambiguously rounding-flavoured and out of the
bare English namespace.

### Relationship to `FLOOR`/`CEIL`/`ROUND`

The integer-rounding words are the `step = 1` special cases, modulo residual:

- `x FLOOR`  ≡  `x 1 TO-FLOOR QUANTIZE` then drop `r`
- `x CEIL`   ≡  `x 1 TO-CEIL QUANTIZE` then drop `r`
- `x ROUND`  ≡  `x 1 HALF-AWAY QUANTIZE` then drop `r`

`QUANTIZE` therefore *unifies and generalises* the existing words and, unlike
them, surfaces the residual instead of discarding it. The existing words are
retained for convenience and backward compatibility.

### NIL, error, and decidability behaviour

- **`x` is NIL:** NIL-passthrough. Both outputs are NIL, and any attached reason
  is preserved. `QUANTIZE` joins the arithmetic NIL-passthrough list in §7.12
  for its `x` operand.
- **`step` is not a strictly positive rational** (zero, negative, NIL, or a
  non-rational scalar): malformed use → error, mirroring the deliberate
  asymmetry of `MOD` by zero (Section 7.3). `step` is `RejectsNil`.
- **`mode` is not a rounding-mode marker:** malformed use → error.
- **Decidability:** for `x` in the admitted domain $D$ (Section 4.2.7) and
  rational `step`, both `q` and `r` are exact rationals computable in finitely
  many steps, so `QUANTIZE` is total on $D$ and never yields `UNKNOWN`. For `x`
  outside $D$ (a lazy irrational) the nearest-multiple choice may consume the
  comparison budget exactly as the comparison words do; a tie that cannot be
  decided within budget yields the same principled outcome as elsewhere (the
  governing comparison is `unknown`), never a silent guess.

### Worked example — penny-perfect allocation

Split `100/1` three ways so the parts sum *exactly* back to the total, with the
rounding residual redistributed rather than lost:

```
100 3 /                 # exact share 100/3 (not representable in cents)
1 100 / HALF-EVEN QUANTIZE   # -> q = 3333/100, r = 1/300  (q + r = 100/3 exactly)
```

Quantizing each of the three shares to cents yields three `q` values plus three
residuals; the residuals sum to exactly `1/1` cent short, and the largest-
remainder rule assigns the extra cent to one share. Because every step is exact
and every residual is visible, the allocation reconciles to `100/1` with no
invented and no lost fraction of a cent — verifiable with Proposal ② below.

### Contract entries (§7.14)

| word | partiality | nil_policy | safety_level | mass |
|------|------------|------------|--------------|------|
| `QUANTIZE` | `Partial` (raises on bad `step`/`mode`) | `Passthrough` on `x`, `RejectsNil` on `step`/`mode` | `B` | `Fixed { consumes: 3, produces: 2 }` |
| mode markers (`HALF-EVEN` … `AWAY-ZERO`) | `Total` | `PreservesReason` | `A` | `Fixed { consumes: 0, produces: 1 }` |

### Supporting spec edits

- **§4.2.6 (numeric error policy):** add a bullet — *"Deliberate quantization.
  The only operation that maps an exact value onto a coarser grid, `QUANTIZE`,
  is explicit in both its step and its rounding mode and emits the exact residual
  alongside the quantized value; the quantized value plus the residual reconstruct
  the input exactly, so quantization is a visible, lossless-in-aggregate
  operation rather than a silent approximation."*
- **§4.2.7 (admitted domain):** add `QUANTIZE` (with rational `step`) to the
  words that keep $D$ closed and are total on $D$.
- **§7.3:** note that `FLOOR`/`CEIL`/`ROUND` are the `step = 1`, residual-
  discarding special cases of `QUANTIZE` (§7.13).
- **§7.12:** add `QUANTIZE` to the Arithmetic NIL-passthrough row (for its `x`
  operand).

---

## Proposal ② — Value conservation (generalised double-entry)

### Placement

New section **§13.3 "Value Conservation"** under the existing chapter 13
"Fractional-Dataflow Internal Invariants", plus a user-facing word `CONSERVE`
listed in §7.7 with a contract entry in §7.14.

### The idea

Double-entry bookkeeping is, at its core, a conservation law enforced
continuously: every movement debits and credits equal amounts, so the ledger's
net always sums to zero and money is neither invented nor lost. Ajisai already
formalises one conservation law — **flow-mass conservation** (Section 13.1),
which is about the *count* of values a word consumes and produces and is checked
statically. Value conservation is the orthogonal law about the *numeric total*
those values carry:

- **Flow-mass (Section 13.1):** how many values cross a boundary. Structural,
  static, arity.
- **Value-mass (Section 13.3, proposed):** what the values sum to. Numeric,
  checked, total-preserving.

They are independent: a split word `[ a b ] -> a1 a2 b1 b2` may conserve
flow-mass counts while silently violating value-mass if `a1 + a2 != a`.

### The word: `CONSERVE`

Canonical name `CONSERVE`, no sugar. A guard/verification word (listed in §7.7
Control and higher-order words).

**Stack effect (`TOP`, `EAT`):**

```
total parts CONSERVE  ->  parts
```

- `total` — an exact scalar: the amount the parts must account for.
- `parts` — a vector of scalars whose exact sum must equal `total`.
- On success (exact equality), `parts` is passed through **unchanged**, so
  `CONSERVE` composes transparently inside a pipeline (it is an identity on
  its data payload, a tripwire on its assumption).
- On violation, `CONSERVE` raises a **channel error** (fail loudly), not a
  Bubble. A broken conservation assumption is precisely the "fail loudly on
  broken assumptions" principle; degrading it to a `NIL` that could flow
  downstream would defeat the guard.

**Why raise, and why not `UNKNOWN`.** `CONSERVE` is a control guard, not an
observation. A guard that cannot *confirm* its safe condition must not let flow
pass. Over the admitted domain $D$ (Section 4.2.7) sum-equality is exact and
total, so for rational money values `CONSERVE` is fully decidable and either
passes or raises. If the operands leave $D$ and equality is genuinely
undecidable within the comparison budget, `CONSERVE` still raises (it never
silently passes on an undecided equality); only a *proven* equality passes.
This strictness is deliberate and is the difference between `CONSERVE` (a
safety assertion) and `EQ` (an observation that may return `unknown`).

**NIL policy.** If `total` or any element of `parts` is NIL, conservation cannot
be certified (an absent amount is exactly "lost data"), so `CONSERVE` raises.
It is `RejectsNil`.

### Worked example — completing the allocation

```
100                             # total
[ ... three cent-quantized shares with the residual cent redistributed ... ]
CONSERVE                        # passes iff the three shares sum to exactly 100/1
```

If a refactor of the split logic ever makes the parts sum to `9999/100` instead
of `100/1`, `CONSERVE` halts evaluation loudly at the exact point the invariant
broke, instead of letting a one-cent discrepancy flow downstream.

### Relationship to `QUANTIZE`

`QUANTIZE`'s residual identity $q + r = x$ is the *atomic* conservation
guarantee; `CONSERVE` is the *aggregate* one over a whole vector of parts.
Together they let a program both round to a currency grid and prove that the
rounding invented and lost nothing — the two halves of "penny-perfect".

### Contract entry (§7.14)

| word | partiality | nil_policy | safety_level | mass |
|------|------------|------------|--------------|------|
| `CONSERVE` | `Partial` (raises on violation / undecidable / NIL) | `RejectsNil` | `B` | `Fixed { consumes: 2, produces: 1 }` |

### Supporting spec edits

- **§13 intro / §13.1:** cross-reference the two conservation notions and state
  that they are orthogonal (count vs sum).
- **§7.7:** add the `CONSERVE` row.
- **§11.1 (error categories):** `CONSERVE`'s violation maps to a `custom` error
  with a stable message (e.g. `"Conservation violated"`); the exact category
  string is fixed when the word is implemented.

---

## Scope, sequencing, and what is intentionally deferred

This note covers principles ① and ② only. Principles ③ (idempotency /
at-most-once as an outward-gate contract) and ④ (independent-path
reconciliation / "no trust") are natural follow-ups at the Gates/contract layer
(Appendix A, Section 7.14) and are out of scope here.

Recommended implementation order once this draft is accepted:

1. Implement `QUANTIZE` + the mode markers in the Rust core, with contract
   entries and per-Coreword tests (Section 15.1).
2. Implement `CONSERVE`, with contract entry and tests.
3. Add NIL-reason and MC/DC coverage (Sections 15.2, 15.3) for the new
   partial/branching behaviour.
4. Lift the section text above into `SPECIFICATION.html`, regenerate the source
   attestation (`npm run provenance:attest`), and update the Reference
   (`public/docs/`) with runnable examples.

Only after step 4 do these words become canonical; until then this note is the
single place the proposal lives.
