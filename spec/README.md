# Ajisai specification sources

This directory introduces the one-way authority structure described by the
semantics-compaction plan. Phase 0 freezes observation; it does not alter the
runtime or GUI.

`host-protocol-v1.schema.json` is the machine-readable compatibility boundary.
Within V1, consumers may receive new optional fields, but existing fields,
meanings, and tuple shapes cannot be removed, renamed, reordered, or changed.
A breaking protocol must coexist under a new major version.

The `freeze/` fixtures pin representative protocol payloads and the production
GUI surface. Contract tests deliberately inspect the existing sources rather
than duplicating GUI behavior in a replacement implementation.
