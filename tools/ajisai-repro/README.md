# ajisai-repro — maintained Core reference interpreter (executable shadow of the spec)

`ajisai.py` is a small Python interpreter for **Ajisai Core**, written from
`SPECIFICATION.html` prose alone (without consulting the Rust sources). It is
maintained as the **executable shadow of the specification**: where the prose
defines the language, this interpreter is meant to *run* it, so a reader can see
the spec behave.

This directory began as a throwaway divergence probe. It is now positioned as a
maintained verification artifact, per
`docs/dev/wasm-style-reference-interpreter-design.md`
(the WebAssembly five-asset arrangement: SpecTec ≡ `ajisai-authoring-style.md`,
and the one missing asset was a reference interpreter).

## Purpose

- **An executable shadow of the spec.** Its goal is the *opposite* of the
  production Rust engine: production optimizes for speed, this optimizes for an
  obvious, section-by-section correspondence to `SPECIFICATION.html`. It carries
  no optimizations.
- **A differential-test oracle.** Run against the production CLI over a corpus,
  any disagreement marks a candidate **class-A spec hole** (a region the prose
  leaves undecided) or **class-B implementation bug** — the same method that
  originally surfaced 15 of 79 divergences (see `DIVERGENCE-ANALYSIS.md`).
- **An executable test of the authoring discipline.** Because it was written
  from prose alone, a reproduction split is itself a measurement of where the
  prose is ambiguous (see the design memo §3).

## Scope: Ajisai Core only

This interpreter covers **Ajisai Core** (host-independent) only. Hosted effects
(IO, SERIAL, clock, secure random, …) require host capabilities and are **out of
scope**; they are covered by the conformance suite and the production
implementations. This matches the Core/Hosted split in `PORTABILITY.md`.

## Position in the authority order: a verification artifact, not an authority

The canonical source is `SPECIFICATION.html` **only**.

- The reference interpreter is a **verification artifact**, sitting in
  `SPECIFICATION.html` §2.5 at the same rank as the conformance suite and law
  tests — **below** the prose spec and the mathematical formalization.
- **If it disagrees with the spec, the spec wins and the interpreter is
  fixed.** It is **not a second design authority** (this is exactly the
  constraint of §16.1 / Conformance Checklist item 1).
- When the production engine and this interpreter disagree and the conformance
  suite is silent, the direction of the fix is decided by the
  **suite-arbitration rule** of `docs/dev/spec-impl-drift-tactic.md` §3.3 — by
  the suite, never by a clock and never by this interpreter.

> Follow-up (separate PR, requires the spec author's approval): annotate the
> reference interpreter as a verification artifact in `SPECIFICATION.html`
> §2.4/§2.5 and in `PORTABILITY.md` principle 2. Those touch canonical text and
> are intentionally **not** part of this change.

## Files

- `ajisai.py`  — the Core reference interpreter
  (run a program: `python3 ajisai.py "1 2 ADD"`).
- `probe.py`   — runs the production Rust CLI and extracts a compact result.
- `compare.py` — the differential-test driver (production CLI ⇔ reference).
- `compare-output.txt` — last recorded comparison run.
- `FINDINGS.md`, `DIVERGENCE-ANALYSIS.md` — analysis of discovered divergences.

## Differential testing

Build the production CLI, then run the driver:

```sh
# build the production headless CLI first
( cd ../../rust && cargo build --bin ajisai --release )

# inline Core corpus (default)
python3 compare.py

# also include the Core cases extracted from tests/conformance/
python3 compare.py --conformance
```

- The CLI path is resolved from `AJISAI_BIN`, defaulting to the repo-relative
  `rust/target/release/ajisai`.
- **The driver exits non-zero when any divergence is observed** (0 otherwise),
  so CI can gate on it.
- Observation is normalized to `(status, stack, output)` using the canonical
  Display (§12); comparison is by **value identity**. A divergence is
  **recorded, not fixed**, here — its direction is adjudicated separately per
  the suite-arbitration rule above.
