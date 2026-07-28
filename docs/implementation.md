# The reference implementation

How `ajisai-core` is built. None of this is normative; `SPECIFICATION.md` is.

## Layout

```
crates/ajisai-core/src/
  number.rs       exact rationals over arbitrary-precision integers
  value.rs        the six value shapes, and Data Plane equality
  role.rs         the Semantic Plane: three roles, one admits/retain rule
  error.rs        every error condition
  syntax.rs       tokenize, parse, render; the source-unit tree
  alias.rs        the one alias table
  mode.rs         TOP/STAK × EAT/KEEP
  k3.rs           the Strong Kleene tables
  contract.rs     the word contract
  words/          the vocabulary, one registry table
  interpreter.rs  the one execution path, the operand layer, VENT
  lint.rs         the contract lint
  manifest.rs     the vocabulary as JSON
  extension.rs    the package surface
  bin/ajisai.rs   the CLI
```

## Three dependencies

`num-bigint`, `num-integer`, `num-traits`. That is the whole list, and all three
are exact integer arithmetic. There is no serialization framework (the manifest
writes its own JSON — the shape is small and fixed, and the dependency list is
worth more than the twenty lines saved), no hashing, no async runtime, no
backend abstraction.

`ajisai-audit` adds nothing: it implements SHA-256 in-crate against the FIPS
180-4 vectors rather than take a dependency in a trust-sensitive position.

## The operand layer

`Interpreter::apply_op` is where `TOP`/`STAK` and `EAT`/`KEEP` are implemented
for operand-to-result words — once, not per word. It selects operands, calls
the word, and commits the result. `Interpreter::apply_full` does the same for
words that reach into the interpreter: `KEEP` still works there, because the
operands can be remembered and laid back underneath what the word produced.

Most words are written as `fn(&str, &[Value]) -> Result<Vec<Value>>`: operands
in, results out, no interpreter and no mode. That signature is what makes the
single implementation possible, and it is what lets `STAK` re-drive a word.

A word that needs the interpreter — to run a quote, to read the flow's depth,
or to reach the dictionary — is written as `fn(&mut Interpreter) -> Result<()>`.
It still declares a real stack effect, and `KEEP` still applies to it; only
`STAK` does not, because the common layer cannot call it repeatedly.

Because the word runs before the stack is touched, word-level atomicity
(`SPECIFICATION.md` §5.7) falls out of the shape of the function rather than out
of a snapshot on every step.

## `VENT`

`Interpreter::run_vent` and `unit_len`, in the same file. `unit_len` is the sole
definition of how far a source unit extends, and the lint calls the same
function rather than reimplementing the rule.

Laziness is structural: a blocked unit is a slice the evaluator does not visit.
There is no evaluate-then-discard path that could leak an effect.

## Roles

`Value` carries its role in a field. `role::admits` is the one well-formedness
predicate; `role::retain` is `admits` with a `RAW` fallback. Assertion and
propagation are the two callers, and a test asserts they agree.

## The registry

`words::core_words()` builds a `Vec<Word>`, each a contract plus an
implementation. It is a plain table, deliberately: it is meant to be read
top to bottom.

Package words enter the same map through `register_package`, so the modes, the
lint, the manifest, and the error paths apply to them with no per-package
handling anywhere.

## Budgets

Nesting depth (512) and vector size (1,000,000). Both are budgets rather than
semantics: exceeding one is an error, never a silent truncation, and a program
that stays inside them behaves identically at any budget.

## Building

```sh
cargo build --workspace --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## Notes for a second implementation

- The alias table is twelve entries and normalization happens in the parser.
  Doing it anywhere later means every downstream layer has to know about both
  spellings.
- Store the role on the value. Every other arrangement needs synchronisation.
- Keep the unchecked role constructors private. A public one is a way for a
  caller to build a value whose role its shape does not admit, and the
  specification states that as an invariant.
- Implement the modes in one operand layer. Per-word branches will diverge.
- Make the abstract flow in your lint go opaque early and often. A lint that
  guesses gets switched off — and make each slot a *set* of possibilities, or
  it will report `UNKNOWN 1 ADD`.
- Declare what `STAK` means per word. Deriving it from arity is the same
  mistake as Flow Mass Conservation wearing a different hat.
- Keep a word's stack effect separate from how you dispatch it.
