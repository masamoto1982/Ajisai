# The Presentation Profile

The specification of the Ajisai playground.

**This document is not part of the language.** It may refer to
`SPECIFICATION.md`; `SPECIFICATION.md` may not refer to it. A headless
implementation that provides none of this is fully conforming
(`SPECIFICATION.md` §14), and nothing described here changes the result of any
program.

That separation is the point of having this file. Panel reachability, screen
transitions, and idempotent selection were once written into the language
specification as if they were conformance conditions. They are not: they are
this, and they belong here.

## What it is

A single page at `playground/`, built from four files and one WebAssembly
module. No bundler, no framework, no runtime dependency:

```
playground/
  index.html         the four surfaces
  app.css            appearance
  app.js             wiring: elements, keys, gestures
  presentation.js    the transition rules, free of the DOM
  worker.js          the session, in a worker
  ajisai.wasm        built from crates/ajisai-wasm
```

`presentation.js` holds every rule below as a pure function, so the rules are
tested without a browser (`presentation.test.mjs`) and the wiring is tested
with one (`browser.test.mjs`).

## The four surfaces

**Input** — where source is written. Carries the run controls, the word
palette, and the suggestion list.

**Output** — diagnostics: errors, and lint findings. Ajisai Core has no I/O
words, so nothing else appears here.

**Stack** — the flow's cross-section, bottom of the flow at the bottom of the
panel. Rendered exactly as the language renders it: a value's role decides its
appearance (`SPECIFICATION.md` §6), so a `TEXT` vector shows as `"hi"` and an
`INTERVAL` as `1..3`. **The playground never re-guesses a reading the language
did not assign** — a `RAW` vector renders structurally however text-like it
looks.

**Dictionary** — two sheets, Ajisai Core and your own words, with a filter.
Clicking a word puts it in the editor.

## Layout

Above 768px, two columns: Input **or** Output on the left, Stack **or**
Dictionary on the right. At 768px and below, one surface at a time.

## Manual selection

Two coupling rules, and both are about intent rather than tidiness:

- **Choosing Output pulls the right column to Stack.** You asked what a run
  said; the flow is the other half of that answer.
- **Choosing Dictionary pulls the left column to Input.** The reason to open
  the dictionary is to put a word into the editor.

Together they keep Output and Dictionary — whose intents conflict — out of the
reachable configurations. Selection is idempotent, and every surface is
reachable. `presentation.test.mjs` proves all three by exploring the whole
reachable space.

## Execution-driven transition

Where the layout moves after a run is decided by **what the run touched**:

| touched | two columns | one surface |
|---|---|---|
| the flow | right → Stack | Stack |
| diagnostics | left → Output | Output |
| the dictionary | right → Dictionary, on the new word's sheet | Dictionary |
| nothing | stay | stay |

Dictionary outranks Stack: defining a word is the more notable change. A run
that changed nothing observable moves nothing — a program that did nothing
should not rearrange the screen.

A failure always counts as touching diagnostics, so a failing program never
leaves you looking at an unchanged screen.

## Keyboard

| | |
|---|---|
| `Shift+Enter` | run |
| `Ctrl+Enter` | run one source unit, and take it out of the editor |
| `Shift+Alt+F` | format to canonical words |
| `Ctrl+Space` | suggest words by prefix |
| `Ctrl+Alt+Enter` | reset the session, after confirming |
| `Escape` | abort a run; otherwise dismiss suggestions, or return to the editor |

**A step is one source unit** — the same definition `VENT` uses
(`SPECIFICATION.md` §9.2). `TRUE VENT { 1 2 ADD }` steps as `TRUE` and then
`VENT { 1 2 ADD }`, never splitting the vent from the unit it governs, because
a stepper that split them would fail on programs that run perfectly.

## Touch

| | |
|---|---|
| triple-tap the editor | run |
| double-tap the Stack | go to Output |
| double-tap Output | go back to the editor |
| swipe left or right | cycle `input → output → stack → dictionary`, wrapping |
| tap a palette word | insert it |

## The mode hint

The Stack panel tints the cells the armed modes would treat as operands. Two
independent axes, mirroring `SPECIFICATION.md` §8.1:

- **Which cells are filled** — the target. `STAK` (or `:`) anywhere in the
  editor fills every cell; otherwise only the surface cell.
- **The fill colour** — the fate. `KEEP` (or `&`) is a pale teal-green
  (operands are retained); the default `EAT` is a pale warm red (operands are
  consumed). The two tints differ in lightness as well as hue, so they read
  apart without colour vision, and both are fills rather than text colours, so
  they never compete with the value's own ink.

This is a **hint about the source being written, not a simulation**: a mode
governs one word, and the hint summarises which modes the fragment uses. It
reads whole tokens only, so `.5` and `5.` are numbers and never a `TOP`.

## Abort

The interpreter runs in a worker, so a long program never freezes the page and
`Escape` can abort by terminating the worker outright.

The session is then rebuilt by **replaying the journal** — the fragments that
already ran without error. The fragment that was running is not in the journal,
so it is the one thing that does not come back, which is what abort means.

## Rules a playground must not break

1. **Presentation never changes computation.** The same program produces the
   same flow in the playground, from the CLI, and from a library call. There is
   no display mode any word can observe.
2. **Rendering is the language's.** Use the renderer; do not reimplement it.
3. **A session is a flow.** It persists across submissions
   (`SPECIFICATION.md` §5.1).
4. **A lint finding is a report, not a refusal.** Findings never block a run —
   greying out the run button on an advisory would have changed the language.
5. **Errors leave the flow where the failing word left it**
   (`SPECIFICATION.md` §5.7). Show it; do not reset it.
6. **Packages are opt-in.** The playground registers none, so it presents
   Ajisai Core exactly.

## Building and testing

```sh
cargo build -p ajisai-wasm --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/ajisai_wasm.wasm playground/ajisai.wasm

cd playground
npm install          # playwright, for the browser test only
npm test             # rules, WebAssembly boundary, and the real interface
```

`ajisai.wasm` is a build artifact and is not committed. The deployment workflow
builds it.

## What this document does not define

Fonts, spacing, and colour beyond the two meaningful stack tints; which
surface a brand-new visitor sees first; whether there is a footer. Those are
product decisions, and writing them down as if they were requirements is what
made them look like conformance conditions in the first place.
