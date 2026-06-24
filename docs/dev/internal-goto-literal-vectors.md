# Compiling literal vectors

Status: prototype (non-canonical design note). Builds on the internal-GOTO
compiled-plan work (`internal-goto-tail-call.md`,
`internal-goto-cond-dispatch.md`). No surface syntax or value-semantics change;
the canonical definition remains `SPECIFICATION.html`.

## Motivation

Vectors are Ajisai's core aggregate, yet a vector literal `[ … ]` compiled to a
`FallbackToken`. Because `execute_compiled_line` drops the **whole line** to the
interpreter the moment any op is a fallback, every word that mentions a literal
vector ran fully interpreted — re-walking and re-allocating the vector on each
call. That includes the most common vector-arithmetic words:

```
{ [ 1 2 3 ] [ 4 5 6 ] + } 'VADD' DEF
```

and, crucially, the guard/body fragments of `COND` (`[ 0 ] >`, `[ 1 ] -`), which
is why the loop body stays interpreted even after tail-call elimination and
COND dispatch.

## Mechanism

`compile_word_definition` now tries to lower a vector literal into a single
prebuilt op:

```
CompiledOp::PushVectorLiteral(Value, Interpretation)
```

`try_collect_literal_vector` mirrors `Interpreter::collect_vector` for the
**literal subset** — numbers, strings, `TRUE`/`FALSE`/`NIL`, and nested literal
vectors — building the exact same promoted `Value` (including dense-tensor
promotion) and the exact same element hint. It returns `None` — leaving the
`FallbackToken` and the interpreter's behavior untouched — the moment anything
non-literal appears:

- a bare symbol that could resolve to a user word (`collect_vector` *executes*
  such words during collection, an effect we must not move to compile time),
- an empty vector `[ ]` (the interpreter rejects it; we must not silently build
  a NIL),
- a `|` separator, an unclosed vector, or nesting past the depth limit.

At runtime the op pushes the prebuilt value and pushes the element hint —
byte-for-byte what `execute_section_core`'s `VectorStart` arm does, so the
stack value *and* its rendered form (which depends on the hint: a boolean vector
renders `TRUE`/`FALSE`) are identical to the interpreted path.

## Why it is safe

- **Same value, same hint.** The construction reuses `from_vector_promoted` /
  `from_vector_promoted_with_hint`, and the element-hint rule is copied verbatim
  from `collect_vector`. Tensor promotion is deterministic and needs no
  interpreter state.
- **Conservative.** Anything that could differ (user-word elements, empty
  vectors, separators) is *not* lowered and keeps the interpreter path. Shadow
  validation compares the compiled and plain paths on every non-recursive call.
- **Mass / quantization analysis** treat `PushVectorLiteral` exactly like
  `PushLiteral` (pushes one pure value).

Toggle with `AJISAI_NO_VECTOR_LITERAL` or `Interpreter::set_vector_literal_enabled`.

## Measured effect

`cargo run --release --example vector_literal_bench` (a word doing 8-wide
element-wise add / multiply / subtract over literal vectors):

```
  lowering OFF (interpreter): ~12.1 us/call
  lowering ON  (compiled):    ~ 2.7 us/call
  speedup: ~4.4x
```

A **~4.4×** speedup for vector-arithmetic words, because the whole line now runs
compiled instead of being dropped to the interpreter.

## Next step

This is also the prerequisite for the larger loop win: with `[ 0 ]` / `[ 1 ]`
now compilable, a `COND` clause's guard (`[ 0 ] >`) and body (`[ 1 ] - DOWN`)
can be compiled into sub-plans and executed compiled rather than interpreted —
moving the recursion loop's *body* off the interpreter. That builds directly on
this op plus the precomputed `CondClause` table.

## Tests

`rust/src/interpreter/vector_literal_tests.rs`: ON vs OFF agree across numeric
(tensor-promoted), boolean, string, NIL-bearing, nested, and arithmetic shapes;
boolean rendering is preserved; non-literal vectors still fall back and execute
their words; the empty-vector error is raised on both paths.
