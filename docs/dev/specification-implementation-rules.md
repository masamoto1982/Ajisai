# AI-first Implementation Rules

How this repository is written. Engineering discipline for contributors —
not part of the language.

## Authority

- **Non-canonical.** LANG.AUTHORITY.SOURCES places `docs/dev/` outside the
  normative set. Nothing here defines Ajisai semantics or constrains a
  conforming implementation.
- Until #1652 this content was inlined into `SPECIFICATION.html` as a
  numbered section 12, where it rendered identically to the normative
  clauses two paragraphs after the one saying `docs/dev/` defines nothing.
  The specification's own table of contents never listed it. It now lives
  only here, which is where the authority clause already said it belonged.
- The first Mandatory rule below is machine-enforced by
  `scripts/check-file-size-budget.mjs` against
  `docs/quality/file-size-baseline.json`; the rest are read, not gated.

## Mandatory

- Prefer explicit, structurally searchable function and module names.
- Keep newly created Rust source files, and files that are substantially
  rewritten, at or under 500 lines. Existing over-budget files are recorded
  as a baseline (`docs/quality/file-size-baseline.json`) and, as a goal,
  must not be changed in ways that further increase their line count.
- Keep control flow shallow and phase-separated.
- Separate semantic changes from structural cleanup in change management.
- Maintain single canonical implementations; do not allow dual-mode drift.
- Source code comments are allowed when they clarify intent, invariants,
  traceability, or non-obvious behavior. When source code is changed, nearby
  comments must be reviewed and updated so they remain accurate. Comments
  that merely restate obvious code should be avoided.

## Advisory

- Prefer small helper extraction for duplicated control scaffolding.
- Prefer deterministic, low-ambiguity error classification.
- Prefer mechanically enforceable tests over narrative documentation.
