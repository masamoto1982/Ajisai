# Changelog — ajisai-mcp-server

This package is versioned separately from the Ajisai engine it speaks for
(`mcp.serverVersion` and `mcp.engineVersion` on every result). The engine is
still alpha and makes no compatibility promise; this file records changes to
the adapter's own surface — tool list, envelope fields, resources and
descriptions.

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
