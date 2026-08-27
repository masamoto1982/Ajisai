# Ajisai MCP server

Ajisai's MCP surface makes the language useful as a bounded, deterministic and
diagnostic computation kernel for AI agents. The server remains a thin adapter:
the Rust CLI owns language semantics and generated artifacts own vocabulary.

Ajisai promises **exactness in its supported numeric domain**, rather than
unqualified “no rounding errors”. Operations such as explicit rounding and
functions outside that domain retain their documented semantics.

Development status and next-agent instructions are tracked in
`docs/dev/mcp-readiness.md` and `docs/dev/mcp-claude-code-handoff.md`.
Host-by-host resource ceilings are compared in `docs/dev/mcp-host-profiles.md`.

## Install and connect

> **Not on npm yet.** `ajisai-mcp-server` is unpublished, so `npm install -g
> ajisai-mcp-server` and `npx -y ajisai-mcp-server` do not resolve. Publishing
> is a release decision, not a missing feature; until it is made, connect from
> a checkout as below. This section will lead with the registry recipe on the
> day `npm view ajisai-mcp-server version` answers.

Requirements: **Node 20 or newer**, and a checkout of this repository. Nothing
else — no build step, no `cargo`, no native binary. The WASM backend is
committed under `wasm/generated/`, so a fresh clone computes immediately.

```sh
git clone https://github.com/masamoto1982/Ajisai.git
cd Ajisai/tools/mcp-server
npm install
node index.js --doctor     # exits 0 when this copy can actually compute
```

Most MCP clients take a JSON server entry. Claude Desktop
(`claude_desktop_config.json`), Claude Code (`.mcp.json`) and Cursor
(`.cursor/mcp.json`) all use this shape:

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
a container start or a support request. It computes `2 3 / 1 3 / +` and
`2 SQRT` through the selected backend: a server that starts and loads its
assets but answers wrongly is still broken, and only running something proves
otherwise. With no arguments the process speaks MCP on stdin/stdout and writes
nothing else there.

## Agent surface

| tool | purpose |
|---|---|
| `compute` | execute source with time, source, output and step limits |
| `check` | parse, resolve and conservatively verify declared contracts without execution |
| `infer_contracts` | infer contracts for user-defined Words without execution |
| `word_contract` | query the complete canonical `spec/words.json` contract registry |

Execution tools accept source text only. Deliberately omitting file-path input
prevents an AI tool call from becoming an arbitrary local-file reader.

### Three outcomes, kept distinct

| Ajisai outcome | `status` | `isError` |
|---|---|---|
| a value | `ok` | — |
| `NIL(reason)` | `ok`, with the absence reason | — |
| language `ERROR` | `error`, with the full diagnosis | — |
| a failure of the *host* | `hostError` | yes |

All four tools answer with the same envelope (`result.schema.json`, also served
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

What was removed instead is padding. The text used to be written with
two-space indentation, which cost about a third of it and told a machine
nothing, and an optional field carrying no value used to be sent as `null` —
so a plain success advertised `message`, `diagnosis`, `aiDiagnostic` and
`contractDecls`, all empty. Those fields are now absent. **Test for presence,
not for `null`.** Nested fields are untouched: one stated rule at the top level
is worth more than the ~4% recursing would add.

Measured across the seven benchmark cases, the text block's median fell 32%
(1,618 → 1,094 bytes) and the whole response's 22% (3,049 → 2,376).
`npm run eval:performance` reports both and fails against a committed
`medianResponseBytesBudget`, so the padding cannot come back unnoticed. The
budget is a ceiling to lower when a response genuinely shrinks, never one to
raise so a regression passes.

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

`2 SQRT` answers with four renderings of one number, and two of them mislead.
On the stack node, read either of:

| field | what it is |
|---|---|
| `semantics.exactDisplay` | the value written short: `"sqrt(2)"`, `"2/1*sqrt(2)"`, `"sqrt(2) - sqrt(3)"` |
| `semantics.exactTerms` | the value itself: `Σ (numerator/denominator)·√radicand`, arbitrary-precision integers as strings |

They are one fact in two shapes, derived from the same extraction, and the
result schema requires each whenever the other is present — so a reader never
has to decide which to believe. Compute with `exactTerms`; `exactDisplay` is a
display, meant to be read rather than parsed.

The two that mislead are the ones a consumer meets first. `stackDisplay` is the
SPEC §4.2.3 continued fraction **truncated at a display budget** — √2 runs to
`( 1; 2, 2, … )`, ~101 characters, ending in the truncation marker `…`, so it
looks complete and is not — and the
node's own `value` is a rational approximation flagged `semantics.approximate`,
so it looks exact and is not. Neither field changed; `exactDisplay` is what
makes reading them unnecessary.

`exactDisplay` renders the stored normal form faithfully, which means equal
values can be written differently: `8 SQRT` gives `"sqrt(8)"` and
`2 SQRT 2 SQRT +` gives `"2/1*sqrt(2)"`, and `=` decides they are the same
number. Reducing the display would only move the discrepancy, by making the
string disagree with the `exactTerms` beside it. Comparison decides equality
here; string comparison does not. Neither field appears on a rational or a
vector of rationals, whose `stackDisplay` is already exact.

### Diagnostics

An unknown Word answers with `diagnosis.candidates` — the closest known names,
best match first, drawn from the compiled-in vocabulary, the live dictionary
and (for `check`) the Words the same source defines. `word_contract` answers an
unmatched name the same way, in `suggestions`.

Each `nextChecks` entry is `{ code, title: { en, ja }, detail: { en, ja } }`.
Match on `code`; the display text is localized and free to be reworded.

A resource-limit failure carries `diagnosis.resourceLimit`
(`{ resource, limit, observed }`), where `resource` is the name of the very
entry in `mcp.limits` that fired.

### Backends and provenance

All execution tools call the same host-neutral Rust agent boundary
(`rust/src/agent`) through one of two interchangeable backends
(`tools/mcp-server/backend/`): a native `ajisai` subprocess per call, or the
same agent code compiled to WASM and run inside a `worker_threads` Worker per
call. Both return the identical schema-1 envelope — verified case by case in
`backend/parity-test.js` — so Node never reinterprets command-specific results.

The backend is chosen **once, at startup**, and named in `mcp.backend.kind`
(`nativeCli` or `wasmWorker`). Choosing per request meant a `cargo build`
finishing mid-session silently moved later calls onto a different execution
path, with nothing in the response saying so. Parity is what makes the two
answers equal; provenance is what would make an unequal one investigable.

Every result also carries `mcp.serverVersion`, `mcp.engineVersion`,
`mcp.registryDigest` and the applied `mcp.limits`. The two versions are two
separately released components: `serverVersion` is this Node adapter, and
`engineVersion` is the Ajisai language it speaks for. A saved result used to
name only the second, so a field missing from an archived envelope could not be
told apart from a field that adapter version never sent.

The packaged registry is verified against its recorded digest at **startup**,
so a corrupt asset stops the server rather than surfacing as a generic failure
on whichever request touched it first.

## Limits

`mcp.limits` is also served as the `ajisai://limits` resource. Every entry has
a matching entry in `golden/limits.json`, and the self-test fails if the two
sets differ — a ceiling cannot be declared without saying how it is exercised.
Six are pinned by real boundary sources run against the live server on every
self-test and compared across both backends; `concurrentExecutions` is pinned
through the adapter's own admission path; and `numericWork`, `bigintBits` and
`algebraicTerms` are pinned in Rust with injected ceilings because they are not
reachable within `wallTimeMs` at their declared values. `golden/limits.json`
and `docs/dev/mcp-host-profiles.md` say so explicitly rather than leaving the
gap to be discovered.

The playground applies a different, looser profile — `[ 0 100001 ] RANGE`
succeeds there and answers `NIL(spaceExhausted)` here. Both hosts now publish
what they apply, and the divergence is recorded as an explicit
`hostDivergence` block on the golden case that shows it.

## Resources

`ajisai://guide/quickstart`, `ajisai://vocabulary`, `ajisai://schema/result`
and `ajisai://limits`. The `ajisai://words/{name}` template exposes the same
complete Word contract as `word_contract` without a tool call. Contract lookups
accept canonical names and aliases; their registry digest is calculated from
the canonical specification, not from a reduced documentation manifest.

`ajisai://guide/quickstart` is an MCP preface (`mcp-quickstart.md`) followed by
the generated writing protocol (`SKILL.md`), joined by `sync-assets.js`. The
guide used to be `SKILL.md` alone, which opens on a CLI run loop — `ajisai run
file --json`, commands a connected client cannot issue — and never says which
of the four tools to call, so a model that read it first learned the language
before it learned the interface. The preface answers tool selection, result
branching and the algebraic-value trap in one screen, then hands off. Its own
examples are executed against the live backend by the self-test, the same
guarantee the generator gives the half below it.

All four tools declare read-only, non-destructive and idempotent MCP
annotations.

## Development

```sh
cd tools/mcp-server
npm install
npm run selftest       # uses the packaged WASM backend unless AJISAI_BIN is set
npm run test:pack
npm run eval
npm run eval:validate
npm run eval:performance
npm run eval:number-baseline
npm run eval:traces
npm run eval:repairs
npm run eval:reference-traces   # regenerate the fixture after editing the corpus
```

The packaged WASM bundle (`wasm/generated/`) is regenerated by
`npm run build:mcp-wasm` at the repo root. Vocabulary, contracts, guide,
version and registry provenance are packaged under `assets/` regardless of
backend.

`npm run test:mcp-backends` at the repo root (builds the native binary, then
runs `backend/parity-test.js`) runs every golden case and every declared limit
boundary against both backends and asserts they agree.

`eval/cases.json` is the agent-evaluation corpus: 79 cases (59 positive, 20
negative), each asked in English and Japanese, so 158 prompts. `npm run eval` executes every case's
expected tool call against the real backend. It measures backend semantic
correctness only; model tool selection and source generation require captured
model traces and are not claimed by this score.

Every case is bilingual because Ajisai is a Japanese-authored language with an
English tool surface, so "does a Japanese prompt reach the same tool with the
same source as its English twin" is a product question rather than a
translation detail. Both halves of a pair name the same task and therefore
share one expected tool and one expected result, which is what makes the
difference between their scores attributable to the language and nothing else.
The contract rejects a pair whose two sides are the same string: a copied
prompt still scores twice, and would report a comparison it never made.

`score-traces.js` accepts captured model traces in the documented reference
shape and reports tool-selection accuracy, first-attempt generation rate,
end-to-end semantic success, missing traces and irrelevant-tool rate — overall,
per language, and as a `languageGap` between the two. Selection and generation
are separate numbers because they have different repairs: a model that reaches
for the wrong tool with correct source has a tool problem, and one that reaches
correctly and writes source computing the wrong thing has a language problem.
Generation is rated over the positive cases only, since a case whose correct
answer is no call has nothing to generate. `positiveSelectionAccuracy` is there
for the same reason from the other side: `toolSelectionAccuracy` mixes the two
classes, so growing the negative set moves it without any behaviour changing.
Each score carries a `composition` block naming how many cases of each class it
was computed over — rates over one class survive a corpus that grows, rates over
both only compare within one composition.

A turn may hold several tool calls, and all of them are recorded and scored. A
model that looks a Word up and then computes has made one attempt containing two
calls, not a wrong choice — 91 of 130 turns in the first baseline did exactly
that, so keeping only the first call scored the lookup as the model's decision
and reported 0.323 selection accuracy where reading the whole turn reports
0.469. `reachedExpectedToolFirstRate` reports the stricter reading beside it,
without making instinct a pass criterion.

`eval/reference-traces.json` is generated from the corpus by
`npm run eval:reference-traces` and drift-checked in `eval:validate`. A perfect
fixture is the corpus answering itself with its own reference arguments, so
maintaining 130 of them by hand only meant that adding a case failed
`--require-perfect` for a reason unrelated to the scorer it asserts.

Every trace document declares what produced it. `provenance.source` is either
`referenceFixture` — a hand-written trace built to pass the scorer, whose
perfect result describes the scorer and nothing else — or `model`, a real
capture, which must additionally record the model id, prompt-template digest,
tool-choice setting, capture time, and the server, engine and registry versions
it ran against. A document without that block is rejected rather than scored,
because the same numbers mean "the harness works" or "the model performs this
well" depending on an answer the file was not carrying. The scorers print the
provenance alongside the metrics, so a score copied out of a log still says
which it is.

`--require-perfect` is only valid on a `referenceFixture`. It asserts that the
scorer runs end to end; pointing it at a model trace would turn the first clean
run into a committed claim that the model is perfect, which is the one thing
this corpus is least entitled to say. A model trace is scored and reported,
never asserted.

**Model baselines have been captured** and are committed under `eval/traces/`;
`claude-opus-5-full-corpus.json` and `claude-opus-5-repairs-full-corpus.json`
are the current full-corpus pair, and `docs/dev/` records what each capture
found. `npm run eval:capture` drives a real model over the four tools — one
call per corpus case per language, `tool_choice: auto` so the irrelevant-intent
cases can correctly produce no call — and writes a
`model` trace under `eval/traces/`, kept apart from the committed fixtures so no
directory listing presents the two as the same kind of artifact. It resolves
credentials the way the Anthropic SDK does (`ANTHROPIC_API_KEY`,
`ANTHROPIC_AUTH_TOKEN`, or an `ant auth login` profile) and, finding none,
exits non-zero having written nothing. `capture-traces.test.js` exercises the
harness against a scripted client; it tests prompt assembly and tool-call
extraction, not a model.
`npm run eval:capture-repairs` captures the other half: for each repair case it
asks, executes the model's call against the real server, hands the whole
structured result back as a `tool_result`, and records the second attempt. Both
attempts are recorded as *calls*, never as outcomes — the scorer replays them
itself, because a capture that recorded its own verdict would be grading the
model with the code that produced its answer. A turn that calls nothing, or a
model that gives up after reading the diagnosis, is recorded rather than
dropped: a harness that could only capture the runs that went well would report
a repair rate computed over those.

`score-repairs.js` replays a failed attempt and its model-produced revision,
requires the expected structured diagnosis before the revision can count, and
reports diagnosis-observation and diagnosis-driven repair rates, per language.
It replays whichever tool the model chose, not only `compute`: `1 2 AD` through
`check` returns the identical diagnosis, so replaying one tool scored a model
that checked before running as never having seen a diagnosis at all.
The cases cover unknown Words, stack shape, malformed source and the
`collectionWork` ceiling — the last of these exists to make a claim testable:
a ceiling named for collections should send a repair at the collection, and its
source contains no arithmetic, so a repaired attempt that succeeds can only
have shrunk the collection. Their reference trace is
also a scorer fixture, not evidence of model performance.
When a refusal comes from a ceiling charged as the operation proceeds — the
collection scans — `diagnosis.resourceLimit` carries a `progress`
`{ completed, total, unit }` alongside `observed`. It exists because `observed`
cannot serve there: such a meter stops the instant the budget is crossed, so it
reads a hair over the limit however far over the request was, and a reader
taking it proportionally under-corrects wildly. `completed` is the size that
fits. Measured against a real model, adding it moved the diagnosis-driven repair
rate from 0.750 to 1.000 on the same corpus.

`eval:validate` rejects duplicate or unknown case IDs, unknown tools, malformed
JSON pointers and incomplete committed reference traces before scores are
calculated. This prevents malformed or selectively omitted traces from
silently producing plausible metrics.
`eval:performance` measures five post-warmup rounds over seven representative
compute, check, inference and registry cases. It reports p50/p95/max latency by
tool and fails when the overall p95 exceeds the committed one-second local
stdio budget. It also reports median and maximum response size — for the whole
result and for its text block alone — and fails against
`medianResponseBytesBudget`; response bytes are deterministic for a fixed
corpus and engine, so unlike the latency figures that gate is exact and
reproducible. The latency measurements describe this adapter and machine, not
remote service latency.
`eval:number-baseline` compares canonical results for five selected rational,
decimal and integer operations with JavaScript `Number`. It includes two
exactly representable controls as well as known precision-sensitive cases, and
labels its scope explicitly; it is not a general JavaScript or CAS benchmark.
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
