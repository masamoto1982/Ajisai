# Traceability Policy

## Purpose
Maintain bidirectional traceability between intent, implementation, and verification evidence.

## Required Links
Each significant change should map:
1. Requirement or objective (spec section, issue, design intent)
2. Implementation unit(s) (file/module/function)
3. Verification evidence (test case, CI job, checklist item)

## Conventions
- Requirement IDs use `AQ-REQ-###`.
- Verification IDs use `AQ-VER-###`.
- Trace rows are maintained in `TRACEABILITY_MATRIX.md`.

## Maintenance
- Add or update rows when behavior changes.
- Do not remove old rows without a replacement or deprecation note.
