# Ajisai specification sources

This directory holds every normative source for the language. Nothing outside
it defines Ajisai semantics.

| Source | Defines |
| --- | --- |
| `language-semantics.md` | Program meaning — the semantic kernel |
| `words.json` (`words.schema.json`) | The canonical vocabulary and each Word's contract |
| `semantic-families.json` | The shared laws Words select |
| `gui-semantics.md` | Presentation |
| `host-protocol-v2.schema.json` | The host protocol boundary between them |
| `tensor-profile-v0.1.md` (`tensor-profile-v0.1.json`, `tensor-profile.schema.json`) | The versioned, opt-in approximate Tensor extension |
| `typed-graph-ir.schema.json` | The portable typed SSA/dataflow exchange format used by profiles |

The two `.md` sources retain raw HTML blocks so the generated specification
preserves the existing typography, anchors, tables, and mathematical channels
without a lossy Markdown migration.

The Tensor Profile is deliberately not part of Core: Core retains its six
canonical domains, exact Scalars, and frozen vocabulary. A profile document is
normative only for a program or graph that explicitly selects that profile.

Within the current host protocol version, consumers may receive new optional
fields, but existing fields, meanings, and tuple shapes cannot be removed,
renamed, reordered, or changed. A breaking change raises the protocol version
and supersedes the previous one; exactly one protocol is current at a time.

The `freeze/` fixtures pin representative protocol payloads and the production
GUI surface. Contract tests deliberately inspect the existing sources rather
than duplicating GUI behavior in a replacement implementation.

`SPECIFICATION.html` is a distribution artifact assembled from the semantic
sources, the implementation-rules fragment, and `specification.template.html`.
Run `npm run specification:generate` after an authoritative source change and
`npm run specification:check` in quality gates.

`npm run semantic-kernel:check` enforces the budgets that keep the language
small: at most 400 lines of kernel, 12 semantic families, 57 canonical Words,
and 16 aliases, with every family and clause reference resolving. The budgets
are ceilings — shrinking is always allowed, growing is a deliberate
specification change.
