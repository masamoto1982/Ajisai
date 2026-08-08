# Ajisai Tensor Profile 0.1

Status: **Draft normative profile**

Profile identifier: **`org.ajisai.tensor/0.1`**

This profile adds explicitly approximate dense tensors without changing Ajisai
Core. It is selected explicitly by a graph or execution request; Core continues
to have six canonical value domains, exact Scalars, and its frozen Word set.
The machine-readable operator contracts in `tensor-profile-v0.1.json` are
normative alongside this document.

The reference runtime types live in `rust/src/tensor_profile`. They are kept
outside `KernelValue`; selecting this profile therefore cannot silently make an
approximate tensor a Core value.

## Value and conversion boundary

A Tensor is the immutable semantic triple `(dtype, shape, elements)`. Shape is
a finite sequence of non-negative dimensions and elements are stored in
row-major logical order. Device, allocation, strides, layout, fusion, and
backend are execution properties, not value identity.

Profile 0.1 supports the approximate numeric dtypes `f32` and `f64`, the exact
numeric dtype `q`, and the separate predicate dtype `bool`. A predicate carries
selection decisions such as an attention mask; it is not a numeric element
type, and every arithmetic and reduction operator rejects it rather than
reading a number as truthy or a predicate as zero-or-one. An exact Core Scalar
enters the approximate domain only through `CAST`; no operator performs an
implicit Scalar-to-Tensor or dtype conversion. `CAST` uses IEEE-754
round-to-nearest, ties-to-even.

`q` elements are exact rationals. They are the same numbers Core computes with,
so an exact tensor result is the mathematically exact value and every backend
must produce the identical rational — there is nothing left for a backend to
decide. A `q` element is never `NIL`: `NIL` is Core's absence marker, not a
number, and an exact tensor carrying one is a contract error rather than a
value.

Each operator declares the numeric domain its contract is written over, as
`dtypeDomain`. `approximate` means the operator's value is irrational for
rational inputs, so there is no exact result to return; `exact` means it acts
on a denominator, which the approximate dtypes do not have; `any` means both.
For an `any` operator the declared numeric tolerances describe approximate
evaluation only — exact evaluation is bitwise by construction. Both the graph
validator and the reference backend reject a dtype outside an operator's
declared domain, so an unexecutable graph is never certified as valid.

Tensor NaN and infinities are floating-point elements, not `NIL`. A resource
budget failure produces `NIL(spaceExhausted)`. A malformed dtype, rank, axis, or
incompatible shape is a contract error. These outcomes must not be substituted
for one another.

## Shape algebra and contracts

Dimensions are non-negative integers. Every dimension product, stride, byte
count, and temporary-memory estimate is checked before allocation. Symbolic
dimensions unify by name. `...` denotes broadcast-compatible leading axes;
operators otherwise use the exact relations recorded in the JSON contract.

Each primitive contract states its input and output signatures, shape relation,
dtype rule, determinism class, numeric model, differentiation/VJP rule, and
resource complexity. These operators are profile primitives, not Core Words.
NN and LLM operations such as attention, RMSNorm, RoPE, optimizers, and sampling
must be libraries or graph compositions over these primitives.

## Numerical conformance

The reference CPU implementation is deterministic for a fixed graph, inputs,
and RNG key. `bitwise` requires identical dtype bits. `bounded` permits only the
operator's declared absolute and relative tolerance. Approximation is therefore
specified rather than silently ignored. Accelerated backends must emit an
execution receipt naming profile version, graph identity, backend, dtype policy,
determinism policy, optimization passes, and RNG key identity.

## Randomness

There is no ambient RNG. `RANDOM_UNIFORM(key, shape, dtype)` is a pure function;
the same 256-bit key and arguments produce the same tensor. `SPLIT_KEY` derives
independent child keys. Training steps pass keys and updated state explicitly.

## Typed Graph IR and differentiation

Profile programs lower from validated stack semantics to the SSA/dataflow form
defined by `typed-graph-ir.schema.json`. Every node names an operator semantic
ID and every value carries dtype and shape. Reverse-mode autodiff is a graph
transformation driven by primitive VJP IDs; it is not interpreter tape state.
Unknown shapes, types, effects, or VJPs are never certified as valid.

The reference validator currently certifies `tensor.matmul.v1`: both inputs
must share a dtype, have rank at least two, unify their contraction dimension,
and have broadcast-compatible leading dimensions. Its declared output dtype and
shape must equal the inferred `[..., M, N]`; merely authoring a plausible output
annotation is not sufficient.

The reference CPU backend implements that same contract for concrete f32/f64
tensors. It evaluates each dot product in increasing `K` order, applies batch
broadcasting before indexing, and validates output elements and bytes before
allocating the result buffer. Accelerated kernels may replace this loop only
under the numerical contract recorded for `tensor.matmul.v1`.

The reference backend also implements the shape-preserving `tensor.exp.v1`,
`tensor.log.v1`, and `tensor.rsqrt.v1` primitives for f32/f64. Their graph
contracts require exactly one input and one output with identical dtype and
shape. IEEE results such as NaN and infinity remain Tensor elements and are not
converted to NIL, so `rsqrt(0)` is positive infinity and `rsqrt(x < 0)` is NaN.
`tensor.rsqrt.v1` is `bounded`, not `bitwise`: a backend may fuse the square
root and the reciprocal only inside the tolerance recorded for the operator.

`tensor.where.v1` selects elementwise between two operands of a common dtype
`D` under a `bool` predicate. It is the only operator whose inputs deliberately
do not share a dtype, and the graph validator rejects a numeric predicate
instead of interpreting nonzero as true. All three operands broadcast against
one another and the output shape is their three-way broadcast. Selection copies
the chosen element rather than computing with it, so the negative infinity used
to mask attention scores can never enter an arithmetic result.

`tensor.reduce_sum.v1` takes one Tensor plus graph attributes `axes` (a required
array of unique zero-based axes) and `keepDimensions` (an optional Boolean,
default `false`). Reduction visits input elements in row-major order, preserves
the dtype, and either removes reduced axes or replaces them with dimension one.
`tensor.reduce_max.v1` uses the same attribute and shape rules, propagates NaN,
and uses negative infinity as the identity for an empty reduced slice.

`tensor.add.v1`, `tensor.sub.v1`, `tensor.mul.v1`, and `tensor.div.v1` are
elementwise operations. Inputs must share a dtype; shapes broadcast from the
trailing axis using equality-or-one, with no implicit cast. Over an approximate
dtype they are IEEE operations and division by zero produces IEEE infinity or
NaN inside the Tensor rather than NIL. Over `q` they are exact, and division by
zero is a contract error: a rational divided by zero has no rational value, so
none is invented. `tensor.reduce_max.v1` differs for the same reason — the
approximate path uses negative infinity as the identity of an empty slice, and
the rationals have no such element, so an empty exact maximum is reported
rather than answered.

## Bounding exact representation size

An exact rational records the whole history of the computation that produced
it: denominators multiply under both addition and multiplication, so a chain of
operations grows one without bound. This is a representation-size problem, not
an accuracy problem, and Profile 0.1 addresses it explicitly.

`tensor.regrid.v1` rounds every element of a `q` tensor to the nearest multiple
of `1/d`, where the required `denominator` attribute is a positive integer `d`.
It is the tensor lifting of Core's `QUANTIZE` word and inherits its tie rule,
halves away from zero. Rounding is exact and fully specified, so the operator is
`bitwise` deterministic; each element moves by at most `1/(2d)`.

Two properties make it the profile's growth strategy rather than a convenience.
Once every element of both operands of a contraction lies on the grid `1/d`,
each product already carries the denominator `d²`, so the contraction's
denominator is `d²` regardless of the reduction length — the length drops out.
Applying `REGRID` once per layer then holds the denominator at `d` across
depth. Representation size therefore stops depending on either the width or the
depth of the graph.

`REGRID` declares no VJP. Rounding to a grid has zero derivative almost
everywhere, and a straight-through estimator is a different function; treating
one as the other would make a gradient graph disagree with its forward graph,
so it requires its own contract.

Denominator size is a budgeted resource alongside element and byte counts.
Every tensor is checked against the declared ceiling as it is constructed, so a
graph that does not regrid often enough reports the operation that exceeded it
instead of continuing into unbounded arithmetic.

Numerically stable softmax is intentionally a library graph composition:
REDUCE_MAX with retained axes, subtraction, EXP, REDUCE_SUM with retained axes,
and division. It is not a Core Word or a Tensor Profile primitive. Masked
attention prefixes that chain with WHERE, and RMS normalization is likewise a
composition — square, REDUCE_SUM with retained axes, scale to a mean, add
epsilon, RSQRT, multiply. Neither becomes a primitive by being useful.

`execute_graph` is the reference bridge from the exchange IR to that backend.
It validates the graph before execution, binds symbolic dimensions from runtime
inputs consistently across the graph, checks concrete input annotations, and
returns only the declared SSA outputs. A missing input or a runtime tensor that
does not satisfy its graph type is an execution error rather than an implicit
reshape or cast.

Graph identity hashes the canonical graph, operator semantic IDs, tensor types,
constants, and referenced artifact identities. Backend and device are excluded
from semantic identity and recorded in the execution receipt instead.
