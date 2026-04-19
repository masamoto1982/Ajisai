# Current GUI Design Memory

This note captures the current Ajisai web-playground GUI behavior for reference. The web deployment at https://masamoto1982.github.io/Ajisai/ is a playground; installable/desktop usage is provided via the Tauri wrapper (`src-tauri/`). The runtime boot is split by `js/entry/entry-bootstrap.ts` and platform-specific behaviors are abstracted in `js/platform/`. The web entrypoint is not a PWA and has no service worker.

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
- Stack area has visual highlight modes triggered by code content:
  - `..` highlights all stack
  - `.` highlights top (unless all-highlight already active)
- GUI tracks layout state (`currentMode`, `currentLeftMode`, `currentRightMode`).
- Desktop and mobile layouts sync via selectors and `body[data-active-area]`.

## Technical composition
- GUI implemented as modular TS components under `js/gui/`.
- Entry bootstrap (`js/entry/entry-bootstrap.ts`) detects runtime and loads `entry-web.ts` / `entry-tauri.ts`; common startup is in `entry-common.ts`.
- Worker manager used for parallel execution and abort handling.
- Persistence/file I/O go through `js/platform/` adapters:
  - Web: IndexedDB + browser file APIs (`js/platform/web/*`)
  - Tauri: app-data JSON persistence + native dialogs/fs (`js/platform/tauri/*`)
- No service worker / offline-mode integration in the web app entrypoint.
- GUI behavior test cases are authored in `js/gui/gui-interpreter-test-cases.ts` and run through the in-app `Test` button (no separate cargo target).

## Styling profile
- App-specific styling lives in `app-interface.css`.
- Base styling lives in `public/ajisai-base.css`.
- Layout relies on flex containers with explicit panel visibility toggles.
