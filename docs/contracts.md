# Word contracts and the contract lint

Companion to `SPECIFICATION.md` §11 and §14.

## The contract

Every word carries one:

```
WordContract {
    name
    stack_effect        ( vector quote -- vector )
    arity               Fixed { inn, out } | Dynamic
    input_types
    output_types
    nil_policy          { rejects, may_produce }
    unknown_policy      { rejects, may_produce }
    stak                MapEach | FoldLeft | Unsupported
    role_required       Option<(operand, role)>
    summary
}
```

Ten fields, every one of them read by something. `docs/semantic-ontology.md`
names the readers.

**`arity` is the stack effect, not the dispatch.** `MAP` takes two values and
leaves one; that it needs the interpreter to run a quote is an implementation
fact and does not belong in its contract. Conflating them made the lint go
blind at every higher-order word, and made `KEEP MAP` a mode error for no
reason. `EXEC` is the one word in Ajisai Core whose effect is genuinely
dynamic.

**`stak` is declared, not derived.** See `SPECIFICATION.md` §8.3 for why: a
count of operands is not a meaning, and deriving one made `1 1 1 STAK EQ`
answer `FALSE`.

There is one registry. `ajisai words` prints it as JSON, generated from the live
registry at the moment you ask — there is no checked-in copy to drift and no
generator to re-run.

## What the lint is

> The contract lint reports obvious inconsistencies between declared stack
> effects and types.

That sentence is the guarantee, and it is deliberately weak.

It reports:

1. **Obvious stack underflow** — a word needs two values and the flow demonstrably holds one.
2. **Obvious type mismatch** — a vector reaching a numeric operand position, or a bare vector reaching a name position.
3. **Unknown words** — excluding the ones the source itself defines.
4. **A dangling mode**, a `VENT` with no unit, a `VENT` under `STAK`, and a gate that is definitely missing or definitely not a truth value.
5. **Advisories** where a value that may be `NIL` or `UNKNOWN` reaches a word that refuses it.

Errors are contradictions: the program will fail if it reaches that point.
Advisories are worth a look and may be exactly what was meant.

## What the lint is not

**It is not a verifier.** It does not decide whether a program terminates,
whether a division will be by zero, whether an index is in range, or whether a
program will succeed.

**It never blocks execution.** Findings are output; the interpreter runs the
program either way.

**It never claims safety.** A clean run prints:

```
nothing obviously wrong (this is not a proof of success)
```

The wording is deliberate. Each of these passes the lint and fails at run time,
and `crates/ajisai-core/tests/lint.rs` asserts both halves:

```
1 0 DIV
[ 1 2 ] 9 NTH
[ 1 2 ] { DUP } MAP
```

## A slot is a set of possibilities, not a type

Each slot in the abstract flow carries a kind and two flags: it may be `NIL`,
it may be `UNKNOWN`. A type contradiction is reported **only when every value
the slot could hold contradicts the contract.**

This is not pedantry. `UNKNOWN 1 ADD` is a correct program — `UNKNOWN`
propagates through arithmetic — and an earlier draft reported it as a type
error, because the slot's kind was "truth value" and `ADD` wants a number. That
is a false accusation on a correct program, which is the failure mode the whole
design is arranged against. Now the slot is read as "a truth value *or*
`UNKNOWN`", `ADD` accepts the second possibility, and nothing is said.

`TRUE [ 1 ] CONCAT` is still reported: no possibility satisfies `CONCAT`.

## Where it stops

The lint walks an abstract flow: a known run of possibility-slots, or *opaque*.
It goes opaque — and stops reporting — at `EXEC`, a user definition, an armed
mode, and after a vent.

Each of those is a place where the true flow depends on something the lint does
not model. It could guess. It does not, because a false accusation costs more
than a missed one: a lint that cries wolf on correct programs gets switched off,
and then it catches nothing at all.

Two things it does *not* go quiet about, which it used to:

* **A word the source defines for itself.** `{ 1 ADD } "B" DEF 1 B` used to draw
  `unknown word: B`. The lint now collects the names a program binds before it
  walks it.
* **A vent's gate.** Everything *after* a vent is unknowable, but the gate is
  drawn before the unit is even considered, so `VENT 1`, `1 VENT 2`, and
  `STAK VENT 1` are all reported.

`crates/ajisai-core/tests/lint.rs` fixes this boundary from both sides — the
things it must report, and the correct programs it must stay quiet about.

## Run-time checking is unaffected

The lint is additive. Every word still validates its own operands and raises its
own error; removing the lint entirely would not change what any program does.
