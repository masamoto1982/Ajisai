# Ajisai

**Ajisai is a concatenative language in which exact values and vectors flow left
to right, and are observed, branched, and held at the points the program names.**

```
1 3 DIV 3 MUL              # 1 — exactly, not 0.9999999999999998
0.1 0.2 ADD 0.3 EQ         # TRUE
1 2 3 4 STAK ADD           # 10  — the whole standing flow, folded
3 4 KEEP ADD               # 3 4 7 — the operands stay, the sum branches above
NIL 1 LT                   # UNKNOWN — no value to compare, so nothing is settled
FALSE ^ { 1 0 DIV } 42     # 42 — the blocked unit is never evaluated
```

`SPECIFICATION.md` is canonical.

---

## The water is load bearing

Ajisai is described in terms of water, and every figure of speech corresponds to
a rule. There is no word in the language whose only purpose is to sound like
water.

| Figure | Rule |
|---|---|
| the flow | evaluation order — values move left to right |
| the basin | the stack: the flow's current cross-section |
| `TOP` / `STAK` | where the next word draws from: the surface, or the whole standing flow |
| `EAT` / `KEEP` | whether the next word swallows what it drew, or leaves it and branches above |
| `VENT` | release or block the next source unit, without evaluating it |
| `UNKNOWN` | the flow reached the gauge and did not settle |
| `NIL` | the flow arrived carrying no value |
| an error | the flow never formed |

---

## What makes it Ajisai

**Every number is exact.** An arbitrary-precision rational, always. There is no
floating point anywhere in the language — no value approximates, no operation
rounds, and no result depends on how far a computation was carried. `0.1` is
`1/10`.

**Three truth values.** `TRUE`, `FALSE`, `UNKNOWN`, under Strong Kleene logic.
`UNKNOWN` is reachable from source — a comparison against `NIL` cannot be
settled, so it isn't — and it never quietly becomes a Boolean. Where a word must
decide and the answer is `UNKNOWN`, the word raises an error rather than picking
a side.

**Three different negatives, kept apart.** `NIL` is a flow that arrived carrying
no value. `UNKNOWN` is a flow that arrived and did not settle. An error is a
flow that never formed. None of them converts into another.

**Two orthogonal flow modes.** `TOP`/`STAK` chooses where the next word draws
from; `EAT`/`KEEP` chooses whether it swallows what it drew. They are
independent, they compose, and they are implemented once — in the operand layer,
not per word.

**A vent, not an `if`.** `VENT` decides whether the next source unit is
evaluated at all. A blocked unit cannot divide by zero, cannot name a word that
does not exist, and cannot reach the dictionary. Two-branch selection falls out
of `KEEP` and `VENT` together:

```
5 0 GT KEEP VENT { "positive" } NOT VENT { "not positive" }
```

**A Semantic Plane that is honest about itself.** Every value carries a reading
— `RAW`, `TEXT`, or `INTERVAL` — stored on the value and nowhere else. It
changes how the value renders and what `ROLE` reports. It never changes a
computation, and the specification says so rather than claiming a purity the
implementation quietly breaks.

**Symbol notation as a first-class surface.** `1 2 +` and `1 2 ADD` are the same
program, normalized in the parser, so every layer after it sees one name. Twelve
aliases, one table.

**Machine-readable contracts.** Every word declares its stack effect, its types,
and its stance towards `NIL` and `UNKNOWN`. `ajisai words` prints the lot as
JSON, generated from the live registry.

---

## Try it

```sh
cargo run -p ajisai-core --bin ajisai -- eval '1 2 3 4 STAK ADD'
cargo run -p ajisai-core --bin ajisai -- repl
```

```
ajisai run <file>       run a source file and print the resulting flow
ajisai eval <source>    run a source fragment
ajisai lint <file>      report obvious contract inconsistencies
ajisai fmt <file>       print the program in canonical form
ajisai words            print the vocabulary manifest as JSON
ajisai repl             read, evaluate, print
```

## A taste

```
# The flow persists; the basin collects.
[ 1 2 3 ] { 2 MUL } MAP            # [ 2 4 6 ]
[ 1 2 3 STAK ADD ]                 # [ 6 ]

# A definition is two values and a word. No defining syntax.
{ 2 MUL } "DOUBLE" DEF  21 DOUBLE  # 42

# The Semantic Plane changes the reading, never the data.
[ 104 105 ] >TEXT                  # "hi"
"A" [ 65 ] EQ                      # TRUE

# Absence and indeterminacy propagate by different rules.
NIL 1 ADD                          # NIL
UNKNOWN 1 ADD                      # UNKNOWN
NIL UNKNOWN ADD                    # NIL — absence wins
NIL NOT                            # error: NIL is not a truth value
```

---

## What is in this repository

```
SPECIFICATION.md        the language — canonical
crates/ajisai-core/     the language: library and CLI
crates/ajisai-music/    exact just intonation, as an external package
crates/ajisai-audit/    content addressing and receipts, as an external package
docs/                   the Semantic Plane, the ontology, contracts, migration,
                        implementation notes, and the playground specification
```

Ajisai Core has three dependencies, all of them exact integer arithmetic. It has
one execution path, no feature flag that changes what a program means, and no
knowledge that any package exists.

The packages exist to prove the boundary is real: `ajisai-music` and
`ajisai-audit` are built entirely from the public extension surface, and an
implementation that registers neither is completely conforming.

## Building

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo build --workspace --release
```

## Coming from an earlier Ajisai

Version 1.0 is a rebuild. Programs written against the pre-1.0 specification
will not run. `docs/migration.md` records what changed and how to move.

## License

MIT. See `LICENSE`.
