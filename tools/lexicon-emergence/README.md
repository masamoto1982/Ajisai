# Lexicon-emergence experiment

Tooling for `docs/dev/lexicon-emergence-experiment-work-order-2026-09.md`: Core
is held fixed, subject agents solve shared task families through the Ajisai MCP
server, and the shared User dictionary passes from generation to generation
through a capacity bottleneck. This directory grades, classifies and reports;
the subject agents themselves are started elsewhere (Phase 1: Claude Code
subagents; Phase 2: a harness).

Nothing here defines Ajisai. Results are observation notes.

## Layout

| Path | What |
| --- | --- |
| `tasks/<family>.json` | Task families: prompts, inputs (the subject sees only the first), and a reference solution per task |
| `lib/ajisai.mjs` | MCP client over `tools/mcp-server` — the grader uses the same `compute` tool as the subjects |
| `lib/source.mjs` | Tokenizing, bound-variable renaming, and expanding user Words into Core-only source (a header-carrying Word through `BIND`, faithful under `KEEP`; a header-less one through `EXEC`, which is not) |
| `lib/grade.mjs` | A solution is correct when every input leaves the reference's final stack; also runs the Core-only expansion (H5) |
| `lib/identity.mjs` | D0 (DIGEST), D0α (DIGEST after renaming bound variables), D1 (probe-battery fingerprint) |
| `lib/evolve.mjs` | Equivalence classes and the next generation's top-K lexicon |
| `lib/analyze.mjs` | H1, H2, H4, H5 over a run |
| `lib/prompt.mjs` | The single message a subject agent receives |
| `runs/<run>/<condition>/gen<g>/` | `lexicon.json`, `submissions/`, `graded/` |

## Use

Requires `npm install` in `tools/mcp-server` (the SessionStart hook does it).

```sh
node cli.mjs prompt  <run> <condition> <gen> <agent>   # message for one subject
node cli.mjs grade   <run> <condition> <gen>           # after submissions are written
node cli.mjs evolve  <run> <condition> <gen> <K>       # writes gen+1/lexicon.json
node cli.mjs analyze <run>                             # runs/<run>/report.{md,json}
```

## What the instrument can and cannot say

- **D0** equal means the same Word. It ignores Word names, even through
  dependencies, and normalizes aliases (`*` = `MUL`). But it tells apart
  bodies that differ only in their `BIND` names.
- **D0α** renames bound variables to `_B0`, `_B1`, … before hashing, and
  merges those bodies.
- **D1** only means the engine gave the same answers on the probe battery in
  `lib/identity.mjs`. The battery includes scalars and one-element Vectors,
  because `[ 2 ] *` and `2 *` differ only on a scalar input, and irregular
  text (`''`, `'x  y'`, `' lead'`), because without it a word-splitting and
  a character-scanning title-caser answered alike (pilot, 2026-09-23). A Word that
  errors on every probe gets no D1.
- CONTRACT is not used as a level. For any body that uses `BIND`, it reads the
  bound names as unresolved Words and answers `inputs: variable`.
