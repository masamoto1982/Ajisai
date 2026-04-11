# Current GUI design/useability memory (before reset)

This note captures the current Ajisai GUI behavior so the interface can be rebuilt from a blank slate.

## Overall structure
- Single-page web GUI (`index.html`) with a two-column main layout.
- Left/editor side can show **Input** or **Output**.
- Right/state side can show **Stack** or **Dictionary**.
- Mobile mode switches to a one-panel-at-a-time selector (`input/output/stack/dictionary`).

## Header/footer and app chrome
- Header includes Ajisai logo, version text, offline indicator, Reference link, and Test button.
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
- Stack area has visual highlight modes triggered by code content:
  - `..` highlights all stack
  - `.` highlights top (unless all-highlight already active)
- GUI tracks layout state (`currentMode`, `currentLeftMode`, `currentRightMode`).
- Desktop and mobile layouts sync via selectors and `body[data-active-area]`.

## Technical composition of old GUI
- GUI implemented as modular TS components under `js/gui/`.
- Entry point (`js/web-app-entrypoint.ts`) initialized WASM interpreter then GUI.
- Worker manager used for parallel execution and abort handling.
- Persistence module handled user dictionary import/export/state retention.
- Service Worker registration and online/offline status messaging integrated with GUI display.

## Styling profile
- App-specific styling lived in `app-interface.css`.
- Theme variables came from `ajisai-theme.js` and were injected into `:root` at boot.
- Layout relied on flex containers with explicit panel visibility toggles.


## Reset operation scope (this commit)
- Removed legacy GUI entry HTML/CSS (`index.html`, `app-interface.css`).
- Removed GUI boot entry (`js/web-app-entrypoint.ts`).
- Removed legacy GUI module tree (`js/gui/*`).
- Removed GUI-related Rust test file (`rust/tests/gui-interpreter-test-cases.rs`).
