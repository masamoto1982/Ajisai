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
`rust/src/coreword_registry.rs`. The text for AQ-REQ-001 through
AQ-REQ-004 appears nowhere in the repository, so the statements below are
**reconstructed from what each verification suite says it covers** — they
record what is actually verified today, not an original requirement anyone
authored. Treat them as descriptive until an owner replaces them; a
reconstruction that disagrees with intent should be corrected here rather than
worked around in the suites.

## Matrix

| Requirement | Statement | Implementation | Verification | QL |
|---|---|---|---|---|
| **AQ-REQ-001** | Exact rational arithmetic and comparison are correct at every boolean decision in the numeric core, including the big-integer and sign-normalization paths. | `rust/src/types/fraction.rs`, `rust/src/types/fraction_arithmetic.rs`, `rust/src/types/fraction_order.rs`; the integer kernels its reduction calls, `rust/src/types/small_divisor.rs` (remainder and gcd by a one-word divisor) and `rust/src/types/lehmer_gcd.rs` (gcd of wide pairs) | `rust/src/types/fraction_mcdc_tests.rs` — **AQ-VER-001**, MC/DC tables `AQ-VER-001-A` `AQ-VER-001-B` `AQ-VER-001-C` `AQ-VER-001-D` `AQ-VER-001-E` `AQ-VER-001-G` `AQ-VER-001-H` `AQ-VER-001-I`; `rust/src/types/fraction_mcdc_tests/repr.rs` — `AQ-VER-001-J` (128-bit gcd dispatch) `AQ-VER-001-K` (machine-word normalizer) `AQ-VER-001-L` (integer reads) `AQ-VER-001-M` (exponent guard) `AQ-VER-001-N` (mixed-representation order and equality, `rust/src/types/fraction_order.rs`); `rust/src/types/fraction_mcdc_tests/arithmetic.rs` — `AQ-VER-001-O` (machine-integer fast path, `sub`) `AQ-VER-001-P` (`BigInt` sums and Henrici's reduction) `AQ-VER-001-Q` (`mul`/`div` `BigInt` arms and `div`'s guards) `AQ-VER-001-R` (`floor`'s remainder row, `round`, `neg`/`abs`) `AQ-VER-001-S` (`balanced_bigint_gcd` dispatch); the two kernels held equal to `num-bigint`'s operators by property tests, `rust/src/types/small_divisor_tests.rs` and `rust/src/types/lehmer_gcd_tests.rs` | QL-A |
| **AQ-REQ-002** | Tokenization produces the correct token stream: no token-count drift, no wrong token kind, no mis-classified whitespace or comment. | `rust/src/tokenizer.rs` | `rust/src/tokenizer_mcdc_tests.rs` — **AQ-VER-002**, MC/DC tables `AQ-VER-002-A` `AQ-VER-002-D` `AQ-VER-002-E` `AQ-VER-002-F`; `rust/src/tokenizer_number_mcdc_tests.rs` — `AQ-VER-002-G` (numeric grammar past the sign) `AQ-VER-002-H` (structural gate and string boundaries) `AQ-VER-002-I` (the digit count the numeric-literal ceiling reads); regression suites `rust/src/tokenizer_regression_tests.rs`, `rust/src/tokenizer_regression_tests_2.rs` (literals over zero are the nested module in `rust/src/tokenizer_mcdc_tests.rs`) | QL-B |
| **AQ-REQ-003** | The `(Value, hint)` → wire-format decision that crosses the host boundary is correct for every value domain, and the same decision survives the real `wasm-bindgen` glue. | `rust/src/types/value_protocol.rs`; `rust/src/wasm_interpreter_bindings/mod.rs` (mechanical `JsValue` shim over it) | `rust/src/types/value_protocol_tests.rs` — **AQ-VER-003-C**, native MC/DC and property coverage of the mapping; `rust/wasm-tests/tests/boundary.rs` — end-to-end on `wasm32` via `wasm-pack test --node` | QL-A |
| **AQ-REQ-004** | The host runtime kind (web vs Tauri) is classified correctly from the build-time injection and the runtime DOM fallback. | `src/platform/index.ts` (`detectRuntimeKind`) | `src/platform/index.test.ts` — **AQ-VER-004-A**; configuration in `vitest.config.ts`, which cites the suite as **AQ-VER-004** | QL-A |
| **AQ-REQ-007** | Built-in word purity classification is self-consistent with the effects and determinism each Word declares. (It also covered a derived `safe_preview` flag; no feature read the flag and no specification declared it, so it was removed.) | `rust/src/coreword_registry.rs`, projected from `spec/words.json` via `rust/src/kernel/generated/word_registry.rs` | `rust/src/coreword_registry.rs` (its `tests` module) — **AQ-VER-007**, cases **A**, **B**, **B2**, **C**, **D** (run the subset with `cargo test aq_ver_007`) | QL-B |

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
