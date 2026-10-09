# Ajisai MCP server

Ajisai's MCP surface makes the language useful as a bounded, deterministic and
diagnostic computation kernel for AI agents. The server remains a thin adapter:
the Rust CLI owns language semantics and generated artifacts own vocabulary.

Ajisai promises **exactness in its supported numeric domain**, rather than
unqualified “no rounding errors”. Operations such as explicit rounding and
functions outside that domain retain their documented semantics.

This README is the server's own record of what it does and how it is
verified; the readiness tracker and the hand-off memo that once accompanied
it in `docs/dev/` were retired when their exit criteria were met.
Host-by-host resource ceilings are compared in `docs/dev/mcp-host-profiles.md`.

## Install and connect

Requirements: **Node 20 or newer**. Nothing else — no build step, no `cargo`,
no native binary: the package carries its WASM backend.

The server is published on npm as `ajisai-mcp-server` and listed in the
official MCP Registry as `io.github.masamoto1982/ajisai`. Most MCP clients take
a JSON server entry. Claude Desktop (`claude_desktop_config.json`), Claude Code
(`.mcp.json`) and Cursor (`.cursor/mcp.json`) all use this shape:

```json
{
  "mcpServers": {
    "ajisai": {
      "command": "npx",
      "args": ["-y", "ajisai-mcp-server"]
    }
  }
}
```

`npx -y ajisai-mcp-server --doctor` exits 0 when the installed copy can
actually compute.

### From a checkout

A clone runs the same server without npm, which is how the repository's own
tests and an unreleased engine are reached. The WASM backend is committed under
`wasm/generated/`, so a fresh clone computes immediately.

```sh
git clone https://github.com/masamoto1982/Ajisai.git
cd Ajisai/tools/mcp-server
npm install
node index.js --doctor     # exits 0 when this copy can actually compute
```

The server entry then names the checkout's `index.js`:

```json
{
  "mcpServers": {
    "ajisai": {
      "command": "node",
      "args": ["/path/to/Ajisai/tools/mcp-server/index.js"]
    }
  }
}
```

That is the whole configuration. No `env` block is needed, and pointing
`AJISAI_BIN` at a native binary — as an earlier version of this file did in its
only working example — is an optional override, not a prerequisite:

- `AJISAI_BIN` selects a native `ajisai` binary instead of the packaged WASM
  backend, which is how a Docker image that builds one in should be wired.
- `AJISAI_REPO` is a development-only fallback for discovering a locally built
  binary without naming it.

Without either, a native binary is discovered only when this package is run
from an Ajisai checkout (`tools/mcp-server` beside `rust/Cargo.toml`), and then
the more recently built of `rust/target/release` and `rust/target/debug` is
used. An installed copy under `node_modules` never looks outside itself: it
runs the packaged WASM backend.

The published package holds only what the server runs — the adapter, its two
backends, the packaged assets and WASM module, `README.md`, `CHANGELOG.md` and
`LICENSE`. The evaluation harness, its corpora and the tests stay in the
repository (`npm run test:pack` fails if one ships).

Both backends answer identically (see [Backends and
provenance](#backends-and-provenance)); the override is about deployment, not
about results.

### Checking an installation

The server is silent when it is healthy, which makes it indistinguishable from
one that is wedged. The same executable answers for itself:

```sh
node index.js --version    # adapter version, engine version, registry digest
node index.js --doctor     # Node, assets, backend and two real computations
node index.js --help
```

`--doctor` exits 0 when every check passes and 1 when any fails, so it can gate
a container start or a support request. It computes `2 3 DIV 1 3 DIV ADD` and
`2 SQRT` through the selected backend: a server that starts and loads its
assets but answers wrongly is still broken, and only running something proves
otherwise. With no arguments the process speaks MCP on stdin/stdout and writes
nothing else there.

## Agent surface

| tool | purpose |
|---|---|
| `compute` | execute source with time, source, output and step limits |
| `check` | parse, resolve and conservatively verify `#:contract` declarations without execution |
| `infer_contracts` | infer contracts for user-defined Words without execution |
| `outcomes` | predict, without execution, the finite set of outcome ids a program could produce |
| `word_contract` | query the complete canonical `spec/words.json` contract registry |

Execution tools accept source text only. Deliberately omitting file-path input
prevents an AI tool call from becoming an arbitrary local-file reader.

`check` answers about the *form* of a program, never about running it: it
tokenizes, parses, resolves names and verifies `#:contract` declarations
(`#:contract DOUBLE inputs=1 outputs=1 purity=pure`, one line per Word, in the
keys and values `word_contract` answers in), all without execution. `compute`
runs the same declaration check **before executing anything**, and a
declaration the body contradicts stops the run: both tools answer it as an
ordinary error — `aiDiagnostic.category: contractViolation`, `message`,
`diagnosis` with `when: checkContract` — with every finding in
`contractDecls.findings`, and `compute` names it `outcome:
error:contractViolation`, which `outcomes` predicts exactly when the source
carries a directive. A source without a directive carries no `contractDecls`.
`check` used to answer a violation with `status: error` and nothing else at
the top level, so a reader following "on error, read `diagnosis.why`" found
nothing, and `compute` did not check at all — a program whose declaration
about itself was false ran and answered a value. So a `check` that returns `status: ok` says the program is
well-formed and its names resolve — it does not say the program will succeed,
stay inside a ceiling, or produce a value rather than a NIL. Nothing that only
a run can decide is decided here. Read it as "this will get as far as
executing", and read `compute` for what executing it does.

The four source tools answer one program consistently, and
`tool-consistency.test.js` (part of `npm run selftest`) holds them to it over
every golden and corpus source:

- `compute`'s `outcome` is always in the set `outcomes` predicts.
- Source that does not parse is `malformedSource` to all four — `infer_contracts`
  included — and `outcomes` answers it exactly.
- `check` ok means the run does not fail on the program's form or its names.
- `check` is stricter than a run: it rejects a name nothing defines even inside
  a Word that is never called, which a run never reaches. `outcomes` still
  allows for `unknownWord` there, and `infer_contracts` reports that Word as
  `partial` with a `gap.unresolvedWord` gap.

### Three outcomes, kept distinct

| Ajisai outcome | `status` | `outcome` (compute) | `isError` |
|---|---|---|---|
| a value | `ok` | `value` | — |
| `NIL(reason)` | `ok`, with the absence reason | `nil:<reason>` | — |
| language `ERROR` | `error`, with the full diagnosis | `error:<category>` | — |
| a failure of the *host* | `hostError` | — | yes |

`outcome` uses the ids of `spec/outcomes.json` — the same ids the `outcomes`
tool predicts — so a run is checked against its prediction with one membership
test, and a reasoned absence is told apart from a value without reading the top
stack node. `outcomes` answers `exact: true` only when it returns a single id:
an empty program (`value`) or source that does not tokenize or balance its
brackets (`error:malformedSource`). Every other program, a bare literal
included, gets a sound superset with `exact: false`.

All five tools answer with the same envelope (`result.schema.json`, also served
as `ajisai://schema/result`), so one schema describes every result a caller can
receive and there is no second contract to keep in step.

### What a result costs

Every result arrives twice: as `structuredContent`, which a caller branches on,
and serialized into a text content block, because MCP asks a tool with an
output schema to also return the serialized JSON that way — a text-only client
has no other route to any of it. That mirror stays. Replacing it with a prose
summary is the one compaction that would actually lose information, and the
self-test pins that a text-only client can still tell a value, a
reason-carrying NIL, a language error and a host failure apart from the text
alone.

What was removed is everything constant. `1 2 ADD` answered 3,732 bytes, of
which the answer — `stack` and `stackDisplay` — was 85: the rest was the
twelve-entry limit table on every result (`mcp.limits`), the same table again
inside the engine's `receipt.limitProfile`, the receipt's copies of
`observationDigest` and `resourceUsage`, and eight optimizer counters
(`runtimeMetrics`) that describe how the engine went about its work and tell a
caller nothing. Five calls in one turn cost 5,000 tokens of that. Now:

- `mcp.limitProfile` names the profile (`mcp-local-stdio`); the ceilings are
  the `ajisai://limits` resource, read once. A ceiling that fires still names
  itself and its value in `diagnosis.resourceLimit`, and a host failure about
  one in `error.limit`, so no result needs the table to be acted on.
- `runtimeMetrics` is not sent. `resourceUsage` — the budget side, what the
  run spent against each ceiling — is.
- `receipt` keeps `digest`, `sourceDigest` and `registryDigest`, the three
  things nothing else in the envelope carries; a verifier takes `status`,
  `observationDigest`, `resourceUsage` and `mcp.engineVersion` from beside
  them and the limit profile from `ajisai://limits`. The native CLI's own
  envelope is untouched — this is the adapter's presentation of it.
- An optional field carrying no value is absent, not `null` — `message`,
  `diagnosis` and `aiDiagnostic` on a plain success, `contractDecls` on a
  source that declares nothing. **Test for presence, not for `null`.**

`1 2 ADD` is 1.8 KB as sent, and what remains constant is the three digests a receipt and its provenance need. `npm run eval:performance` measures the whole
response and the text block alone over the seven benchmark cases and fails
against a committed `medianResponseBytesBudget`, so none of it can come back
unnoticed. The budget is a ceiling to lower when a response genuinely shrinks,
never one to raise so a regression passes.

A host failure is machine-readable: `error.code` is a stable identifier
(`invalidRequest`, `unknownTool`, `sourceTooLarge`, `backendUnavailable`,
`capacityExhausted`, `timeout`, `responseTooLarge`, `malformedBackendResponse`,
`backendFailure`, `registryUnavailable`), `error.retryable` says whether trying
again can help, and `error.limit` names the declared ceiling when the failure is
about one. `error.message` is written for a model and carries no host paths,
environment-variable names or spawn diagnostics; that detail goes to the
server's stderr, where an operator is looking.

Saturating `concurrentExecutions` queues the caller for up to a second before
answering `capacityExhausted`, so an ordinary burst becomes back-pressure
rather than a retry loop the caller has to write.

### Reading an algebraic value

`2 SQRT` answers with the value, a rendering of it, and an approximation.
On the stack node, read:

| field | what it is |
|---|---|
| `stackDisplay` | the value written as one token: `"sqrt(2)"`, `"2/1*sqrt(2)"`, `"sqrt(2)-sqrt(3)"`, exact and never truncated |
| `semantics.exactTerms` | the value itself: `Σ (numerator/denominator)·√radicand`, arbitrary-precision integers as strings |

The display renders exactly the terms beside it. Compute with `exactTerms`;
the display is meant to be read rather than parsed.

The one that misleads is the node's own `value`: a rational approximation
flagged `semantics.approximate`, so it looks exact and is not.

The display writes the canonical normal form, so equal values are written
the same way: `8 SQRT` and `2 SQRT 2 SQRT ADD` both give `2/1*sqrt(2)`, and
`EQ` decides they are the same number. Comparison decides equality here;
string comparison does not — the string is display text, not a value. `exactTerms` does not appear on a rational
or a vector of rationals, whose `stackDisplay` is already the whole value.

### Diagnostics

An unknown Word answers with `diagnosis.candidates` — the closest known names,
best match first, drawn from the compiled-in vocabulary and the Words the same
source defines (each call runs in a fresh session, so the same source is the
whole User dictionary). `word_contract` answers an
unmatched name the same way, in `suggestions`.

Each `nextChecks` entry is `{ code, title: { en, ja }, detail: { en, ja } }`.
Match on `code`; the display text is localized and free to be reworded.

`diagnosis` is the one copy of an error's diagnosis. `aiDiagnostic` only
classifies it — `category` (the `spec/outcomes.json` error category, the same
id as in `outcome`), `repair: "program"` when the registry says the program is
what to change (absent: an operand is wrong), `word` and `family` — and the
error's `errorFlowTrace` event does not repeat it. A `nilProduced` event keeps
its own diagnosis, since a NIL has no other; it is recorded once, at the Word
that projected the NIL, and the Words it then passed through record nothing. Repeating the diagnosis three times
is what made `1 ADD` a 12 KB response; it is now 7 KB.

`responseBytes` bounds the response as sent: the structured result, its
serialized text mirror and provenance together. A *successful* result is kept
under it by the engine rather than refused: under the agent profile a stack is
sent whole up to 440 KiB (a 5,000-element vector of small integers), and past
that the slot that does not fit is elided the same way a failing stack's is —
`value: null`, an `elided` record with `elements` and `approxBytes`, a
matching `<elided …>` marker in `stackDisplay`, and `stackElided.reason:
valueStackBudget` at the top level — while every value beside it arrives
whole. `0 7000 RANGE 1 2 ADD` used to be `responseTooLarge`, which told the
caller its answer was too big and nothing else; it now answers `3/1` and a
record saying a 7,001-element vector was left under it, which is what tells
the caller to drop it. `responseTooLarge` remains the hard gate behind that,
for a result no elision can bring under the ceiling.

A resource-limit failure carries `diagnosis.resourceLimit`
(`{ resource, limit, observed }`), where `resource` is the name of the very
entry in `ajisai://limits` that fired and `limit` its value — every ceiling,
`executionSteps` included, reports what it observed.

A ceiling can refuse a call without failing it. A well-formed generative Word
whose result will not fit — `0 100001 RANGE` against
`materializedElements` — *projects* to NIL under the NIL Projection Rule, so
the call is `status: ok` and there is no top-level `diagnosis` to carry
anything. The same facts are on the value that came back instead:
`stack[i].semantics.absence.diagnosis.resourceLimit`, and on the
`nilProduced` entry of `errorFlowTrace`. Read them there when
`absence.reason` is `spaceExhausted`; `reason` says a ceiling fired, and only
these say which one, what it is set to, and what size crossed it.

### Backends and provenance

All execution tools call the same host-neutral Rust agent boundary
(`rust/src/agent`) through one of two interchangeable backends
(`tools/mcp-server/backend/`): a native `ajisai` subprocess per call, or the
same agent code compiled to WASM and run in a fresh WebAssembly instance per
call, inside a reused `worker_threads` Worker. Both return the identical result envelope — verified case by case in
`backend/parity-test.js` — so Node never reinterprets command-specific results.

The backend is chosen **once, at startup**, and named in `mcp.backend.kind`
(`nativeCli` or `wasmWorker`). Choosing per request meant a `cargo build`
finishing mid-session silently moved later calls onto a different execution
path, with nothing in the response saying so. Parity is what makes the two
answers equal; provenance is what would make an unequal one investigable.

Every result also carries `mcp.serverVersion`, `mcp.engineVersion`,
`mcp.assetDigest` and the name of the applied profile, `mcp.limitProfile`. The two versions are two
separately released components: `serverVersion` is this Node adapter, and
`engineVersion` is the Ajisai language it speaks for. A saved result used to
name only the second, so a field missing from an archived envelope could not be
told apart from a field that adapter version never sent.

The packaged registry is verified against its recorded digest at **startup**,
so a corrupt asset stops the server rather than surfacing as a generic failure
on whichever request touched it first.

## Limits

The profile is the `ajisai://limits` resource, named on every result as
`mcp.limitProfile`. Every entry has a matching entry in `golden/limits.json`,
and the self-test fails if the two sets differ — a ceiling cannot be declared
without saying how it is exercised. Five are pinned by real boundary sources
run against the live server on every self-test and compared across both
backends; `concurrentExecutions`, `wallTimeMs` and `responseBytes` are pinned
through the adapter's own admission and delivery paths; and `numericWork`,
`bigintBits` and `algebraicTerms` are pinned in Rust with injected ceilings
because they are not reachable within `wallTimeMs` at their declared values. `golden/limits.json`
and `docs/dev/mcp-host-profiles.md` say so explicitly rather than leaving the
gap to be discovered.

The playground applies a different, looser profile — `0 100001 RANGE`
succeeds there and answers `NIL(spaceExhausted)` here, and its step budget is
120 times this one's, so a block iteration that walks 100,000 elements there
is refused here (`executionSteps`; the quickstart says what to write
instead). Both hosts publish what they apply — the playground's splash shows
this profile's ceilings beside its own — and the divergence is recorded as an
explicit `hostDivergence` block on the golden case that shows it.

## Resources

`ajisai://guide/quickstart`, `ajisai://vocabulary`, `ajisai://contracts`,
`ajisai://schema/result` and `ajisai://limits`. `ajisai://contracts` is every
Word's full contract in one read; `ajisai://vocabulary` is the inventory only —
each name with its `kind`, `family` and `vocabularyTier`. The
`ajisai://words/{name}` template exposes the same complete Word contract as
`word_contract` without a tool call. Word names are case-insensitive, in
lookups as in programs (`add` runs as `ADD`); the registry digest is calculated from
the canonical specification, not from a reduced documentation manifest.

`ajisai://guide/quickstart` is an MCP preface (`mcp-quickstart.md`) followed by
the generated writing protocol (`SKILL.md`), joined by `sync-assets.js`.
Reading `ajisai://words/{name}` with a name that is not a valid percent-encoded
string is refused as an invalid request, and a tool call carrying an argument
its schema does not declare is refused as `invalidRequest` naming the
argument — `additionalProperties: false` is enforced by the server, not only
advertised. The
guide used to be `SKILL.md` alone, which opens on a CLI run loop — `ajisai run
file --json`, commands a connected client cannot issue — and never says which
of the tools to call, so a model that read it first learned the language
before it learned the interface. The preface answers tool selection, result
branching and the algebraic-value trap in one screen, then hands off. Its own
examples are executed against the live backend by the self-test, the same
guarantee the generator gives the half below it.

All five tools declare read-only, non-destructive and idempotent MCP
annotations.

## Development

```sh
cd tools/mcp-server
npm install
npm run selftest       # uses the packaged WASM backend unless AJISAI_BIN is set
npm run test:pack
npm run eval:validate
npm run eval:performance
```

The packaged WASM bundle (`wasm/generated/`) is regenerated by
`npm run build:mcp-wasm` at the repo root. Vocabulary, contracts, guide,
version and registry provenance are packaged under `assets/` regardless of
backend.

`npm run test:mcp-backends` at the repo root (builds the native binary, then
runs `backend/parity-test.js`) runs every golden case and every declared limit
boundary against both backends and asserts they agree.

How the server is evaluated — the bilingual agent corpus, the trace and
repair scorers, the captured model baselines, the performance and
response-size budgets — is its own document: `docs/dev/mcp-evaluation.md`.
This README is for connecting to the server and reading what it answers.

`npm run test:pack` creates the allowlisted tarball and installs it into an
empty temporary prefix, then exercises that installed copy four ways: importing
`createServer` with neither `AJISAI_REPO` nor `AJISAI_BIN` set, proving it
computes through its packaged, self-contained WASM backend with no repository
and no native binary in reach; **launching the `ajisai-mcp-server` bin through
`node_modules/.bin`**, which is how every documented client entry starts it;
running `--doctor` on the installed package; and finally spawning that bin
again with an explicit `AJISAI_BIN`, asserting `mcp.backend.kind` is
`nativeCli`.

The last two of those are spawned processes on purpose. The backend is resolved
once per process, so setting `AJISAI_BIN` and constructing a second server in
the same process reused the WASM backend the first scenario had already fixed —
the native assertion passed on machines with no native binary at all. Because
it is now real, `npm run test:pack` needs one built; `npm run test:mcp-pack`
from the repository root builds it first.

The browser playground is independent of this package and remains available.
