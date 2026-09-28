# Changelog — ajisai-mcp-server

This package is versioned separately from the Ajisai engine it speaks for
(`mcp.serverVersion` and `mcp.engineVersion` on every result). The engine is
still alpha and makes no compatibility promise; this file records changes to
the adapter's own surface — tool list, envelope fields, resources and
descriptions.

## 0.6.0

The four source tools give one answer about one program.

### Changed

- **`infer_contracts` reports malformed source as an error.**
  - Source that does not tokenize or balance its brackets is now `status: "error"` with `aiDiagnostic.category: "malformedSource"`. This is the same report `check` gives.
  - It used to be `status: "ok"` with no contracts, while `compute`, `check` and `outcomes` all called the same source malformed.
  - A caller that took every `infer_contracts` answer as a success must now branch on `status`.
- **A Word whose body calls an undefined name is `partial`.** That call raises `unknownWord`, which the registry marks `repair: program`, so the Word is `partial` by the registry's own rule. Inference used to report it as `total` next to its own `gap.unresolvedWord` gap. (The native CLI's `agent infer-contracts` also exits 1 on malformed source.)

### Added

- **`tool-consistency.test.js`**, part of `npm run selftest`. It runs every golden and evaluation‑corpus source, plus edge cases, through `compute`, `check`, `infer_contracts` and `outcomes`, and fails when their answers contradict each other. It fails on the previous engine at exactly the two cases above.

## 0.5.1

Packaging only; nothing a tool answers changes.

### Changed

- **The package ships `LICENSE`.** `sync-assets.js` copies the repository's MIT licence into the package, and its `--check` (run by `prepack`) fails if the copy drifts.
- **The package holds only what the server runs**, 21 files instead of 37. The evaluation harness (`eval.js`, `benchmark.js`, the scorers and validators), its corpora under `eval/`, the golden cases and `backend/parity-test.js` are no longer published. None of them could run from an installed copy. `npm run test:pack` now fails if a development-only file ships.
- **`ajv` is a development dependency.** Only the self-test imports it; the server does not.
- **Native binary discovery stays inside a checkout.**
  - Before, an installed copy looked at `../../rust/target` relative to itself, which is the installing project's own directory, and ran any `ajisai` binary it found there.
  - Now discovery happens only when the package is an Ajisai checkout's `tools/mcp-server`, or `AJISAI_REPO` names one. An installed copy under `node_modules` always runs the packaged WASM backend unless `AJISAI_BIN` says otherwise.
  - When both a release and a debug build exist, the more recently built one is used. Before, debug always won.
  - `npm run test:pack` plants a decoy binary where the old discovery looked and asserts the installed copy ignores it.

## 0.5.0

Result envelope `schemaVersion` 3 (engine report `SCHEMA_VERSION` 3). The error
vocabulary is the outcome registry's, and each error says it once.

### Changed

- **`aiDiagnostic`** classifies an error and nothing else:
  - `kind` is renamed `category`.
  - `recoverability`, a seven-value scale of the engine's own (`fixInput`, `fixProgram`, `fixHost`, …), is replaced by the registry's `repair`. It is `"program"` exactly when `spec/outcomes.json` marks the category so, and absent otherwise, as in the registry.
  - Its copies of `nextChecks`, `candidates` and `resourceLimit` are gone; they are `diagnosis`'s.
  - `result.schema.json` now types these four fields.
- **One diagnosis per error.** The `wordError` event in `errorFlowTrace` no longer repeats the top-level `diagnosis`. A `nilProduced` event keeps its own, since a NIL has no other. `1 ADD` fell from 12,172 to 7,170 bytes as sent, and `FROBNICATE` from 9,206 to 6,090.
- **`divisionByZero` is no longer an error category.** It is a NIL reason in `spec/outcomes.json` and nothing else. The trace used to report a NIL from `DIV` under an error category of that name as well.
- **`diagnosis.summary`** uses protocol spellings and outcome ids: `executeWord / ADD / stackShape (error:stackUnderflow) …` instead of `ExecuteWord / ADD / StackShape (stackUnderflow) …`, and `(nil:divisionByZero)` in place of `(divisionByZero) nil=DivisionByZero`.
- **`responseBytes`** is enforced on the response as sent: the structured result, its serialized text mirror and provenance together. The backends still refuse early on their single copy. Before this, a result that fit that copy could arrive at more than twice the declared ceiling.

## 0.4.0

The contract surfaces are brought back in line with the engine and with each
other. Nothing here changes a computed value.

### Added

- `compute` results carry a top-level `outcome`: `value`, `nil:<reason>` or
  `error:<category>`, in the `spec/outcomes.json` ids the `outcomes` tool
  predicts. A reasoned absence is now distinguishable from a value without
  reading the top stack node, and a run is checked against its prediction with
  one membership test.
- `outcomes` is documented where the other tools are: the README tool table,
  the quickstart's tool-selection table, `result.schema.json` (`outcome`,
  `outcomes`, `exact`, `limitProfile`) and the evaluation contract's tool set.

### Changed

- **`ajisai://vocabulary`** serves the inventory only — `name`, `kind`,
  `family`, `vocabularyTier` per entry, plus `wordCount`. The repository
  manifest it used to serve verbatim carried retired classification axes
  (`semantic_role`, `algebraic_family`, `core_tier`), coverage ids and source
  paths. Full contracts remain at `ajisai://contracts`.
- **Empty `source`** is the empty program (`status: ok`, `outcome: value`), as
  whitespace-only source already was. Only a missing or non-text `source` is
  `invalidRequest`.
- The `outcomes` description states when `exact` is true as it actually is:
  exactly when one id is returned — an empty program, or source that does not
  tokenize or balance its brackets. A bare literal is `exact: false`.
- The `compute` description names every Word (it said 78 and listed 72), counts
  them from the registry rather than by hand, and says Word names are
  case-insensitive — the engine canonicalizes them to upper case. The
  quickstart preface is corrected the same way, and no longer calls an exact
  `sqrt(2)` display "truncated" in one section and "never truncated" in
  another, nor sends a caller to `ajisai://vocabulary` for contracts.
- Tool `outputSchema` copies omit `$id`, so a client that caches compiled
  validators by `$id` no longer rejects the second tool's schema as a
  duplicate. `ajisai://schema/result` still carries it.
- `npm run selftest` passes on a fresh clone: the word-candidate parity check
  asks whichever backend the server would use instead of requiring a native
  binary.

## 0.3.0

Baseline for the evaluation traces committed under `eval/traces/`.
