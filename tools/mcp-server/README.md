# Ajisai MCP server

Ajisai's MCP surface makes the language useful as a bounded, deterministic and
diagnostic computation kernel for AI agents. The server remains a thin adapter:
the Rust CLI owns language semantics and generated artifacts own vocabulary.

Ajisai promises **exactness in its supported numeric domain**, rather than
unqualified “no rounding errors”. Operations such as explicit rounding and
functions outside that domain retain their documented semantics.

## Agent surface

| tool | purpose |
|---|---|
| `compute` | execute source with time, source, output and step limits |
| `check` | parse, resolve and conservatively verify declared contracts without execution |
| `infer_contracts` | infer contracts for user-defined Words without execution |
| `word_contract` | query the complete canonical `spec/words.json` contract registry |

Execution tools accept source text only. Deliberately omitting file-path input
prevents an AI tool call from becoming an arbitrary local-file reader. Ajisai
language errors and reason-carrying `NIL` remain structured, successful MCP
results; only invalid requests and host failures set `isError`.
All execution tools call the native CLI's common `agent` operation, backed by
the typed Rust agent API; Node does not reinterpret command-specific results.
The local server also caps concurrent CLI executions, returning a retryable
host error instead of allowing an unbounded process fan-out.
Its Rust agent profile additionally caps materialized elements, numeric-literal
digits, cumulative numeric work, BigInt bit length and algebraic term count.

Static context is available through `ajisai://guide/quickstart`,
`ajisai://vocabulary`, and `ajisai://schema/result`. Tool calls return both text
and `structuredContent`, including the engine version, registry SHA-256 and
applied limits. Algebraic results carry their canonical `exactTerms` normal
form; the accompanying rational value is explicitly marked as an approximation
and is never the canonical result.
The committed `result.schema.json` is both the tools' `outputSchema` and the
content of `ajisai://schema/result`, preventing the advertised contract and the
resource documentation from drifting apart. All four tools declare read-only,
non-destructive and idempotent MCP annotations.
The committed `golden/cases.json` fixtures pin observable success, NIL, ERROR,
algebraic and execution-limit behavior without pinning incidental runtime
counters.

The `ajisai://words/{name}` resource template exposes the same complete Word
contract without a tool call. Contract lookups accept canonical names and
aliases; their registry digest is calculated from the canonical specification,
not from a reduced documentation manifest.

## Setup

```sh
cargo build --bin ajisai --manifest-path rust/Cargo.toml
cd tools/mcp-server
npm install
npm run selftest
```

Set `AJISAI_BIN` to select another CLI binary or `AJISAI_REPO` to select the
artifact root. Connect any stdio MCP client to `node /path/to/index.js`. The
browser playground is independent of this package and remains available.
