# Software Level Classification

## Intent
Ajisai uses an internal software criticality classification inspired by DO-178B levels.

## Internal Levels
- **QL-A (Highest)**: Behavior that can corrupt core execution semantics or persisted user data.
- **QL-B**: Behavior that can produce incorrect user-visible execution results without data loss.
- **QL-C**: Behavior that can degrade tooling/workflow while preserving core correctness.
- **QL-D (Lowest)**: Documentation, UX polish, and non-functional enhancements.

## Current Baseline Mapping
- Interpreter semantics and numeric correctness: **QL-A**
- Parser/tokenizer correctness and WASM boundary conversions: **QL-B**
- Build tooling and non-critical adapters: **QL-C**
- Documentation and process-only changes: **QL-D**

## Usage Rules
- Each PR should label affected areas with the highest impacted internal level.
- Verification rigor is scaled by the highest impacted level (see `VERIFICATION_PLAN.md`).
