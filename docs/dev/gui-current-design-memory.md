# Current GUI Design Memory

This note captures the current Ajisai web-playground GUI behavior for reference. The web deployment at https://masamoto1982.github.io/Ajisai/ is a playground; installable/desktop usage is provided via the Tauri wrapper (`src-tauri/`). The runtime boot is split by `src/entry/entry-bootstrap.ts` and platform-specific behaviors are abstracted in `src/platform/`. The web entrypoint is not a PWA and has no service worker.

## Overall structure
- Single-page web GUI (`index.html`) with a two-column main layout.
- Left/editor side can show **Input** or **Output**.
- Right/state side can show **Stack** or **Dictionary**.
- Mobile mode switches to a one-panel-at-a-time selector (`input/output/stack/dictionary`).

## Header/footer and app chrome
- Header includes Ajisai logo, version text, and Test button.
- Footer includes copyright + GitHub link.
- Skip link exists for accessibility (`Skip to main content`).

## Primary interaction model
- Main code entry is a textarea.
- Run via button or `Shift+Enter`.
- Step execution via `Ctrl+Enter`.
- Abort via `Escape`.
- Full reset via `Ctrl+Alt+Enter` (with confirmation dialog).
- Output panel supports copy-to-clipboard.
- Clicking output panel (desktop) toggles focus back to input mode.

## Dictionary UX
- Dictionary panel supports sheet switching:
  - Core words
  - User words
  - Dynamic module sheets
- Search box with clear button filters dictionary words.
- User word sheet supports import/export actions.

## Data/state behavior
- Stack display and dictionary update after execution.
- Stack area has visual highlight modes triggered by code content, scanned as whole whitespace-delimited modifier tokens so the combined forms (`.,,`, `..,,`) and the `;`/`;;` sugar are recognized, and decimals like `.5`/`5.` never false-trigger. Both modifier axes ride on one **background-fill** channel on the operand nodes (a fill rather than a text-color change, so it never competes with the bracket depth-colors that show Vector nesting; the tint is kept very pale so the value's own ink stays legible) — the filled set is the target, the fill color is the consumption fate (a non-operand can never be consumed, so no extra marker is needed):
  - Target axis (which items are filled): `..` (or `;;`) fills every stack item (STAK), otherwise the default `.` (TOP) fills only the top item — "which values are the operands".
  - Consumption axis (the fill color): the default `,` (EAT) is a very pale warm red (operands are removed), `,,` (or `;;`) a very pale teal-green (KEEP — operands are retained) — "what becomes of them". The two tints are separated in lightness as well as hue (`--color-consume-eat` / `--color-consume-keep` in `tokens.css`) so they read apart under color-vision deficiency. The fill uses even padding on all four sides so the value gets equal vertical and horizontal breathing room. The two axes are independent, mirroring SPEC §6.3.
- GUI tracks layout state (`currentMode`, `currentLeftMode`, `currentRightMode`).
- Desktop and mobile layouts sync via selectors and `body[data-active-area]`.

## Stack math view (KaTeX)
- The Stack area renders the canonical protocol strings **by default**; a checkbox labeled **LaTeX** (bottom-right of the Stack area, persisted in `localStorage`) opts into KaTeX-typeset mathematics: rationals as fractions, rank-1/rank-2 numeric vectors and tensors as matrices, `approximate` nodes with a leading approx sign. Keeping the math view opt-in keeps the GUI's standard surface independent of KaTeX (portability).
- The unchecked (standard) mode is deliberately **unnamed**: a checkbox only states what checking it adds, so no name for the default view is needed. Naming the default "Ajisai" was considered and rejected — it would be ambiguous with the language name.
- Fractions with a ten-or-more-digit numerator or denominator render at six significant digits — as a plain decimal for human-scale values (a best rational approximation of sqrt(2) shows as approx 1.41421, not as a ten-digit fraction) and as a power of ten outside 10^-4..10^5 — always with a leading approx sign unless the shortened form is exactly the value. The math node also scroll-contains itself so an oversized rendering never clips at the panel edge.
- **Presentation only.** The canonical display strings (`3/1`, bracketed vectors, the nested continued-fraction form) remain the default rendering and the conformance observation; the math view never changes a value or its protocol string.
- **Structure-driven, never text-scanning.** LaTeX is derived from the structured `Value` protocol (`src/gui/value-latex.ts`, pure and unit-tested); `katex.renderToString` runs only on that generated TeX. The Output area is user text and is deliberately not math-rendered — auto-render-style delimiter scanning is never used in the GUI.
- Values without a faithful flat math reading (strings, NIL, booleans, ragged or rank>=3 data, oversized matrices, text-hinted byte tensors) return `null` and fall back to the canonical text node.
- Lazy continued fractions cross the WASM boundary as display strings (SPEC §12.2), which must not be parsed back, so the math view does not yet show a nested `\cfrac`; that would need a structured partial-quotient field on the protocol first.

## Technical composition
- GUI implemented as modular TS components under `src/gui/`.
- Entry bootstrap (`src/entry/entry-bootstrap.ts`) detects runtime and loads `entry-web.ts` / `entry-tauri.ts`; common startup is in `entry-common.ts`.
- Worker manager used for parallel execution and abort handling.
- Persistence/file I/O go through `src/platform/` adapters:
  - Web: IndexedDB + browser file APIs (`src/platform/web/*`)
  - Tauri: app-data JSON persistence + native dialogs/fs (`src/platform/tauri/*`)
- No service worker / offline-mode integration in the web app entrypoint.
- GUI behavior test cases are authored in `src/gui/gui-interpreter-test-cases.ts` and run through the in-app `Test` button (no separate cargo target).

## Styling profile
- App-specific styling lives in `app-interface.css`.
- Base styling lives in `public/ajisai-base.css`.
- Layout relies on flex containers with explicit panel visibility toggles.
