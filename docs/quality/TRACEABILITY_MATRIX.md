# Traceability Matrix

Requirement → implementation → verification evidence, as
`QUALITY_POLICY.md` §2 requires, `VERIFICATION_PLAN.md` asks QL-A and QL-B
changes to update, and `RELEASE_VERIFICATION_CHECKLIST.md` asks to be free of
unresolved high-criticality gaps.

Seven source files cite this document by path as their trace target. It did not
exist until this commit, so each of those citations was a dangling reference:
the matrix the quality process is built around had never been written down.

## On the requirement text below

Only **AQ-REQ-007** was ever written out, inline in
`rust/src/coreword_registry/safety_tests.rs`. The text for AQ-REQ-001 through
AQ-REQ-004 appears nowhere in the repository, so the statements below are
**reconstructed from what each verification suite says it covers** — they
record what is actually verified today, not an original requirement anyone
authored. Treat them as descriptive until an owner replaces them; a
reconstruction that disagrees with intent should be corrected here rather than
worked around in the suites.

## Matrix

| Requirement | Statement | Implementation | Verification | QL |
|---|---|---|---|---|
| **AQ-REQ-001** | Exact rational arithmetic and comparison are correct at every boolean decision in the numeric core, including the big-integer and sign-normalization paths. | `rust/src/types/fraction.rs`, `rust/src/types/fraction_arithmetic.rs`, `rust/src/types/bigint_gcd.rs` | `rust/src/types/fraction_mcdc_tests.rs` — **AQ-VER-001**, MC/DC tables `AQ-VER-001-A` `AQ-VER-001-B` `AQ-VER-001-C` `AQ-VER-001-D` `AQ-VER-001-E` `AQ-VER-001-G` `AQ-VER-001-H` `AQ-VER-001-I` | QL-A |
| **AQ-REQ-002** | Tokenization produces the correct token stream: no token-count drift, no wrong token kind, no mis-classified whitespace or comment. | `rust/src/tokenizer.rs` | `rust/src/tokenizer_mcdc_tests.rs` — **AQ-VER-002**, MC/DC tables `AQ-VER-002-A` `AQ-VER-002-D` `AQ-VER-002-E` `AQ-VER-002-F`; regression suites `rust/src/tokenizer_regression_tests.rs`, `rust/src/tokenizer_regression_tests_2.rs`, `rust/src/malformed_numeric_literal_tests.rs` | QL-B |
| **AQ-REQ-003** | The `(Value, hint)` → wire-format decision that crosses the host boundary is correct for every value domain, and the same decision survives the real `wasm-bindgen` glue. | `rust/src/types/value_protocol.rs`; `rust/src/wasm_interpreter_bindings/wasm_value_conversion.rs` (mechanical `JsValue` shim over it) | `rust/src/types/value_protocol_tests.rs` — **AQ-VER-003-C**, native MC/DC and property coverage of the mapping; `rust/wasm-tests/tests/boundary.rs` — end-to-end on `wasm32` via `wasm-pack test --node` | QL-A |
| **AQ-REQ-004** | The host runtime kind (web vs Tauri) is classified correctly from the build-time injection and the runtime DOM fallback. | `src/platform/runtime-kind.ts` | `src/platform/runtime-kind.test.ts` — **AQ-VER-004-A**; configuration in `vitest.config.ts`, which cites the suite as **AQ-VER-004** | QL-A |
| **AQ-REQ-007** | Built-in word purity classification is self-consistent with the effects and determinism each Word declares. (It also covered a derived `safe_preview` flag; no feature read the flag and no specification declared it, so it was removed.) | `rust/src/coreword_registry.rs`, `rust/src/coreword_registry/contract.rs`, projected from `spec/words.json` via `rust/src/kernel/generated/word_registry.rs` | `rust/src/coreword_registry/safety_tests.rs` — **AQ-VER-007**, cases **A**, **B**, **B2**, **C**, **D** (run the subset with `cargo test aq_ver_007`) | QL-B |

## Verification without a requirement ID

`rust/src/interpreter/nil_conformance_tests.rs` cites this document for "NIL
projection conformance" without naming a requirement. It drives the
interpreter and asserts the runtime honors each Word's declared `nil_policy`
and the NIL Projection Rule (`LANG.FAILURE.PROJECT`,
`LANG.FAILURE.PASSTHROUGH`), with registry-driven completeness so a newly
declared passthrough or projecting Word without a behavioral probe fails the
suite.

It is listed here rather than given an ID because assigning one is an owner's
call: the conformance corpus in `tests/conformance/`, not a requirement row, is
what `PORTABILITY.md` makes the definition of Ajisai's behavior.

## Unassigned requirement IDs

Requirement IDs are not dense. `AQ-REQ-005` and `AQ-REQ-006` are unused: no
evidence survives of what they were meant to be, so they are left unassigned
rather than backfilled with something invented. Nothing in the source may claim
them until someone decides what they are.

## Retired verification IDs

These were live and are not, so that finding them in history is not confusing.
Each went when the code it was the sole verification of went — a verification
ID outlives its subject only as a dangling claim.

| ID | Subject | Retired |
|---|---|---|
| `AQ-VER-003-A` | `resolve_effective_hint` (arena → JS hint resolution) | with the arena value representation |
| `AQ-VER-003-B` | `build_bracket_structure_from_shape` (`#[cfg(test)]` bracket rendering) | with the arena value representation |
| `AQ-VER-007-E` | `is_safe_preview_word` decision truth table | with that function, whose only caller was a WASM export nothing called |

`AQ-VER-003-C` keeps its letter rather than being renumbered: the letters are
identifiers, not an ordering, and renaming a live ID to close a gap would break
every citation of it for no gain.

## Keeping this true

`scripts/check-traceability-matrix.mjs` (`npm run check:traceability`, run in
CI) enforces both directions this document can drift in:

- every `AQ-VER-*` / `AQ-REQ-*` ID in the source has a row here, so adding a
  suite without recording it fails;
- every live ID and every file path this document names exists in the tree, so
  a row cannot outlive its subject — and an ID listed above as retired or
  unassigned must stay absent from the source.

The gate exists because that drift had already happened before it was written:
the three retired IDs above went with their subjects in the two commits before
this one, and nothing noticed. A name-reachability check cannot tell whether a
row *describes* its suite correctly, only whether both ends still exist; the
statements themselves are a reviewer's job.
