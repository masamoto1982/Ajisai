# Ajisai specification sources

This directory holds every normative source for the language. Nothing outside
it defines Ajisai semantics.

| Source | Defines |
| --- | --- |
| `language-semantics.md` | Program meaning — the semantic kernel |
| `grammar.json` | The lexical grammar — what text is Ajisai source |
| `termination.json` | Why every evaluation is finite — the recursion sites and the measure |
| `identity.json` | When two things are the same — the law and each level's reach |
| `words.json` (`words.schema.json`) | The canonical vocabulary and each Word's contract — including its family, whose shared laws are the clauses every Word of the family cites |
| `outcomes.json` | The complete outcome space — every NIL reason and every error category a Word's contract can name |
| `outcome-witnesses.json` | Programs that reach an outcome the one-Word semantics table cannot — the by-hand half of the outcome bijection gate |
| `retired-words.json` | The names that were once Words and must stay unknown — read by both the registry gate and the runtime test |
| `gui-semantics.md` | Presentation |
| `host-protocol.schema.json` | The host protocol boundary between them |

`grammar.json` is the one source here that is executed rather than only read.
`scripts/lib/reference-lexer.mjs` interprets it — it hardcodes no character, no
token spelling and no rule order — and two gates hold both implementations to
that one file: `npm run check:grammar` runs the grammar over its own numeric
examples and over one witness program per source-error condition, and
`rust/src/lexical_grammar_laws.rs` runs the same file and the same witnesses
against the Rust tokenizer under `cargo test`. A failure means the grammar and
the tokenizer disagree; the grammar is canonical for what source *is*, so a
deliberate language change updates it first.

The two `.md` sources retain raw HTML blocks so the generated specification
preserves the existing typography, anchors, tables, and mathematical channels
without a lossy Markdown migration.

Within the current host protocol version, consumers may receive new optional
fields, but existing fields, meanings, and tuple shapes cannot be removed,
renamed, reordered, or changed. A breaking change raises the protocol version
and supersedes the previous one; exactly one protocol is current at a time.

`SPECIFICATION.html` is a distribution artifact assembled from the semantic
sources, the implementation-rules fragment, and `specification.template.html`.

`npm run specification:check` runs in CI (`.github/workflows/test.yml`), so
the committed copy cannot drift from `spec/`: a change to any source here is
followed by `npm run specification:generate` in the same commit, or the gate
fails. Every source in this directory is current and still drives code
generation.

Only `words.schema.json` is machine-read (`scripts/generate-word-registry.mjs`
builds the Rust enums from it, and `word-schema:check` holds `words.json` to
it); the other JSON sources are validated by the gates that consume them
(`check:grammar`, `check:termination`, `check:identity`,
`outcome-registry:check`, `outcome-bijection:check`) rather than by a schema
document beside them.

Ajisai carries exactly two version numbers, not three: the implementation's
(`package.json`, `src-tauri/tauri.conf.json`, `npm run check:version-sync`)
and the specification's own, which the Status block of
`language-semantics.md` states beside the release stage. The two are tracked
independently — they both began beta at `1.0.0-beta.1` — and the
specification's moves only when the language does: a breaking change to the
vocabulary, to program meaning, or to the host protocol raises it. A
build-date stamp on the specification would have been a third, redundant
axis, so the Status block does not carry one; git history is the record of
when a given specification text was current.

`npm run semantic-kernel:check` enforces the budgets that keep the language
small — ceilings on the kernel's lines, on semantic families and on canonical
Words, whose numbers live in `scripts/check-semantic-kernel.mjs` alone — with
every family and clause reference resolving. The budgets are ceilings —
shrinking is always allowed, growing is a deliberate specification change.
