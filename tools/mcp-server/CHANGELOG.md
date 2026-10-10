# Changelog — ajisai-mcp-server

This package is versioned separately from the Ajisai engine it speaks for
(`mcp.serverVersion` and `mcp.engineVersion` on every result). The engine is
beta, and promises compatibility from 1.0.0; this file records changes to
the adapter's own surface — tool list, envelope fields, resources and
descriptions.

## Unreleased

An engine rule changes and the goldens, Word summaries, quickstart and README this server serves say so; `result.schema.json` only reworded a description. No tool, envelope field or resource changes.

### Changed

- **A host ceiling is never a value.** A generative Word asked for a result past a ceiling — `RANGE`, `FILL`, `RESHAPE` past `materializedElements`, a `FILL`/`RESHAPE` shape or `JSON-DECODE` text past `nestingDepth`, `NUM` or `JSON-DECODE` past `numericLiteralDigits`, `POW` past `bigintBits` or `algebraicTerms` — now fails as `resourceLimitExceeded`, naming the ceiling, its value and the size requested in `diagnosis.resourceLimit`, with the Word's operands left on the stack. It used to project `NIL(spaceExhausted)` and answer `status: ok`; that NIL flowed on like any other value, so `0 100001 RANGE LENGTH` answered `100001` in the playground and `NIL` here, and `0 X X NIL? SELECT` chose a different branch per host without any failure to notice. The NIL reason `spaceExhausted` no longer exists; `outcomes` predicts `error:resourceLimitExceeded` where it predicted `nil:spaceExhausted`.
- **`0/0` absorbs `POW` in either operand.** `0/0 0 POW` and `2 0/0 POW` are `0/0`, as LANG.VALUES.EXACT's "0/0 absorbs every operation" says; `0/0 0 POW` used to answer `1`, and `2 0/0 POW` a `domainMiss`. `1/0 0 POW` stays `1`, the empty product. `1/0` and `-1/0` as exponents still project `domainMiss`.
- **The quickstart's step budget is stated per Word.** A block iteration costs one step per element for every Word the block runs, so a two-Word block walks 50,000 elements under the 100,000-step budget, not "tens of thousands" for any block. `CHARS` says it splits by code point, not grapheme cluster.

## 0.8.2

An engine rule changes and the Word summaries, quickstart and goldens this server serves say so; no tool, envelope field or resource changes.

### Changed

- **Division is total.** A number is a reduced pair over a non-negative denominator, and a quotient by zero is the dividend's sign over zero — one of the three numbers `1/0`, `-1/0`, `0/0` — not a reasoned NIL. `1 0 DIV` is `1/0` and `outcomes` predicts `value` for it; `[ 6 6 6 ] [ 1 2 0 ] DIV` is `[ 6/1 3/1 1/0 ]`; `0/0` has no place in the order, so `0 0 DIV 1 LT` projects `nil:domainMiss`. The number node's `denominator` may now be `"0"` (`host-protocol.schema.json`: `nonNegativeIntegerString`). The NIL reason `divisionByZero` no longer exists; the quickstart's absence examples use `-1 SQRT`.

## 0.8.1

An engine rule changes and the Word summaries this server serves say so; no tool, envelope field or resource changes.

### Changed

- **A higher-order block leaves exactly one value.** `MAP`, `FILTER`, `FOLD` and `SCAN` used to take the top of what their block left and discard the rest, so `[ 1 2 3 ] [ 1 GT TRUE ] FILTER` kept every element on the strength of the `TRUE` written above the comparison, and `[ 1 2 3 ] [ 2 MUL 7 ] MAP` answered `[ 7 7 7 ]` — quiet wrong answers. A block that leaves a surplus is now the same `blockContractViolation` a block that leaves nothing has always been (`MAP: expected the block to leave one value, and it left 2`), and the run stops. LANG.COLLECTIONS.HIGHER, the four Words' summaries (`assets/words.json`, `word_contract`, the quickstart's Word table), the Reference and the conformance corpus state the rule the same way; the quickstart's generated half gains a §7 entry for it. A value computed along the way is consumed before the block ends or named with `BIND`, which leaves nothing. The packaged WASM engine is rebuilt with the rule.

## 0.8.0

What a result carries changes, so this is a minor version: constant metadata leaves the envelope, a successful result too large to send is elided rather than refused, and `compute` checks `#:contract` declarations before it runs anything. The engine it speaks for is unchanged in version.

### Changed

- **A result names its profile instead of carrying it.** `mcp.limits` — the twelve-entry ceiling table, identical on every result — is replaced by `mcp.limitProfile: "mcp-local-stdio"`; the table is the `ajisai://limits` resource. A ceiling that fires still names itself and its value in `diagnosis.resourceLimit` (now for `executionSteps` too, whose `observed` was `null`), and a host failure about one in `error.limit`. `runtimeMetrics` (the engine's optimizer counters) is no longer sent; `resourceUsage` is. `receipt` keeps `digest`, `sourceDigest` and `registryDigest` and drops its copies of `limitProfile`, `observationDigest`, `resourceUsage`, `engineVersion` and `outcomeStatus`, all of which are beside it or in `ajisai://limits`. `1 2 ADD` goes from 3,732 bytes as sent to 1,782; `medianResponseBytesBudget` is lowered to match.
- **An oversized success is elided, not refused.** Under the agent profile the engine sends a stack whole up to 440 KiB and past that replaces the slot that does not fit the way a failing stack's is replaced — `value: null`, an `elided` record (`elements`, `approxBytes`), a `<elided …>` marker in `stackDisplay` and `stackElided.reason: valueStackBudget` — while every value beside it arrives whole. `0 7000 RANGE 1 2 ADD`, previously `hostError: responseTooLarge`, now answers `3/1` and a record of the 7,001-element vector left under it. `responseBytes` stays the hard gate and is exercised through the backend's own budget; its golden coverage moves from `boundary` to `hostGate`. The byte estimate in an `elided` record is calibrated to the real rendering (`0 99999 RANGE` was reported at ~33 MB for a stack of 8.6 MB; it now reads 9.0 MB).
- **`compute` checks `#:contract` declarations before anything runs.** A declaration the body contradicts stops the run: `status: error`, `outcome: error:contractViolation`, nothing executed and nothing printed. Both `check` and `compute` answer a violation in the shape every other error has — `message`, `diagnosis` (`when: checkContract`, `why: contractViolation`, `where.word`), `aiDiagnostic.category: contractViolation` and a `checkDeclaredContract` next-check — with every finding still in `contractDecls.findings`; `check` used to answer `status: error` and nothing else at the top level. A verified declaration is reported too (`contractDecls.outcome: value`); a source with no directive carries no `contractDecls`. `outcomes` predicts `error:contractViolation` exactly when a directive is present. `contractViolation` is a new structural category of `spec/outcomes.json`.
- **The quickstart documents `#:contract`.** The directive's grammar, a verified example and a refused one (§8), what a block iteration costs against the 100,000-step budget and what to write past tens of thousands of elements (§6), and `stackElided` (§2). The generated protocol below it writes scalars bare (`42`, `1 3 DIV`, `[ 1 2 3 ] 0 [ ADD ] FOLD`) where it used to write `[ 42 ]`, shows a Record as the engine displays it (`[ 'x' 'y' ] [ 1/1 2/1 ] RECORD`), and gains §2a on declarations and a §7 entry for a violated one.
- **New next-checks.** `DUP` / `SWAP` / `OVER` / `ROT` answer `checkNoStackShufflers`; `IF` / `ELSE` / `WHILE` / `FOR` answer `checkNoControlKeywords`; a name with full-width characters (`ＡＤＤ`, `１`) answers `checkCharacterWidth`, and full-width names are folded before "did you mean" ranks them, so `ＬＥＮＧＨＴ` suggests `LENGTH` (`word_contract` folds the same way).
- **Requests are validated against their declared schema.** An argument no tool declares is refused as `invalidRequest` naming it, where it used to pass silently; `ajisai://words/{name}` with a malformed percent-encoding is refused as an invalid request instead of surfacing a `URIError`.
- **The README is for connecting and using.** The evaluation harness — corpus, scorers, baselines, budgets — moved to `docs/dev/mcp-evaluation.md`.

## 0.7.2

Documentation and engine speed: what the server says about `DIV` is extended, and vector programs that meet a zero divisor or an absent lane run faster on the packaged engine. No tool, envelope field or computed answer changes.

### Changed

- **A zero divisor empties its own lane and no other.** `DIV`'s summary in `assets/words.json` (what `word_contract` answers and the quickstart's Word table shows) says so with the worked example `[ 1 2 3 ] [ 1 0 2 ] DIV` is `[ 1 NIL 3/2 ]`: the lanes beside the absent one are quotients, a Word after it passes the absent lane through while computing the rest (LANG.FAILURE.PASSTHROUGH), and `1 0 DIV 1 ADD NIL-REASON` is `'divisionByZero'`.
- **Engine: a zero divisor or an absent lane no longer costs a vector its columns.** The packaged WASM backend's column kernels project a zero divisor and carry an absent lane in place, the reason beside it, where they used to hand the whole operation to a route that boxed every lane and left the result boxed for every Word after it. Answers are unchanged. On a million-lane vector with one zero divisor, `DIV` goes from 237 ms to 45 ms and the five Words after it (`1 ADD 2 MUL 3 DIV FLOOR 0 GT`) from 664 ms to 100 ms (`scripts/bench/speed-bench-wasm.mjs`, its two `lane` cases, the previous bundle against this one).

## 0.7.1

Documentation only: what the server serves about the language is corrected, and no tool, envelope field or computed answer changes.

### Changed

- **Record keys.** The quickstart resource, the Word summaries (`assets/words.json`) and the specification say which keys `GET`, `PUT`, `HAS?` and `WITHOUT` address: their key operand is a leaf, so a key that is itself a Vector, a Record or NIL — which `RECORD`, `TALLY` and `GROUP` accept — is read through `KEYS` and `VALUES` (`R VALUES R KEYS k INDEX-OF GET`). `GROUP`'s summary no longer promises that `R 'a' GET` reads every group.
- **A leaf includes a Symbol.** The leaf role is any value that is not a container, Symbol included, which is what every leaf Word already did; `STR`'s summary now says it writes a Symbol as its bare name (`[ ADD ] 0 GET STR` is `'ADD'`), as a String that is not the Symbol.
- **Publishing waits for npm.** The release workflow waits until npm answers for the new version before registering it in the MCP Registry, which refused 0.7.0's first registration because npm had not yet served it.

## 0.7.0

The first published release, speaking for the Ajisai 1.0.0-beta.1 engine.

### Added

- **Published on npm and in the MCP Registry.** `npx -y ajisai-mcp-server` runs the server; `server.json` is its official MCP Registry entry, `io.github.masamoto1982/ajisai`, and `package.json` carries the matching `mcpName` the registry verifies ownership by. `sync-assets.js --check` (run by `prepack`) fails when the two files disagree on the name, the version or the npm package.

### Changed

- **Engine 1.0.0-beta.1.** `mcp.engineVersion` reads `1.0.0-beta.1`. The engine is beta: a breaking change to the vocabulary, to program meaning or to the host protocol now raises the specification version and is named in the release that ships it.
- **Every stack node carries `semantics`.** A Vector nested in a Vector, when the engine stored it densely, rendered its rows without the bag, while the same rows stored otherwise carried an empty one. Storage no longer decides the shape; the result schema's `protocolSemantics` says the bag is present on every node.
- **Engine: the diagnosis vocabulary is what the engine emits.** Phase, locus and cause values that no diagnosis could carry (`parseStructure`, `nilPropagation`, `optimizerMismatch` and the like) are gone from the engine, along with the next checks written for them.

## 0.6.1

The diagnosis locus is typed, and the tool text says that DEF does not
outlive its call.

### Changed

- **`result.schema.json` types `diagnosis.where`.** It was an untyped object; it is `{ kind: "coreWord" | "userWord" | "unknown", word }`, the shape the engine has emitted since its `dictionary` field went. `userWord` names a Word the live dictionary holds; `unknown` a name that resolved to nothing, which is the one the `candidates` are spelled against.
- **The `compute` description says each call runs in a fresh session with no User Words**, so a `DEF` lasts for that source only. The README's account of `diagnosis.candidates` says the same, in place of "the live dictionary".
- **Engine (0.2.0-alpha.1 at a later commit): a definition is kept as its source.** A body built from a computed Vector that carries a Record or an exact irrational whole is written back by `DEF` as the source that builds it, so the definition a `check` or `infer_contracts` sees is one text; a body carrying a NIL with a reason, which no source denotes, is `error:invalidDefinitionBody`.
- **Engine: `wordNotFound` no longer offers candidates** — `'FOO' DEL` used to suggest `DEF`, the spelling of `DEL` against the vocabulary — and the "check the user Word's definition and the dictionary it belongs to" next check reads "check that the User Word is defined (DEF) and spelled as defined". `DEL` no longer declares `invalidName`, which it never raised.

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
