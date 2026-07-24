# Structural Constraint Ledger (design note, non-canonical)

> Canonical semantics live in `SPECIFICATION.html`. This note is Phase 5 of the
> structural-memory-safety roadmap (`structural-memory-safety-roadmap.md`): it
> applies the uploaded instruction *"maximize constraints enforceable by
> structure"* to Ajisai. It is a planning/ledger document; each conversion it
> lists lands with its own tests when implemented.

## The method (from the instruction)

The instruction's thesis: **stop guarding an invariant by convention (a comment,
a doc, a review habit) and move it into structure a machine rejects before the
program runs.** A constraint belongs in *structure* when it is (1) local, (2)
auto-rejectable at a single boundary, (3) declaratively expressible, (4) stable
across judges, and (5) always on the path. What cannot be closed that way —
behavioral compatibility of a replacement (LSP), time/history-dependent rules,
distributed integrity — stays *executable specification* (tests, contracts, the
diagnosable NIL/UNKNOWN/error model), not a false structural guarantee.

Ajisai already embodies this at its core: `#:contract` + `ajisai check`, the
Coreword contract registry (§7.14), and a growing set of consistency tests are
all "reject before run" structure. Phase 5 is therefore not a new paradigm — it
is a **systematic sweep**: inventory Ajisai's invariants, mark each as already
structural or still convention-guarded, and convert the high-scoring
convention-guarded ones, adapting each to the Ajisai boundary that can reject it.

## The ledger

Status: **S** = already structural (compiler/test/registry enforces it);
**C** = convention-guarded (prose/authoring/review only), a conversion candidate.

| # | Invariant | Ajisai boundary | Status | Notes |
|---|-----------|-----------------|--------|-------|
| 1 | Every Coreword declares partiality / nil_policy / mass | registry consistency test | **S** | `coreword_registry` asserts each is a valid value |
| 2 | `stability` agrees with `safety_level` | consistency test | **S** | asserted per `BuiltinSpec` doc comment |
| 3 | Control-directive execution form (`VENT`/`FLOW`) is machine-classified, not prose-parsed | `ExecutionForm` enum + tests | **S** | the codebase's stated pattern: "assert the classification instead of parsing the prose" |
| 4 | Comparison words share one stack-effect prose | `comparison_words_have_uniform_stack_effect` | **S** | per-family text pin |
| 5 | Every authored LOOKUP doc names a real builtin | `every_authored_doc_entry_names_a_real_builtin` | **S** | doc→registry reference check |
| 6 | Opt-in `#:contract` arity/purity/nil/linearity/space checked before run | `ajisai check` | **S** | Phases 1–2 of this roadmap |
| 7 | Runtime cost ceilings fire diagnosably (steps, materialization water level) | `RuntimeLimits`, Phase 3 | **S** | over-budget materialization bubbles |
| 8 | Implementation is `unsafe`-free outside one audited island | `#![deny(unsafe_code)]`, Phase 4 | **S** | compiler-enforced |
| 9 | **Each `hover_syntax` example is a well-formed snippet** | tokenizer consistency test | **S** | landed 5.1; caught the `COND`/`TRANSPOSE` bugs |
| 10 | **Each `hover_syntax` symbol resolves to a real word** | registry-resolution consistency test | **S** | landed 5.2; caught `COMPARE-WITHIN`/`FLOW` metavariables |
| 10b | **Each concrete `hover_syntax` example actually runs** | execution consistency test | **C→S (this increment)** | landed 5.3; schematic snippets (bare-modifier fragments, `...` templates) excluded structurally; see below |
| 11 | **`stack_effect` prose arity matches the machine `mass`** | consistency test parsing prose | **C→S (this increment)** | landed 5.4; parser abstains on anything outside its machine-checkable subset, so no false mismatch; 25 fixed-mass words compared, all agree |
| 12 | **Authored LOOKUP examples run** | example-runner test | **C→S (this increment)** | landed 5.5; caught 3 authored examples that had drifted to the pre-fix COND/COMPARE-WITHIN/DEL forms. Verifying rendered value vs prose `result` is item 12b (needs normalization) |
| 13 | Manifest / lockfile shape is well-formed and consistent | `cli/manifest.rs`, `lockfile.rs` checks | **partial** | deploy/config-shape class of the instruction; audit for gaps |

## What landed in this increment (item 9)

A `hover_syntax` is a runnable example shown to users and AI, but its
well-formedness was guarded only by authoring review. The tokenizer already
rejects unbalanced brackets, unclosed string literals, the display-only `( )`
nested-vector form, and inline `|` COND clauses (which must be one per line), so
a single consistency test — *every non-empty `hover_syntax` tokenizes* — turns
"the example is well-formed" from convention into a build-time guarantee.

Writing the test immediately paid for itself: it caught **two real malformed doc
examples** that review had missed and that would not have run if a user pasted
them into the Playground:

- `COND`: `1 { TRUE | 'y' } { IDLE | 'n' } COND` used inline `|` clauses →
  fixed to the block-pair form `1 { TRUE } { 'y' } { IDLE } { 'n' } COND`
  (verified to run and yield `'y'`).
- `TRANSPOSE`: `[ ( 1 2 ) ( 3 4 ) ] TRANSPOSE` used the display-only `( )` form →
  fixed to input syntax `[ [ 1 2 ] [ 3 4 ] ] TRANSPOSE` (verified to run and
  yield `[ [ 1 3 ] [ 2 4 ] ]`).

Only tokenization is *sound to require of all* snippets, because some are
deliberate fragments (`. +`, `,, +`) that show modifier syntax and would
under-flow if executed.

## What landed in 5.2 (item 10)

A second consistency test requires every `Symbol` a `hover_syntax` names to
resolve, after alias canonicalization, to a Coreword-registry entry — covering
operators (`+`), modifiers (`. ,,`), casts (`>CF`), and `@`-module words
(`MATH@SQRT`), which all canonicalize to registry entries. It catches a doc
example that references a removed or misspelled word, and it forces every example
to be a *concrete* runnable snippet rather than a schematic one.

Writing it exposed two more non-runnable examples that used metavariable
placeholders — `COMPARE-WITHIN`'s `a b 64 COMPARE-WITHIN` and `FLOW`'s
`xs ~ { ... } MAP`. Both are now concrete and verified to run:
`1/3 1/2 64 COMPARE-WITHIN` → `-1/1`, and `[ 1 2 3 ] ~ { [ 2 ] * } MAP` →
`[ 2 4 6 ]`. Every `hover_syntax` in the registry now both tokenizes and names
only real words.

## What landed in 5.3 (item 10b)

A third consistency test executes every *concrete* `hover_syntax` on a fresh
interpreter and requires it to run without a raised error (a Bubble/NIL result
is fine — that is a value, not a failure). "Concrete" excludes the structurally
*schematic* snippets, identified by two unambiguous markers: a snippet that
starts with a bare modifier (`. , .. ,, !` — the modifier words `TOP`/`EAT`/
`STAK`/`KEEP`/`FORC` demo their own syntax operand-lessly) or that contains the
ellipsis `...` ("your code here", e.g. `UNFOLD`, `PRECOMPUTE` — the latter is
definition-time-only). Both markers are structural, so the exclusion needs no
maintained word-list and stays free of false failures.

A whole-corpus probe confirmed the split: exactly nine snippets failed to run,
all in the excluded categories except two that *should* run and were fixed to
concrete, self-contained, verified forms:

- `>CF`: `2 MATH@SQRT >CF` used an unimported module word → `1/3 >CF`
  (→ `( 0 ( 3 ) )`).
- `DEL`: `'WORD' DEL` deleted a non-existent placeholder word →
  `{ [ 1 ] } 'W' DEF 'W' DEL` (defines then deletes; ends with an empty stack).

Every concrete example in the registry now tokenizes (item 9), names only real
words (item 10), and runs (item 10b). Fully de-schematising the remaining
templates (`FORC`, `UNFOLD`, `PRECOMPUTE`) into runnable examples is a possible
follow-up, but each needs care (protected-entry force, a convergent generator, a
definition-time context), so they stay honestly marked schematic for now.

## What landed in 5.4 (item 11)

The human-facing `stack_effect` prose (`[ start end ] -> [ seq ]`) and the
machine `mass` contract (SPEC §13.1, `Fixed { consumes, produces }`) are two
descriptions of one word's arity that could drift. A consistency test now parses
the arity out of the prose and requires it to equal the `mass` for every
`Fixed`-mass word. The prose DSL is regular enough to parse — `LHS -> RHS`, each
side a sequence of `[ … ]` / `{ … }` groups — and the parser **abstains** on
anything outside that subset (variadic `...`, annotations `(…)`, control-
directive prose, multi-arrow), so it never raises a false mismatch: it fires
only when the two descriptions provably disagree. Of the 98 built-ins, 73 have a
`Dynamic` mass (no fixed arity to check) and 25 are `Fixed`; all 25 parse and all
agree today, so the check locks the invariant against future drift. A coverage
guard (`compared >= 20`) keeps the check from silently going vacuous if the
parser ever regresses into abstaining.

## What landed in 5.5 (item 12)

The authored LOOKUP examples (`builtin_word_lookup_docs.rs`) carry a runnable
`code` and an expected `result`, but the `code` was only ever *rendered* into
docs, never executed — so it could drift or break unseen. A test now runs every
authored example on a fresh interpreter and requires it to execute without a
channel error, extending the item-10b guarantee to the authored corpus.

It confirmed the drift risk is real: three authored examples still carried the
*pre-fix* forms that items 9/10/10b corrected in `hover_syntax` — `COND`'s inline
`|` clauses, `COMPARE-WITHIN`'s `a b` metavariables, and `DEL`'s `'WORD'`
placeholder — because the authored examples are a separate copy. All three are
now concrete and verified. Verifying the *rendered value* against the prose
`result` ("Pushes [ 1 2 3 ].") is item 12b; the prose is free-form, so it needs
a normalization pass to stay sound.

## Sequencing

Convert in ledger order of score (locality × single-boundary auto-reject),
cheapest-sound-first: item 9 (done) → item 10 (symbol resolution) → item 11
(stack-effect arity, carefully) → item 12/13 (example runner, manifest shape).
Each stays within the "never a false failure" discipline the rest of the
`#:contract` tooling already follows.
