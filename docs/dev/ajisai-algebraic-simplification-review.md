# Ajisai algebraic simplification review

> Status: Design review / non-canonical. `SPECIFICATION.md` remains the canon; the mathematical formalization is a lens for compression, portability review, and design cleanup, not a second authority.

## 1. Purpose

This review evaluates the requested theme, **language refinement through algebraic simplification**, against the current repository state.

The instruction is directionally sound because Ajisai already models programs as state transformers over `Σ = Stack × Dict × Eff`, tracks formalization coverage, and uses conformance/law tests to avoid implementation-language lock-in. The useful refinement is to make the task explicitly metadata-first and observation-preserving:

- Use the mathematical formalization not only as a portability check, but also as a way to discover redundant semantic primitives.
- Reduce duplicate **semantic primitives**, not surface vocabulary. English words, aliases, and symbol sugar may remain rich if they desugar homomorphically.
- Keep `SPECIFICATION.md` canonical. Algebraic simplification proposes compression candidates; it must not override specified observations.
- Prefer small, reversible changes: metadata, validator checks, documentation, and clarification of existing mathematical definitions before any runtime rewrite.

The original instruction is too broad for a single safe implementation pass if interpreted as a mandate to reclassify and refactor every word. The improved interpretation used here is: classify the coverage entries that already exist, document the compression path, extend validation, and only make a semantics-preserving formalization clarification.

## 2. Classification

The coverage metadata now distinguishes formalization progress (`status`) from algebraic role (`semantic_role`). The currently tracked entries classify as follows.

| Area / entry | Semantic role | Algebraic family | Rationale |
| --- | --- | --- | --- |
| `TRUE`, `FALSE` | `Primitive` | `k3-truth` | Literal injections into the K3 truth domain; they are not numbers. |
| `AND`, `OR`, `NOT` | `Derived` | `k3-truth` | Strong Kleene meet, join, and order-reversing involution. |
| `ADD/SUB/MUL/DIV` | `Derived` | `exact-arithmetic` | Special coefficient choices of the exact-real bihomographic/Gosper schema, with NIL-passthrough behavior. |
| `EQ/LT/GT` | `Derived` | `observation` | Budgeted exact-real ordering projected into K3 truth observations. |
| `'math' IMPORT SQRT` | `Derived` | `exact-arithmetic` | Continued-fraction exact-real construction and Gosper-compatible observation; not a host approximation. |
| `KEEP`, `STAK` | `Derived` | `modifier` | State-transformer combinators built from consumption policy and region selection. |
| `NOW`, `RANDOM`, `SERIAL-*`, missing-capability diagnostics | `HostedEffect` | `hosted-effect` | Capability-gated host observations represented by `Eff` append plus structured diagnostics. |
| `vector/tensor` | `Derived` | `structure-lift` | Applicative/zip lift of scalar operations over tensor index structures. |
| `record` | `Primitive` | `dictionary` | Finite partial map with insertion order is a distinct value-domain component. |
| `string` | `Primitive` | `exact-scalar` | Codepoint/text value domain tracked separately from numeric scalar arithmetic. |
| `IMPORT / DEF / dictionary resolution` | `Primitive` | `dictionary` | Dictionary lookup and state-transformer composition are core semantic machinery. |
| `SPAWN/AWAIT` | `Exploratory` | `state-transformer` | Modeled but excluded from the current Core portability contract. |

### Role definitions used

- `Primitive`: a semantic domain or state operation not naturally derivable from the currently documented Ajisai algebra.
- `Derived`: an operation expressible as a parameterization, lift, handler, or combinator over existing primitives.
- `Sugar`: syntax-only surface that desugars without changing denotation. No tracked `formalization-coverage.json` entry currently uses this role.
- `HostedEffect`: externally supplied capability or host interaction with a structured observation boundary.
- `Exploratory`: modeled but not part of the Core portability contract.
- `NotPortableYet`: not currently safe to classify as Core. No tracked entry currently uses this role.

### Semantic primitive inventory (closed `derived_from` vocabulary)

The success criterion of this theme is that Ajisai reads as *few semantic primitives + derived words that are traceable through `derived_from`*. To make that checkable rather than aspirational, `formalization-coverage.json` now declares the primitives explicitly in a top-level `algebra_primitives` registry, and every `derived_from` reference must resolve to a declared primitive (enforced by `check:formalization-coverage`).

The inventory grew as later rollout phases classified more surface and module words; the registry now declares 30 primitives, grouped by algebraic family:

| Family | Primitives |
| --- | --- |
| `k3-truth` | `algebra.k3.domain`, `algebra.k3.meet`, `algebra.k3.join`, `algebra.k3.involution` |
| `exact-arithmetic` | `algebra.exact-real.bihomographic`, `algebra.exact-real.budgeted-order`, `algebra.exact-real.continued-fraction`, `algebra.exact-real.gosper` |
| `exact-scalar` | `algebra.exact-scalar.codepoint-sequence` |
| `bubble` | `algebra.bubble.domain`, `algebra.bubble.passthrough`, `algebra.bubble.handler` |
| `structure-lift` | `algebra.structure-lift.indexed-sequence`, `algebra.structure-lift.applicative`, `algebra.structure-lift.zip`, `algebra.structure-lift.reshape-group` |
| `modifier` | `algebra.modifier.consumption.keep`, `algebra.modifier.consumption.eat`, `algebra.modifier.region.stack`, `algebra.modifier.region.top` |
| `state-transformer` | `algebra.state-transformer.combinator`, `algebra.state-transformer.composition`, `algebra.state-transformer.identity`, `algebra.eff.append`, `algebra.handle.domain` |
| `dictionary` | `algebra.dictionary.lookup`, `algebra.dictionary.finite-partial-map` |
| `observation` | `algebra.observation.structured-diagnostic`, `algebra.observation.digest` |
| `hosted-effect` | `capability.check` |

Reading note: `derived_from` records what an entry *rests directly on*, regardless of role. A `Primitive` Ajisai word rests on its defining domain primitive (e.g. `TRUE` rests on `algebra.k3.domain`); a `Derived` word rests on the operation(s) it specializes (e.g. `AND` rests on `algebra.k3.meet`). The `semantic_role` field, not the presence of `derived_from`, is what distinguishes a primitive injection from a derived operation.

## 3. Algebraic compression candidates

### 3.1 K3 truth operations

The formalization already exposes the desired compression: `K3 = {F < U < T}`, with `AND = meet`, `OR = join`, and `NOT` as the order-reversing involution. The design invariant is that `K3` is a separate summand of `V`, so truth values remain distinct from exact numbers.

Recommended next step: move law and conformance naming toward the algebraic family, for example “K3 meet/join/involution” rather than treating `AND`, `OR`, and `NOT` as independent primitive truth tables.

Must preserve:

- `TRUE 1 EQ => FALSE` style observations.
- `UNKNOWN` as K3 truth, not NIL.
- NIL priority rules where Bubble propagation and K3 logic interact.

### 3.2 Arithmetic operations

`ADD`, `SUB`, `MUL`, and `DIV` are already documented as coefficient instances of a bihomographic transform:

```text
z(x,y) = (a·xy + b·x + c·y + d) / (e·xy + f·x + g·y + h)
```

This is a strong compression candidate because the words differ by coefficients, not by semantic kind. The remaining design work is to make conformance and coverage clearly say that the primitive is exact-real continued-fraction arithmetic/Gosper normalization, while the surface arithmetic words are derived instances.

Must preserve:

- Exact rational observations.
- Irrational continued-fraction observations.
- Division-by-zero as NIL/Bubble, not host exception.
- No return to `sqrt(...)` wrappers or approximate `~n/d` rational fallback.

### 3.3 Vector / tensor lift

Vector and tensor arithmetic is best treated as structure-lifted scalar semantics:

```text
unary:  map scalar operation over shape
binary: zipWith scalar operation with documented broadcast rules
```

The simplification is not “vectors are scalars”; it is “the scalar operation is the denotation, and tensor traversal/broadcast is the lift.” This keeps shape mismatch and scalar/structure mixing as explicit structure-lift rules instead of duplicating every scalar word as a vector word.

Recommended next step: classify conformance cases as either scalar primitive examples or structure-lift examples.

### 3.4 NIL / Bubble / OR-NIL

The formalization already gives the clean compression:

```text
M(X) = X + (⊥ × R∞)
OR-NIL = orelse handler / recovery
```

This should be treated as a Bubble monad plus one recovery handler. It avoids treating each NIL-producing word as an independent special case.

Must preserve:

- NIL is not `FALSE` and not numeric zero.
- NIL and `Error` remain separate layers.
- `UNKNOWN` remains a truth value, not operational absence.

### 3.5 Modifier system

`KEEP`, `STAK`, `EAT`, and `TOP` can be compressed as:

```text
modifier = region selection × consumption policy × reconstruction
```

Current formalization already expresses this as region and consumption combinators over a base state transformer. The main smell is not the model but the possibility that case names and coverage entries make `KEEP` and `STAK` look like independent primitives. They should be tracked as derived modifier combinators.

Must preserve:

- Existing `STAK` conformance observations.
- Mass-conservation laws.
- Composition as phrase-level state-transformer transformation.

### 3.6 Hosted effects

Hosted effects should remain outside Core and be modeled as:

```text
hosted effect = capability check + Eff append + structured observation
```

This prevents wall-clock time, CSPRNG entropy, serial transport, or missing-capability host details from becoming Core semantics. Coverage metadata should reject `semantic_role = HostedEffect` entries classified as `Core`.

## 4. Design smells found by formulas

| Smell | Evidence | Improvement |
| --- | --- | --- |
| Value-domain formula omitted `K3` even though the prose required truth values to be disjoint from numbers. | The main `V` equation listed exact numbers, vectors, records, Bubble, blocks, and handles, while the note said truth values enter separately. | Clarify the main equation as `V = 𝔸 ⊎ K3 ⊎ V* ⊎ ...` so the direct-sum invariant is visible at the definition site. |
| Coverage progress and algebraic role could be conflated. | `status` previously said whether an entry was `Formalized`, `Sketched`, `HostedEffect`, etc., but not whether a formalized word was primitive or derived. | Add `semantic_role`, `primitive`, `derived_from`, and `algebraic_family` metadata. |
| Hosted effects can be accidentally treated like Core if only the status name is checked. | Hosted entries use host capabilities and structured effect observations. | Validator now rejects `semantic_role = HostedEffect` with `classification = Core`. |
| Arithmetic words appear as a semantic area rather than derived coefficient instances. | The formalization compresses arithmetic into a single bihomographic schema. | Coverage marks arithmetic as `Derived` from exact-real bihomographic arithmetic. |
| Modifier words can look primitive in conformance names. | `KEEP` and `STAK` are user-visible words, but mathematically they are combinators over state transformers. | Coverage marks them `Derived` from modifier region/consumption combinators. |

## 5. Proposed simplification path

### Phase 1: Add classification metadata

Completed in this pass for current coverage entries. `semantic_role` is intentionally independent of `status`.

### Phase 2: Document compression candidates

Completed in this review for K3, exact arithmetic, structure lift, Bubble/OR-NIL, modifiers, and hosted effects.

### Phase 3: Reclassify tests by semantic primitive

Recommended follow-up: annotate law-test groups and conformance sections as either primitive-domain checks or derived-word examples. For example, K3 law tests should be treated as one algebra family with word-level examples.

Realized in this pass as a reverse index rather than by re-tagging tests in place: `scripts/generate-primitive-test-map.mjs` (`npm run primitive:test-map`) inverts each word's `derived_from` / `law_tests` / `conformance_cases` into a primitive → tests map at `docs/primitive-test-map.json`, so every declared primitive is traceable to the concrete tests exercising the words that rest on it. `check:formalization-coverage` additionally emits a non-fatal note if any declared primitive is exercised by no test, so a newly admitted primitive cannot stay silently untested.

### Phase 4: Move small derived words toward shared primitive implementations

Safe future candidates, after tests are clearly grouped:

- Use a single K3 helper for meet/join/involution if implementation duplication exists.
- Ensure arithmetic dispatch is visibly coefficient-driven where possible.
- Keep modifier evaluation factored by region and consumption policy.

### Phase 5: Prevent regressions through coverage and conformance

Keep `check:formalization-coverage` in CI. Extend it only backward-compatibly: new fields should be validated when present, while older metadata-free entries should not break consumers unless the project later decides to require these fields globally.

Done in this pass: the validator now closes the `derived_from` vocabulary against the `algebra_primitives` registry. Any new derived word must point at a declared semantic primitive, and a declared-but-unused primitive is surfaced as a non-fatal note so dead metadata is visible. This turns "few primitives, traceable derived words" from a documentation claim into a CI-checked invariant without changing any user-visible observation.

## 6. Revised instruction shape

A safer version of the original instruction is:

1. Treat `SPECIFICATION.md` as canonical and the formalization as a design lens.
2. Add algebraic-role metadata to existing coverage entries without changing user-visible observations.
3. Validate the new metadata with backward-compatible checks.
4. Document compression candidates and design smells before runtime refactors.
5. Make only semantics-preserving clarifications in formalization documents.
6. Defer implementation refactors until conformance/law tests are grouped by algebraic primitive and derived-word examples.
