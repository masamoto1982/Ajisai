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

Profile 0.1 supports `f32` and `f64`. An exact Core Scalar enters the approximate
domain only through `CAST`; no operator performs an implicit Scalar-to-Tensor or
dtype conversion. `CAST` uses IEEE-754 round-to-nearest, ties-to-even.

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

`execute_graph` is the reference bridge from the exchange IR to that backend.
It validates the graph before execution, binds symbolic dimensions from runtime
inputs consistently across the graph, checks concrete input annotations, and
returns only the declared SSA outputs. A missing input or a runtime tensor that
does not satisfy its graph type is an execution error rather than an implicit
reshape or cast.

Graph identity hashes the canonical graph, operator semantic IDs, tensor types,
constants, and referenced artifact identities. Backend and device are excluded
from semantic identity and recorded in the execution receipt instead.
