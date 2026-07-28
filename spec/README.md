# Ajisai specification sources

This directory implements the one-way authority structure described by the
semantics-compaction plan. `language-semantics.md` and `gui-semantics.md` are
the normative sources. They retain raw HTML blocks during the physical split
so the generated integrated specification preserves the existing typography,
anchors, tables, and mathematical channels without a lossy Markdown migration.

`host-protocol-v1.schema.json` is the machine-readable compatibility boundary.
Within V1, consumers may receive new optional fields, but existing fields,
meanings, and tuple shapes cannot be removed, renamed, reordered, or changed.
A breaking protocol must coexist under a new major version.

The `freeze/` fixtures pin representative protocol payloads and the production
GUI surface. Contract tests deliberately inspect the existing sources rather
than duplicating GUI behavior in a replacement implementation.

`SPECIFICATION.html` is a distribution artifact assembled from the two
semantic sources, the implementation and quality fragments, and
`specification.template.html`. Run `npm run specification:generate` after an
authoritative source change and `npm run specification:check` in quality gates.
