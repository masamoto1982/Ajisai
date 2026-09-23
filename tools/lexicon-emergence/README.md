# Lexicon-emergence experiment

Tooling for `docs/dev/lexicon-emergence-experiment-work-order-2026-09.md`: Core
is held fixed, subject agents solve shared task families through the Ajisai MCP
server, and the shared User dictionary passes from generation to generation
through a capacity bottleneck. This directory grades, classifies and reports;
the subject agents run either as Claude Code subagents (route A, the pilot)
or through the Claude API harness in `lib/harness.mjs` (route B, Phase 2 on).

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
| `lib/prompt.mjs` | The single message a subject agent receives (route A: read SKILL.md, write a file; route B: the harness supplies SKILL.md and a `submit` tool) |
| `lib/harness.mjs` | Route B: one subject through the Claude API. Offers the MCP server's own `compute` / `check` / `word_contract` definitions plus `submit`, relays calls unchanged, caps turns and spending, records every request's served model, usage and cost |
| `test/` | The harness against a scripted model (no API call) with the real server and grader: `npm test` |
| `runs/<run>/<condition>/gen<g>/` | `lexicon.json`, `submissions/`, `graded/`, and for route B `transcripts/` |

## Use

Requires `npm install` in `tools/mcp-server` and here (the SessionStart hook does both).

```sh
node cli.mjs prompt  <run> <condition> <gen> <agent>   # message for one subject
node cli.mjs grade   <run> <condition> <gen>           # after submissions are written
node cli.mjs evolve  <run> <condition> <gen> <K>       # writes gen+1/lexicon.json
node cli.mjs analyze <run>                             # runs/<run>/report.{md,json}

# Route B — needs ANTHROPIC_API_KEY in the environment and spends money.
node cli.mjs run   <run> <condition> <gen> G0A,G0B --model large --budget 10
node cli.mjs pilot <run> --model large --effort high --budget 40   # Phase 1's design end to end
```

`--model` takes a model id or `large` / `medium` / `small` (`claude-opus-5`,
`claude-sonnet-5`, `claude-haiku-4-5`). `--budget` is a ceiling in USD checked
before every request, from the rates in `lib/harness.mjs`; a run that reaches
it stops with an error and keeps what it wrote. Server-side refusal fallbacks
are deliberately not enabled: a fallback would change which model answered,
which is the variable §5.3 controls. A refusal is recorded as the run's end.

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
