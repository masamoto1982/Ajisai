# Migration to Ajisai 1.0

Ajisai 1.0 is a rebuild, not a revision. The language was redesigned from its
own principles and the implementation was written from scratch. Programs written
against the pre-1.0 specification will not run.

This document is not a memorial. It records the changes that are observable from
a program, and how to move.

## The shape of the change

| | before | 1.0 |
|---|---|---|
| Rust source (all crates, incl. tests) | 288 files, 81,650 lines | 40 files, 7,230 lines |
| — of which Ajisai Core `src/` | — | 25 files, 4,644 lines |
| Vocabulary | 224 entries (98 core, 96 module, 20 aliases, 10 surface forms) | 54 words + 12 aliases |
| Exploratory words | 26 | 0 — the classification no longer exists |
| Value shapes | 12, including process and supervisor handles | 6 |
| Execution paths | greedy, elastic/hedged, compiled plan, SIMD, shadow validation | 1 |
| Crate dependencies (Core) | 11 | 3, all exact integer arithmetic |
| Cargo features affecting semantics | `elastic-engine`, quantization, tracing | none |
| Contract fields | 13, including water sensitivity, confidence, space class | 10, all read |
| Semantic role storage | value hint + stack roles | the value, alone |
| Docs | 104 files | 6 |

Counts are from `find crates -name '*.rs'` at the 1.0 tag, so they include the
conformance tests.

## Removed words

| Removed | Move to |
|---|---|
| `FLOW`, `~` | delete them — they were no-ops and never had an execution rule |
| `OR-ELSE` | `VENT` (§9), or K3 `OR` (§7), depending on which you meant — see below |
| `CONSERVE` | write the check explicitly: `FOLD` the vector and compare to the total |
| `SPAWN`, `AWAIT`, `KILL`, `MONITOR`, `SUPERVISE`, and the rest of the child-runtime vocabulary | nothing in Ajisai Core. They were exploratory and are gone, along with the `ProcessHandle` and `SupervisorHandle` value shapes |
| the `MUSIC` vocabulary | `crates/ajisai-music`, redesigned — see below |
| the audit vocabulary | `crates/ajisai-audit`, as a library rather than words |

### `OR-ELSE`

It conflated two different things, which is why it is gone rather than renamed.

*If you meant "use this value, or that one when it is absent":*

```
NIL?  KEEP VENT { DROP <fallback> }
```

or, more usually, restructure so the absence is handled where it arises.

*If you meant "either of these conditions holds":* that is K3 `OR`, and it now
gives you `UNKNOWN` where the old word would have quietly given you a Boolean.

*If you meant "try this, and use that if it fails":* Ajisai Core has no such
construct, deliberately. An error is a flow that never formed
(`SPECIFICATION.md` §10), and nothing in the language converts one back into a
value. Handle it in the host.

### `CONSERVE`

The old word was named for a conservation law that did not exist. What it
actually did — check that a vector of parts sums to a stated total — is an
ordinary numeric check, and now reads as one:

```
# does [ 33.33 66.67 ] sum to 100 ?
[ 33.33 66.67 ] 0 { ADD } FOLD 100 EQ
```

## Removed concepts

**Flow Mass Conservation** is gone: the theory, the `MassContract` type, the
`Fixed { consumes, produces }` classification, and the per-word conservation
metadata. What it described was a stack effect, and it is now called a stack
effect and written `( a b -- c )`.

**Elastic execution, hedged execution, SIMD execution, compiled plans, and
shadow validation** are gone. There is one execution path. A conforming
implementation may not offer a second one a program can select
(`SPECIFICATION.md` §14).

**The Exploratory classification** is gone, along with every word that carried
it and the review-gate machinery around it. A word is in Ajisai Core or it is in
a package.

**The Presentation Profile** is no longer in the language specification. Panels,
screen transitions, and panel reachability are in `docs/playground-ui.md`, which
is not a conformance condition.

**Content addressing is no longer part of word identity.** Two words with the
same name and body are the same word whether or not anyone has hashed them.

`crates/ajisai-audit` provides **digests, receipts, and verification** — and
those three only. **Lockfiles and source attestation were deleted, not moved**,
and this document promises nothing about their return: they were built around
content-addressed word identity, which the language no longer has, so porting
them would have meant reinventing what they were for. If a use for either shows
up, it will be designed against the language as it now is.

**The old Python implementation** (`python/`) is gone: it was self-declared
non-canonical, derived from a superseded specification, and not run in CI. The
Python differential oracle (`tools/ajisai-repro/`) is also gone — it validated
the old language, and there is no sense in checking a new language against an
oracle for a different one.

**The web playground** (`src/`, `src-tauri/`) is gone. See
`docs/playground-ui.md`.

## Changes that will surprise you

These are behaviour changes in constructs that still exist.

**`NIL` in a logical position is now an error.** `NIL NOT` fails. Previously a
`NIL` could drift into a Boolean position; that is how a three-valued logic
collapses back to two. Use `NIL?` first.

**`NIL NIL EQ` is `UNKNOWN`, not `TRUE`.** Two absences are not evidence of
sameness. Use `NIL?` to observe absence, which is a question observation can
settle.

**`FILTER` errors when its predicate answers `UNKNOWN`.** Keeping would read
`UNKNOWN` as `TRUE`; dropping would read it as `FALSE`. Decide it explicitly.

**A blocked `VENT` with an `UNKNOWN` gate pushes one `UNKNOWN`.** It used to
block silently, which made it indistinguishable from a `FALSE` gate.

**`VENT` enters a quote rather than pushing it.** `TRUE ^ { 1 2 ADD }` leaves
`3`. This is what makes `VENT` the branching construct, and why there is no
`IF`.

**`KEEP VENT` returns the gate to the surface *above* what the unit released.**
That ordering is what makes the two-branch idiom work; see `SPECIFICATION.md`
§9.4.

**A vector literal is a basin.** `[ 1 2 ADD ]` is `[ 3 ]`, not a parse error.

**A definition is `{ body } "NAME" DEF`.** There is no defining syntax.

**A name must be read as `TEXT`.** `{ 2 MUL } [ 68 79 85 66 76 69 ] DEF` is an
error, even though that vector spells `DOUBLE`. `DEF` and `DEL` are the two
words in the language that read the Semantic Plane (`SPECIFICATION.md` §6.3);
write `"DOUBLE"`, or say `>TEXT` and mean it.

**`STAK` is refused where it has no meaning.** `1 1 1 STAK EQ` used to compute
`EQ(EQ(1, 1), 1)` and answer `FALSE` about three equal values; `7 STAK EQ` used
to return `7` without running `EQ` at all. `STAK` now folds only **closed**
operations — `ADD`, `SUB`, `MUL`, `DIV`, `MIN`, `MAX`, `AND`, `OR`, `CONCAT` —
and maps only one-input words. Everything else is `ModeUnsupported`.

**`KEEP` reaches the higher-order words.** `[ 1 2 ] { 2 MUL } KEEP MAP` leaves
`[ 1 2 ] { 2 MUL } [ 2 4 ]`. It used to be a mode error, for implementation
reasons that had leaked into the language.

**Word names fold case in ASCII only.** `add` and `ADD` are one word; a
non-ASCII name is itself, and source must be NFC (`SPECIFICATION.md` §2.4).

**A dangling mode is an error.** `1 2 ADD KEEP` fails rather than silently
discarding the mode.

**Text is a vector.** `"A"` and `[ 65 ]` are equal (`SPECIFICATION.md` §6.4).

**Decimal literals are exact.** `0.1 0.2 ADD 0.3 EQ` is `TRUE`.

## Music

`ajisai-music` is not a port. Equal temperament is gone from it, because the
twelfth root of two is not rational and Ajisai has no floating point: an
exact language can only offer just intonation honestly. The vocabulary is seven
words over ordinary vectors, and the package defines its own stability policy.
