# Agent CLI output contract (`ajisai --json`)

Status: implemented host contract. This document is not an authority for
language semantics; `SPECIFICATION.html` remains that authority. It documents
only commands and fields emitted by the current native CLI.

## Commands

```text
ajisai run <file.ajisai> [--json] [--step-limit <N>]
ajisai check <file.ajisai> [--json] [--contract]
ajisai contract <file.ajisai> [--json]
ajisai agent <compute|check|infer-contracts> <file.ajisai>
ajisai test <file-or-dir> [--json]
ajisai repl [--json]
ajisai version [--json]
```

No other command or option is part of the current CLI contract. In particular,
this document does not reserve planned commands.

| exit | meaning |
|---:|---|
| 0 | success |
| 1 | Ajisai language, check, contract, or test failure |
| 2 | CLI usage or host file-reading failure; JSON is not guaranteed |

With `--json`, commands that produce a JSON report write one document to
stdout. Program `PRINT` effects are captured in the document rather than mixed
into stdout. `--step-limit` is a positive integer and applies only to `run`;
the default is 100,000. `--contract` applies only to `check`.

## `run` and `check`

Both commands emit schema version 1:

```json
{
  "schemaVersion": 1,
  "status": "ok",
  "stack": [],
  "stackDisplay": [],
  "output": [],
  "message": null,
  "diagnosis": null,
  "errorFlowTrace": [],
  "aiDiagnostic": null,
  "runtimeMetrics": {},
  "contractDecls": null
}
```

`status` is `ok` or `error`. Ajisai language errors use exit 1 and retain this
JSON envelope. `check` tokenizes, checks delimiter structure, performs
best-effort static Word resolution, and never executes the program. With
`--contract`, `contractDecls` contains the conservative result of comparing
`#:contract` declarations with inferred contracts.

Native `run` obtains this document from the typed, source-only Rust
`cli::agent_api::compute` boundary. Terminal formatting is a consumer of that
report and is not part of computation semantics.
JSON `check` likewise consumes `cli::agent_api::check`; human-readable check
output remains a terminal-only projection.

### Stack value nodes

Stack nodes are produced by the same Rust value-protocol mapping as the WASM
playground boundary:

```json
{
  "type": "number",
  "value": { "numerator": "3", "denominator": "2" },
  "displayHint": "rawNumber",
  "semantics": {}
}
```

Arbitrary-precision integers are decimal strings, never JSON floating-point
numbers. Vectors contain arrays of value nodes. NIL carries normalized absence
metadata in `semantics.absence`. Logical Unknown is observed through the truth
axis rather than serialized as operational NIL.

An algebraic irrational retains the approximate rational compatibility view,
marks it with `approximate: true`, and carries its authoritative multiquadratic
normal form:

```json
{
  "type": "number",
  "value": { "numerator": "768398401", "denominator": "543339720" },
  "displayHint": "rawNumber",
  "semantics": {
    "approximate": true,
    "exactTerms": [
      { "numerator": "1", "denominator": "1", "radicand": "2" }
    ]
  }
}
```

`exactTerms` encodes `Σ (numerator / denominator) √radicand`. When it is
present, the `value` rational is a display compatibility view and is not the
canonical value.

### Diagnosis and error flow

`diagnosis` is a structured failure explanation with `when`, `why`, `summary`,
`where`, `evidence`, `nextChecks`, and `agreedPrefix`. `aiDiagnostic` is its
machine-oriented classification. Consumers must treat new protocol-string
variants as opaque values rather than rejecting the report.

`errorFlowTrace` records Word errors and reason-carrying NIL production. A
successful run may therefore have a non-empty trace. Neither NIL nor an Ajisai
language `status: error` is a host transport failure.

## `contract`

`contract --json` emits a JSON array, not the `run` envelope. Each entry reports
a user Word's inferred `name`, `arity`, `purity`, `determinism`, NIL behavior,
order sensitivity, space class, effects, confidence, and a paste-ready
`suggested` declaration. Inference registers definitions without executing
their bodies.

The MCP adapter normalizes this legacy bare array into its common result
envelope under `contracts`; the native CLI shape remains unchanged in schema
version 1.
The array itself is produced by `cli::agent_api::infer_contracts` so native and
future embedded hosts share inference rather than reimplementing it.

## `test`

`test` evaluates files against `#@` directives for expected status, stack,
output, and error text. With `--json`, it emits a test-run report and exits 1 if
any case fails. Test directives are host tooling comments and do not change
Ajisai program semantics.

## `repl`

The REPL preserves stack and definitions across lines. In JSON mode, every
submitted program line produces one `run`-shaped JSON document. REPL
meta-commands (`:help`, `:stack`, `:reset`, and `:quit`) are host commands, not
Ajisai Words.

## `version`

`version --json` emits:

```json
{ "schemaVersion": 1, "status": "ok", "version": "0.2.0-beta.1" }
```

## `agent`

`agent` is the stable JSON-only host boundary used by the MCP adapter. Its
`compute`, `check`, and `infer-contracts` operations call the typed Rust agent
API and always return a schema-versioned object. In particular,
`infer-contracts` returns the array under `contracts`, avoiding the legacy bare
array emitted by the compatibility `contract --json` command.

## Compatibility

Additive fields do not change `schemaVersion`; consumers must ignore fields
they do not recognize. Removing or renaming fields, changing their types, or
changing exit-code meaning requires a schema-version increment.

The MCP-specific envelope and its exact-value definitions are additionally
published as `tools/mcp-server/result.schema.json`. That schema adds MCP
provenance and applied limits to the native report without changing Ajisai
semantics.
