# Ajisai — Python port (spec-only reproduction)

A from-scratch Python implementation of Ajisai, written **only** from
`SPECIFICATION.html` (the canonical authority, Section 2.1) without consulting
the Rust/WASM/TypeScript implementation or any prior port.

The purpose is the experiment described in the project task: re-deriving Ajisai
from the specification surfaces every place where the spec is under-determined.
Those findings — the deliverable that refines the spec — are in
[`SPEC_GAPS.md`](SPEC_GAPS.md).

## Run it

```sh
cd python
python -m ajisai            # REPL
python -m ajisai prog.aji   # run a file
echo '1 2 ADD' | python -m ajisai -
python tests/test_spec_examples.py   # spec-example conformance checks
```

## What is implemented

- **Exact-real scalars** (Section 4.2) as continued fractions, backed by an
  `AlgebraicReal` (Q-combinations of square-free surds): rationals plus the
  square roots `MATH@SQRT` produces. Arithmetic is exact; equality and ordering
  are exact and total over this domain (Section 2.3.1). See `ajisai/numbers.py`.
- **Value model** (Section 4): Scalar, Boolean (distinct from numbers), Vector,
  Text (codepoint vector with the Text role), Record, NIL with structured
  absence metadata, CodeBlock, process/supervisor handles.
- **Stack + modifiers** (Sections 5, 6): `TOP`/`STAK` × `EAT`/`KEEP` with the
  count-fold and chained-comparison semantics of Section 6.1, plus the `.`/`..`/
  `,`/`,,`/`;`/`;;` sugar and combined forms.
- **Words** (Section 7): arithmetic, the six comparisons, `COMPARE-WITHIN` with
  an explicit NICF budget yielding `UNKNOWN`, K3 logic, vector/tensor ops,
  string/conversion words, higher-order words, `COND`, user dictionary
  (`DEF`/`DEL`/`LOOKUP`), `PRINT`, modules (`MATH`, `ALGO`, `TIME`, `CRYPTO`,
  `IO`, and stubs for `SERIAL`/`JSON`/`MUSIC`), and a synchronous child runtime.
- **NIL / Bubble Rule** (Section 11.2): well-formed-but-no-value → reasoned NIL;
  malformed use → raised error; passthrough preserves the leftmost reason.
- **Roles + rendering** (Section 12): RawNumber, ContinuedFraction (nested form),
  Text, TruthValue, NIL, with distinct Stack vs. Output surfaces.

## Known divergences

Everything the port had to guess is marked with a `SPEC-GAP` comment in the
source and explained in `SPEC_GAPS.md`. The highest-impact ones are COND's clause
collection and stack discipline, the higher-order word signatures, VENT's
non-NIL branch, and the exact-vs-budgeted reading of the six comparison relations.
