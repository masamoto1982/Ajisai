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
    effect              Pure | Dictionary
    summary
}
```

Nine fields, every one of them read by something. `docs/semantic-ontology.md`
names the readers.

There is one registry. `ajisai words` prints it as JSON, generated from the live
registry at the moment you ask — there is no checked-in copy to drift and no
generator to re-run.

## What the lint is

> The contract lint reports obvious inconsistencies between declared stack
> effects and types.

That sentence is the guarantee, and it is deliberately weak.

It reports:

1. **Obvious stack underflow** — a word needs two values and the flow demonstrably holds one.
2. **Obvious type mismatch** — a vector reaching a numeric operand position.
3. **Unknown words.**
4. **A dangling mode**, and a `VENT` with no unit.
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

## Where it stops

The lint walks an abstract flow: a known run of typed slots, or *opaque*. It
goes opaque — and stops reporting — at a word with a dynamic stack effect, a
user definition, an armed mode, and after a vent.

Each of those is a place where the true flow depends on something the lint does
not model. It could guess. It does not, because a false accusation costs more
than a missed one: a lint that cries wolf on correct programs gets switched off,
and then it catches nothing at all.

`crates/ajisai-core/tests/lint.rs` fixes this boundary from both sides — the
things it must report, and the correct programs it must stay quiet about.

## Run-time checking is unaffected

The lint is additive. Every word still validates its own operands and raises its
own error; removing the lint entirely would not change what any program does.
