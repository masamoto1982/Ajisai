# Ajisai Canonical Specification

Status: **Canonical**  
Version: **2026-04-10 canonical reset**

This document is the single design authority for Ajisai. If any other file conflicts with this document, this document wins.

---

## 1. Language Identity

Ajisai is an **AI-first, vector-oriented, fractional-dataflow language** with:
- Rust interpreter core
- WASM boundary
- TypeScript GUI/runtime shell

Ajisai is designed for mechanical reasoning, automated refactoring, and structurally searchable implementation.

---

## 2. Canonical Scope and Non-Canonical Scope

### 2.1 Canonical
1. `SPECIFICATION.md` (this file)
2. Rust implementation behavior that conforms to this file
3. WASM/TS observable contracts derived from this file

### 2.2 Non-canonical
Any roadmap, handover note, TODO note, or historical design memo is non-canonical unless explicitly promoted here.

---

## 3. Architectural Core

### 3.1 Two-plane architecture (mandatory)
Ajisai runtime is split into:

1. **Data plane** (mandatory, computation plane)
   - `ValueData` payloads used in execution
   - arithmetic/comparison/structure operations
   - no semantic formatting metadata in compute path

2. **Semantic plane** (mandatory, metadata plane)
   - display hints, module metadata, presentation annotations
   - queried only at explicit semantic boundaries (rendering/output/module side effects)

This separation is mandatory and testable.

### 3.2 Observable behavior boundary

Observable behavior is limited to:
- produced values
- explicit runtime errors
- documented side effects (I/O/module effects)
- documented ordering constraints

Internal data layout, temporary allocations, and optimization strategy are implementation details unless explicitly elevated here.

---

## 4. Runtime Value Model

Canonical runtime values:
- Scalar fraction
- Vector (possibly nested)
- Record
- NIL
- Code block
- Process handle
- Supervisor handle

Strings/booleans/datetime-like representations are encoded over core value forms and may be accompanied by semantic hints.

---

## 5. Fractional-Dataflow Semantics

### 5.1 Canonical user-facing semantics
- Operations consume inputs and produce outputs under stack/mode rules.
- Keep-mode and targeting modifiers determine whether source values remain accessible after operation.
- Pipelines are deterministic for identical code/input/module state.

### 5.2 Internal invariant semantics
Ajisai runtime may track flow mass/conservation using flow tokens.

This is an **internal runtime invariant**, not a default user-facing semantic contract.

The runtime must preserve internal consistency where enabled by implementation.

Canonical boundary: FlowToken fields (ID, remaining mass, parent/child links, ratios) are internal runtime state and must not be treated as default user-visible output.

### 5.3 Bifurcation semantics
- `,,` (bifurcation / keep-mode) is user-visible as: “retain source context while emitting result according to modifier rules.”
- Mass ratio / branch conservation details are internal by default.
- Optional diagnostics may expose flow-token information, but diagnostic visibility is non-default and must not redefine user-level language meaning.

---

## 6. Modifiers and Execution Modes

### 6.1 Target mode
- `.`: default target selection
- `..`: whole-stack target selection

### 6.2 Consumption mode
- `,`: consume mode (default)
- `,,`: keep/bifurcation mode

### 6.3 Safety mode
- `~`: safe mode (errors become NIL where defined)

### 6.4 Let-it-crash runtime model
- `~` is local error absorption for a single operation.
- Child runtime words (`SPAWN`, `AWAIT`, `STATUS`, `KILL`, `MONITOR`, `SUPERVISE`) provide isolated execution lifecycle control.
- Child runtimes are isolated from parent stack/user-word mutation during execution.
- Child failures are observed as exit values (`ok` / `exit` / `killed` / `timeout`) and do not immediately crash the parent interpreter.

Mode composition must be explicit and mechanically testable.

---

## 7. Error Model (Canonical decisions)

### 7.1 NoChange policy
**`NoChange` is not a canonical runtime error.**

Operations that produce a value equal to their input are successful unless another canonical error condition applies.

### 7.2 Canonical error categories
Runtime errors are reserved for conditions such as:
- invalid arity
- invalid target/type/shape constraints
- invalid indices/ranges
- parse/execution contract violations
- module/import failures

Error wording should be stable enough for tests, but exact phrasing is secondary to category correctness unless strict wording is explicitly required by tests.

---

## 8. Call Depth and Recursion Policy

Legacy fixed custom-word call-depth limits are **non-canonical**.

Canonical rule:
- No hard-coded shallow call-depth limit as a language semantic rule.
- Implementations may apply execution resource guards (step budget, timeout, memory guard) as runtime safety controls.

Resource guard behavior must be documented by implementation and tested as implementation policy, not as core language semantics.

---

## 9. Vector/Tensor Operation Discipline

Vector/tensor implementations must follow explicit staged structure:
1. flatten input
2. compute shape/stride/index metadata
3. transform indices/selections
4. rebuild output

Ad hoc recursive shape mutation in intermediate stages is prohibited except for final reconstruction boundary.

---

## 10. AI-first Implementation Rules

### 10.1 Mandatory rules
- Prefer explicit, structurally searchable function/module names.
- Keep Rust files under 500 lines (including comments/blanks).
- Keep control flow shallow and phase-separated.
- Separate semantic changes from structural cleanup in change management.
- Remove legacy coexistence patterns instead of dual-mode drift.

### 10.2 Advisory rules
- Prefer small helper extraction for duplicated control scaffolding.
- Prefer deterministic, low-ambiguity error classification.
- Prefer mechanically enforceable tests over narrative docs.

---

## 11. Documentation Authority Rules

- Canonical: this file.
- Secondary docs (`README.md`, `docs/dev/*.md`, handover notes) must not define competing semantics.
- If historical context is needed, it must be explicitly marked archival and non-canonical.

---

## 12. Compatibility Policy

Ajisai does not guarantee backward compatibility with obsolete semantics.

When behavior changes are made to converge with this spec, each meaningful change must be classified as one of:
- canonicalization
- legacy removal
- AI-first strengthening
- bug fix

---

## 13. Conformance Checklist

A change is conformant only if all are true:
1. It does not introduce a second design authority.
2. It does not reintroduce `NoChange` runtime-error semantics.
3. It does not rely on fixed shallow call-depth legacy semantics.
4. It preserves data-plane/semantic-plane separation.
5. It keeps vector/tensor staged pipeline boundaries explicit.
6. It improves or preserves AI-first structural clarity.

---

## 14. Phase-1 Frozen Canonical Decisions

This canonical reset freezes the following:
1. Single authority = `SPECIFICATION.md`.
2. `NoChange` runtime error model is removed from canonical semantics.
3. Legacy fixed call-depth limit is removed from canonical semantics.
4. Bifurcation mass/conservation is internal invariant by default.
5. Observable behavior boundary is explicitly defined.
6. AI-first rules are split into mandatory vs advisory.
7. Secondary docs are explicitly non-canonical unless promoted.
