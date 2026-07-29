# Ajisai specification sources

This directory implements the one-way authority structure described by the
semantics-compaction plan. `language-semantics.md` and `gui-semantics.md` are
the normative sources. They retain raw HTML blocks during the physical split
so the generated integrated specification preserves the existing typography,
anchors, tables, and mathematical channels without a lossy Markdown migration.

`host-protocol-v1.schema.json` is the machine-readable compatibility boundary.
Within V1, consumers may receive new optional fields, but existing fields,
meanings, and tuple shapes cannot be removed, renamed, reordered, or changed.
A breaking protocol must coexist under a new major version.

The `freeze/` fixtures pin representative protocol payloads and the production
GUI surface. Contract tests deliberately inspect the existing sources rather
than duplicating GUI behavior in a replacement implementation.

`SPECIFICATION.html` is a distribution artifact assembled from the two
semantic sources, the implementation and quality fragments, and
`specification.template.html`. Run `npm run specification:generate` after an
authoritative source change and `npm run specification:check` in quality gates.

Phase 2 compacts the Language Semantics into the common laws in its ten
chapters. `semantic-families.json` is the shared-law vocabulary used by later
Word-schema migrations. The previous integrated wording remains available only
as the audit snapshot under `legacy/`; `legacy-clause-map.json` maps every one
of its headings to an active kernel clause. Run `npm run semantic-kernel:check`
to enforce the 500-line ceiling, family references, complete legacy mapping,
and the frozen 224-surface inventory.

Phase 3 migrates Word metadata one semantic family at a time. `words.schema.json`
defines the canonical differential contract and `words.json` contains only the
families already migrated; `migration.completeInventory` remains false until
all families have moved. `npm run word-schema:check` prevents a partial rollout
from changing names, aliases, descriptions, executor keys, clause links, or the
224-surface generated manifest. Runtime executors remain unchanged.

The second Phase 3 slice adds the orthogonal TOP/STAK and EAT/KEEP modifiers,
control directives, condition dispatch, code execution, conservation guards,
and lazy fallback. Child-runtime control Words remain deferred to the hosted
effect slice so their lifecycle and capability metadata move together.

The third Phase 3 slice migrates dictionary definition, deletion, observation,
and full or selective module import state. Internal dictionary effects are
recorded separately from `hostedEffect`, keeping deterministic resolution and
host capability mediation as distinct semantic axes.

The fourth Phase 3 slice migrates every Core vector, tensor, and higher-order
Word. Shape validation, reasoned materialization failure, element/block
evaluation order, and the UNKNOWN/NIL/ERROR boundaries are recorded through
the collectionShape, generativeCollection, and higherOrder families.

The fifth Phase 3 slice migrates every Core arithmetic Word without changing
the exact-real engine. Lifted binary/unary arithmetic and quantization retain
exact residuals, broadcast errors, NIL passthrough, reasoned undecidability,
and the deliberate MOD-by-zero ERROR versus DIV-by-zero NIL distinction.

The sixth Phase 3 slice begins hosted effects with Core PRINT and migrates the
complete child-runtime lifecycle as one unit. Capability, ordered semantic
effect, hosted-effect request, handle role, lifecycle ERROR boundary, and
supervision exhaustion remain separate fields rather than display inference.

The final Core slice migrates conversion, text, and definition-time staging
contracts. The migration gate derives all 98 Core Words from the frozen
manifest instead of maintaining a second handwritten allow-list. Module Words
remain outside `words.json`, so `migration.completeInventory` stays false and
legacy module metadata must not yet be removed.

The first Module-Word slice migrates all six `DATA` contracts as qualified
canonical names. Table shape, reasoned CSV projection, missing-field behavior,
purity, executor ownership, and authored LOOKUP summaries are checked from the
same differential contracts. The other 90 Module Words remain deferred, so
this slice does not yet authorize Phase 5 metadata deletion.

The second Module-Word slice migrates all ten `JSON` contracts. JSON parsing,
functional object access/update, missing-value projection, and host-mediated
export remain separate contract axes. Eighty Module Words remain deferred, so
the full-inventory and Phase 5 deletion gates remain closed.

The third Module-Word slice migrates the complete `IO` and `CRYPTO` modules.
Host input/output, secure-random observation, deterministic digest computation,
capability, effect, and determinism are represented as independent axes.
Seventy-six Module Words remain deferred, so full-inventory is still false.

The fourth Module-Word slice migrates all four `ALGO` contracts. Stable
deduplication, membership, reasoned search misses, and comparison-budget
UNKNOWN remain distinct from malformed collection use. Seventy-two Module
Words remain deferred.

The fifth Module-Word slice migrates the nine interval and Tier 2 `MATH`
contracts. Exact roots, sound intervals, interpretation roles, reasoned scalar
domain misses, malformed interval use, PI, and water-bounded enclosure remain
separate observable contracts. Sixty-three Module Words remain deferred.

The sixth Module-Word slice completes `MATH` with eight exact arithmetic and
number-theory contracts. NIL passthrough, comparison-budget UNKNOWN, the
zero-to-negative-power projection, integer-domain errors, and exponent work
limits remain distinct. Fifty-five Module Words remain deferred.
