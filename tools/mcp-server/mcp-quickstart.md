<!-- MCP-facing preface to the generated writing protocol below.
     Hand-written source; `sync-assets.js` composes it with SKILL.md into
     assets/quickstart.md, and `selftest.js` runs every ```ajisai block here
     against the live backend, so no example can drift from what the server
     actually answers. -->

# Ajisai over MCP — read this first

You are connected to Ajisai: a small, bounded, deterministic engine whose
numbers are **exact rationals closed under square root** — no floats anywhere in
its supported domain. Use it when the answer has to be right and the failure
has to be explainable. It is not a general-purpose language runtime.

This preface is the MCP entry point. Everything after it is the generated
protocol for *writing* Ajisai, and is the reference to consult once you know
which call to make.

## 0. What it does, in one table

Ajisai is more than arithmetic, and a caller who assumes otherwise stops
reaching for it exactly where it would have helped. The 65 Words are:

| you need | Words |
|---|---|
| arithmetic | `ADD` `SUB` `MUL` `DIV` `MOD` `FLOOR` `ROUND` `QUANTIZE` `ABS` `NEG` `MIN` `MAX` `SQRT` `SUM` `RANDOM` |
| comparison and logic | `EQ` `NEQ` `LT` `LTE` `GT` `GTE` · `AND` `OR` `NOT` `TRUE` `FALSE` |
| vectors | arithmetic broadcasts element-wise; no separate vector Words |
| collections | `SORT` `ORDER` `UNIQUE` `TALLY` `GROUP` `ZIP` `RANGE` `FILL` `TAKE` `CONCAT` `REVERSE` `LENGTH` `GET` `PUT` `INDEX-OF` `COLLECT` |
| blocks over a collection | `MAP` `FILTER` `FOLD` `ANY` `ALL` |
| text | `CHARS` `JOIN` `TOKENIZE` `TRIM` `NUM` `STR` |
| absence | `NIL` `NIL?` `NIL-REASON` `VENT` (`^`) |
| naming, control, output | `DEF` `BIND` `DEL` · `COND` `EXEC` · `PRINT` `KEEP` `REFLECT` |

**Word names are exact and case-sensitive, and this is the whole list.** Do not
invent one: `vec-add`, `group-by` and `nil-or` are not Ajisai, and a name that
is not here does not exist under another spelling. When unsure, call
`word_contract` — it answers a near miss with `suggestions` — or read
`ajisai://vocabulary` for every contract at once.

Out of domain, and not worth a call: transcendental functions, floating point,
I/O, and anything that is really a program rather than a calculation.

Ajisai also carries no external or real-world reference data — no exchange
rates, no calendars, no reading speeds, no other language's syntax semantics.
Do not invent a plausible-looking number for one of those and run it through
`compute` to dress a guess up as an exact answer; if the question needs a
real-world fact rather than a value already given or derivable from first
principles inside this domain, answer directly without a call, or say you
don't know.

## 1. Choose a tool

| you want | call | pass |
|---|---|---|
| a number, a vector, an exact root, a `PRINT` line | `compute` | `source` |
| to know whether source parses and resolves, without running it | `check` | `source` |
| the inferred contract of Words *you* defined | `infer_contracts` | `source` |
| a built-in Word's contract, or "did I spell it right?" | `word_contract` | `word` |

All four take text, never a file path. To run a file, read it yourself and pass
its contents as `source`.

## 2. Read a result in this order

1. **`status`** decides everything else. `ok` — a value. `error` — an *Ajisai*
   error, still an ordinary successful call carrying a full diagnosis.
   `hostError` (with `isError` set) — this server failed, and your program may
   be fine.
2. On `ok`: `stackDisplay` is the final stack bottom→top, `output` holds `PRINT`
   lines, and `stack` is the machine-readable form of the same values. That is
   the general rule and it has exactly one exception: for an irrational square
   root `stackDisplay` is a *truncated* rendering and the value lives in
   `semantics.exactTerms` — see §4, which you must read before computing with
   any `SQRT` result.
3. On `error`: `diagnosis.why` and `.where` locate it; `diagnosis.candidates`
   names the Word you probably meant; `diagnosis.nextChecks[].code` is a stable
   identifier to act on — never match on its display text, which is localized.
4. On `hostError`: branch on `error.code`, and retry only if `error.retryable`
   is true. `mcp.limits` states every ceiling that applies.

A field carrying no value is **absent**, not `null`: a successful result simply
has no `diagnosis`. Test for presence.

Do not collapse these into "it worked / it broke". Retrying a division by zero
and rewriting a program that merely timed out are both wasted turns.

## 3. Absence is a value, not an exception

A partial operation that has no answer produces `NIL` carrying a reason, and the
call still succeeds:

```ajisai tool=compute status=ok stack="NIL"
1 0 /
```

The reason is on the value (`semantics.absence.reason`, here `divisionByZero`)
and in `errorFlowTrace` as a `nilProduced` event. Supply a fallback with `^`:

```ajisai tool=compute status=ok stack="[ 99/1 ]"
[ 1 ] [ 0 ] / ^ [ 99 ]
```

## 4. Exact arithmetic: what to read, and what not to

Rationals are exact and their display is exact too:

```ajisai tool=compute status=ok stack="1/1"
2 3 / 1 3 / +
```

An irrational square root is where display and value part company. On the
result of

```ajisai tool=compute status=ok
2 SQRT
```

read either of these two fields, in this order:

- **`semantics.exactDisplay`** — the value written short: `"sqrt(2)"`. Read this
  first. It is a display: read it, do not parse it.
- **`semantics.exactTerms`** — the value itself: a list of
  `{ numerator, denominator, radicand }` terms meaning `Σ (n/d)·√radicand`,
  arbitrary-precision integers as strings. Compute with this.

They are the same fact in two shapes and always appear together. Two *other*
fields on that same result are **not** the value, and reading either as if it
were will mislead you:

- `stackDisplay` shows the canonical continued fraction, truncated at a display
  budget (`( 1 ( 2 ( 2 …)`). It is a rendering, and an incomplete one.
- `value.numerator / value.denominator` is a rational *approximation*, marked
  `semantics.approximate: true`. It is a convenience, not the number.

Neither `exactDisplay` nor `exactTerms` appears on a plain rational or a vector
of rationals — there is no radical to write, and `stackDisplay` is already
exact for those.

One caution about `exactDisplay`: it writes the stored form faithfully, so two
values that *are* equal can be written differently — `8 SQRT` gives
`"sqrt(8)"` and `2 SQRT 2 SQRT +` gives `"2/1*sqrt(2)"`. Never compare these
strings to decide equality. Ask Ajisai, which decides on the exact value:

```ajisai tool=compute status=ok stack="TRUE"
8 SQRT 2 SQRT 2 SQRT + =
```

## 5. When a name is wrong, the answer says so

```ajisai tool=compute status=error
[ 1 2 3 ] LENGHT
```

That returns `status: "error"` with `diagnosis.candidates` beginning `LENGTH`.
Fix from the diagnosis rather than guessing. Before writing an unfamiliar Word,
`word_contract` gives its arity, purity and NIL policy — and answers a
misspelling with `suggestions`.

## 6. Bounds

Every result carries the profile it ran under in `mcp.limits`, alongside
`mcp.serverVersion`, `mcp.engineVersion` and `mcp.backend.kind`. Exceeding a
ceiling is a diagnosed outcome, never a hang. The full profile is also readable
without a tool call at `ajisai://limits`, the result contract at
`ajisai://schema/result`, and the whole vocabulary at `ajisai://vocabulary`.

---

The rest of this document is the generated writing protocol: syntax, the full
Word table, and worked examples verified against the real interpreter.

