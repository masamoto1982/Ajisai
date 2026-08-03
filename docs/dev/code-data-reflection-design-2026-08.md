# Code/data reflection design record (2026-08)

> **Status:** non-normative architecture decision record. This document explains the choice; `spec/` defines Ajisai semantics and Word contracts.

## Decision

Ajisai exposes one canonical Core Word, `REFLECT`, as the explicit and reversible boundary between a `CodeBlock` token sequence and the versioned `AJISAI-CODE-1` Vector representation. For every legal operand, reflection is an involution under token or structural equality.

## Why `REFLECT`

The name describes the symmetric relation rather than either destination. `CODE` would read as a data-to-code conversion and bias one direction; directional names would make the inverse look like a separate capability. Reflection names code and its data image without implying execution.

## Why one Word rather than two

The two directions are one reversible relation, not independent operations. A single type-directed involution keeps the boundary compact, makes the round-trip law visible, and avoids consuming two Core names and any aliases.

## Why Core grows from 69 to 70

Existing user Words cannot inspect or construct `CodeBlock` token sequences: the separation is deliberately below the user-definition layer. The boundary therefore requires one irreducible primitive. Adding exactly one Word is preferable to weakening the type separation or adding a general evaluator. The alias count remains unchanged.

## Why `EXEC` is not extended

A Vector remains data even when it resembles canonical code data. Execution still requires two explicit steps—`REFLECT` to obtain a `CodeBlock`, then `EXEC`, `DEF`, or a higher-order Word. This keeps evaluation authority visible in source and prevents ordinary Vectors or Strings from becoming executable by representation guessing.

## Why persistence remains separate

`PersistToken` is a private compatibility wire format for session snapshots. Publishing it would couple language semantics to serde tags and field names and make persistence migrations language changes. `AJISAI-CODE-1` instead defines only the language-visible representation; the persistence codec and existing snapshots remain unchanged.

## Deferred capabilities

This change does not introduce a String parser, because display or source-like text is not authoritative code. It does not introduce macros, because token reflection and expansion are distinct capabilities. It adds no AST or seventh value domain: canonical code data uses ordinary Vectors, while executable values remain CodeBlocks.

## Consequences

Number lexemes, String contents, Symbol spelling/case/aliases, delimiters, control tokens, and line breaks round-trip without rendering or normalization. Decoding is strict and malformed data is ERROR. Reflection performs no evaluation, dictionary lookup or mutation, dependency fixation, output, host effect, identity computation, cache update, or persistence operation.
