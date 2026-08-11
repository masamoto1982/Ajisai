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

```sh
npm install -g ajisai-mcp-server
```

No build step and no native binary are required: the packaged WASM backend
ships inside the package.

Most MCP clients take a JSON server entry. Claude Desktop
(`claude_desktop_config.json`), Claude Code (`.mcp.json`), and Cursor
(`.cursor/mcp.json`) all use this shape:

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

A globally installed copy, or a checkout, can be named directly instead:

```json
{
  "mcpServers": {
    "ajisai": {
      "command": "ajisai-mcp-server"
    }
  }
}
```

```json
{
  "mcpServers": {
    "ajisai": {
      "command": "node",
      "args": ["/path/to/Ajisai/tools/mcp-server/index.js"],
      "env": { "AJISAI_BIN": "/path/to/Ajisai/rust/target/release/ajisai" }
    }
  }
}
```

`AJISAI_BIN` is optional and selects a native `ajisai` binary instead of the
packaged WASM backend — useful in a Docker image that builds one in.
`AJISAI_REPO` is a development-only fallback for discovering a locally built
binary without naming it.

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

Every result also carries `mcp.engineVersion`, `mcp.registryDigest` and the
applied `mcp.limits`. The packaged registry is verified against its recorded
digest at **startup**, so a corrupt asset stops the server rather than
surfacing as a generic failure on whichever request touched it first.

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
```

The packaged WASM bundle (`wasm/generated/`) is regenerated by
`npm run build:mcp-wasm` at the repo root. Vocabulary, contracts, guide,
version and registry provenance are packaged under `assets/` regardless of
backend.

`npm run test:mcp-backends` at the repo root (builds the native binary, then
runs `backend/parity-test.js`) runs every golden case and every declared limit
boundary against both backends and asserts they agree.

`eval/cases.json` is the 22-prompt seed agent-evaluation corpus. `npm run eval`
executes its expected tool calls against the real backend. It measures backend
semantic correctness only; model tool selection and source generation require
captured model traces and are not claimed by this score.
`score-traces.js` accepts captured model traces in the documented reference
shape and reports tool-selection accuracy, end-to-end semantic success, missing
traces and irrelevant-tool rate. The committed reference trace is a harness
conformance fixture, not a model benchmark result.
`score-repairs.js` replays a failed attempt and its model-produced revision,
requires the expected structured diagnosis before the revision can count, and
reports diagnosis-observation and diagnosis-driven repair rates. The seed cases
cover unknown Words, stack shape and malformed source. Their reference trace is
also a scorer fixture, not evidence of model performance.
`eval:validate` rejects duplicate or unknown case IDs, unknown tools, malformed
JSON pointers and incomplete committed reference traces before scores are
calculated. This prevents malformed or selectively omitted traces from
silently producing plausible metrics.
`eval:performance` measures five post-warmup rounds over seven representative
compute, check, inference and registry cases. It reports p50/p95/max latency by
tool and fails when the overall p95 exceeds the committed one-second local
stdio budget. The measurements describe this adapter and machine, not remote
service latency.
`eval:number-baseline` compares canonical results for five selected rational,
decimal and integer operations with JavaScript `Number`. It includes two
exactly representable controls as well as known precision-sensitive cases, and
labels its scope explicitly; it is not a general JavaScript or CAS benchmark.
`npm run test:pack` creates the allowlisted tarball, installs it into an empty
temporary prefix and imports that installed copy. It computes through the
real backend twice: first with neither `AJISAI_REPO` nor `AJISAI_BIN` set,
proving the installed package computes through its packaged, self-contained
WASM backend with no repository and no native binary in reach; then again
with an explicit `AJISAI_BIN`, proving the native/Docker override path still
works.

The browser playground is independent of this package and remains available.
