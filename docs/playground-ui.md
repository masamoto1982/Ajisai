# The Presentation Profile

A specification for a user interface to Ajisai.

**This document is not part of the language.** It may refer to
`SPECIFICATION.md`; `SPECIFICATION.md` may not refer to it. A headless
implementation that provides none of this is fully conforming
(`SPECIFICATION.md` §14), and no behaviour described here may change the result
of any program.

## Status

There is no playground in this repository at present. The web playground that
existed before 1.0 was built against a language that no longer exists — its
vocabulary, value shapes, and interpreter API are all gone — and it was removed
rather than left to rot against an API it no longer matched.
`docs/migration.md` records this.

This document is the specification a replacement should be written against. The
interpreter is embeddable today: `ajisai-core` is an ordinary Rust library, and
`ajisai` is a CLI with `run`, `eval`, `lint`, `fmt`, `words`, and `repl`.

## The four panels

A playground presents four surfaces.

**Input.** Where source is written. It may offer completion from the vocabulary
manifest (`ajisai words`) and formatting via `ajisai fmt`. Both surfaces must
use the canonical word names the manifest and formatter produce; a playground
must not invent a display name for a word.

**Output.** Diagnostics — errors and lint findings. A lint finding must be
presented as a report, never as a refusal to run: the language's position is
that findings do not block execution, and a UI that greys out the run button on
an advisory has changed the language.

**Stack.** The flow's cross-section, bottom first, rendered as the language
renders it. **A value's rendering is determined by its role**
(`SPECIFICATION.md` §6): a `TEXT` vector shows as `"hi"`, an `INTERVAL` as
`1..3`. A playground must not re-guess a reading the language did not assign —
if the role is `RAW`, the value renders structurally, however number-like or
text-like it looks.

**Dictionary.** The vocabulary: Ajisai Core's words, any registered package's
words, and the user's definitions. Package words should be shown as belonging to
their package, which the manifest records.

## Rules for a playground

1. **Presentation never changes computation.** The same program produces the
   same flow whether it is run in a playground, from the CLI, or from a library
   call. There is no display mode, no rendering option, and no panel state that
   any word can observe.
2. **Rendering is the language's, not the UI's.** Use the renderer; do not
   reimplement it.
3. **A session is a flow.** The flow persists across submissions
   (`SPECIFICATION.md` §5.1). A playground that silently clears the flow between
   runs is showing a different language.
4. **Errors leave the flow where the failing word left it**
   (`SPECIFICATION.md` §5.7). Show it; do not reset it.
5. **Packages are opt-in.** A playground chooses which packages to register and
   must say which it did. Registering `ajisai-music` does not make its words
   part of Ajisai.

## What this document does not do

It does not define screen transitions, panel reachability, selection state,
which panel is shown after execution, whether an empty panel is permitted, or
any other interaction rule. Those are product decisions for whoever builds the
playground, and putting them in a language specification is what made them look
like conformance conditions in the first place.
