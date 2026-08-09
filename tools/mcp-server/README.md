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
| `word_contract` | query the generated canonical Word registry |

Execution tools accept source text only. Deliberately omitting file-path input
prevents an AI tool call from becoming an arbitrary local-file reader. Ajisai
language errors and reason-carrying `NIL` remain structured, successful MCP
results; only invalid requests and host failures set `isError`.

Static context is available through `ajisai://guide/quickstart`,
`ajisai://vocabulary`, and `ajisai://schema/result`. Tool calls return both text
and `structuredContent`, including the registry SHA-256 and applied limits.

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
