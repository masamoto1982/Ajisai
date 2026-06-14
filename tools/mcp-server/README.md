# Ajisai MCP server

A thin [Model Context Protocol](https://modelcontextprotocol.io) server that
exposes Ajisai to AI agents. It holds **no language logic**: every answer
comes from running the `ajisai` CLI (Phase 1) or reading a generated artifact
(`SKILL.md` from Phase 2, `docs/word-manifest.json`). Its only dependency is
the MCP SDK.

## Tools

| tool | input | returns |
|---|---|---|
| `run` | `source` (text) **or** `file` (path) | the CLI's `ajisai run --json` envelope (status, stack, `stackDisplay`, output, diagnosis, errorFlowTrace, aiDiagnostic, runtimeMetrics incl. `energyProxyScore`) |
| `explain_word` | `word` (e.g. `MAP`, `MUSIC@PLAY`) | matching `docs/word-manifest.json` entries |
| `skill` | — | the full `SKILL.md` agent writing protocol |

## Setup

```sh
cargo build --bin ajisai --manifest-path rust/Cargo.toml   # the CLI it wraps
cd tools/mcp-server && npm install                          # the MCP SDK
node selftest.js                                            # optional: end-to-end check
```

The server finds the repo (for `SKILL.md` / the manifest) and the CLI binary
automatically. Override with `AJISAI_REPO` and `AJISAI_BIN` if needed.

## Connect from Claude Code

Add to `~/.claude.json` (or run `claude mcp add`), then restart Claude Code:

```json
{ "mcpServers": { "ajisai": { "command": "node",
  "args": ["/ABSOLUTE/PATH/Ajisai/tools/mcp-server/index.js"] } } }
```

Any other MCP client connects the same way: launch `node index.js` as a
stdio MCP server. The three tools then appear as `ajisai/run`,
`ajisai/explain_word`, and `ajisai/skill`.
