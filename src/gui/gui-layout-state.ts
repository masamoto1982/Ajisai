// Presentation layer for Ajisai's four observation surfaces (LANG.OBSERVATION.PROJECTIONS:
// Input/π_Input, Output/π_Output, Stack/π_Stack, Dictionary/π_Dict). The
// concrete way those surfaces are made visible on a device is a "Presentation
// Profile" (SPEC Portability Profiles): a labeled transition system over
// visibility configurations. This module holds the initial configuration c0
// (`createLayoutState`), the desktop transition core (`updateDesktopModes`)
// and the execution-driven transition (`applyExecutionAreaState`); the
// single-surface core lives in `mobile-view-switcher`, and the spec invariants
// are checked in `presentation-profile.test.ts`.

import type { ViewMode, MobileHandler } from './mobile-view-switcher';
import type { GUIElements } from './gui-dom-cache';

const LEFT_TAB_MODES: ViewMode[] = ['input', 'output'];
const RIGHT_TAB_MODES: ViewMode[] = ['stack', 'dictionary'];

// Plain-text placeholder cheat sheet shown in the empty editor. Desktop lists
// keyboard shortcuts; mobile lists the equivalent touch gestures. A non-empty
// placeholder also drives the :placeholder-shown CSS that hides the inline
// clear/format buttons while the field is empty.
// Every operation here lives only as a shortcut (or, for the two clears, also
// a button) — none can be typed into the editor and run. That is deliberate:
// the vocabulary holds nothing that throws away the values a program was
// handed, the text it was typed as, or the dictionary it was defined in, and
// the Input surface holds nothing but the program being written.
const DESKTOP_EDITOR_PLACEHOLDER = [
    'Enter code here',
    '',
    'run this code                  → Shift+Enter',
    'run one step at a time         → Ctrl+Enter',
    'format this code               → Shift+Alt+F',
    'look up the word at the cursor → Ctrl+Alt+L',
    '',
    'clear the stack, keep your words   → Ctrl+Alt+S',
    'clear this editor, keep everything → Ctrl+Alt+E',
    // Reset is the one line here a reader acts on expecting an empty
    // dictionary: it clears the stack and the words you defined, then seeds
    // the Example Words back (`fullReset` → `loadExampleWords`). The line says
    // so, so the seeded words do not read as leftovers of your own work.
    'reset: erase the stack and your words, keep the example words → Ctrl+Alt+Enter',
    '',
    'bring back your last program → Ctrl+Up / Ctrl+Down',
    'stop a running step          → Escape',
    '',
    // The dictionary panel writes into this editor, and the space *between* its
    // buttons is a control of its own: a click there types a space, a
    // double-click takes the last word back. Undocumented, it reads as a
    // misfired click on a word button.
    'click a word in the dictionary → write it here',
    'click the space around them    → write a space',
    'double-click that space        → take back the last word'
].join('\n');

// The mobile sheet carries the whole touch vocabulary, because on a phone this
// is the only place it is written down. A bar of labelled buttons would take
// its height out of the editor, and the editor is the thing the page is for.
// Prose in the placeholder costs nothing — the editor is empty whenever it
// shows — so the sheet is where the teaching goes, and it is allowed to be
// long. It scrolls, and its first four lines are the ones a first-time reader
// needs.
//
// Ordered by what a reader reaches for: run it, move between surfaces, fix the
// text, then what types for you, then the stack's own control. The last block
// is the honest one — four operations have a shortcut and no touch control,
// and saying so beats letting someone hunt for a button that is not there.
//
// Every line is kept under 27 characters on purpose. A textarea placeholder
// wraps on width, and a hand-aligned continuation (`'    or the list above'`)
// lands wherever the wrap leaves it, which on a 360px phone turns a tidy
// two-column sheet into ragged prose. Short whole lines wrap nowhere, so the
// sheet reads the same on every phone from 320px up.
const MOBILE_EDITOR_PLACEHOLDER = [
    'Enter code here',
    '',
    'run it → triple-tap here',
    'change surface → swipe',
    '',
    'format → lower-right icon',
    'clear  → × upper right',
    '',
    'tap a symbol below to type',
    'tap a Dictionary word too',
    'two letters → suggestions',
    '',
    'the stack survives reload',
    '× on Stack empties it',
    '',
    'keyboard only, for now:',
    'step    Ctrl+Enter',
    'stop    Escape',
    'look up Ctrl+Alt+L',
    'reset   Ctrl+Alt+Enter'
].join('\n');

/** The one record of which surface is showing; every other view of it is derived. */
export interface LayoutState {
    /** Last mode selected. Shared between desktop and mobile; used to re-apply layout on resize and to drive mobile-only behaviors. */
    currentMode: ViewMode;
    /** Desktop left column state. Always 'input' or 'output'. Mobile does not read this. */
    currentLeftMode: ViewMode;
    /** Desktop right column state. Always 'stack' or 'dictionary'. Mobile does not read this. */
    currentRightMode: ViewMode;
}

/** The initial configuration c0. */
export const createLayoutState = (): LayoutState => ({
    currentMode: 'input',
    currentLeftMode: 'input',
    currentRightMode: 'stack'
});

const syncSelectorState = (elements: GUIElements, leftMode: ViewMode, rightMode: ViewMode): void => {
    elements.leftPanelSelect.value = leftMode;
    elements.rightPanelSelect.value = rightMode;
};

const syncMobileSelectorState = (elements: GUIElements, mode: ViewMode): void => {
    elements.mobilePanelSelect.value = mode;
};

const syncDesktopLayout = (elements: GUIElements, state: LayoutState): void => {
    elements.inputArea.hidden = state.currentLeftMode !== 'input';
    elements.outputArea.hidden = state.currentLeftMode !== 'output';
    elements.stackArea.hidden = state.currentRightMode !== 'stack';
    elements.dictionaryArea.hidden = state.currentRightMode !== 'dictionary';
};

// LANG.OBSERVATION.PROJECTIONS (Observation surfaces) / Portability Profiles "Presentation Profile".
// Pure transition core of the desktop presentation profile: it maps a selection
// of one observation surface onto the (left, right) column configuration. The two
// coupling rules below are the spec's Semantic-coupling invariant (Invariant 6),
// not layout cosmetics — they keep the surfaces that conflict in intent (Output
// vs. Dictionary) out of the reachable configuration space, which is exactly what
// makes the reachable subspace closed under idempotent selection (Invariant 5).
// Exported so the conformance suite (presentation-profile.test.ts) can verify
// the shipped logic is a model of the Presentation Profile LTS.
export const updateDesktopModes = (state: LayoutState, mode: ViewMode): void => {
    if (LEFT_TAB_MODES.includes(mode)) {
        state.currentLeftMode = mode;
        if (mode === 'output') {
            // Running code surfaces Output on the left, so pull the right column to Stack so execution results are immediately visible (Presentation Profile Invariant 6.ii: execution is observable).
            state.currentRightMode = 'stack';
        }
    }
    if (RIGHT_TAB_MODES.includes(mode)) {
        state.currentRightMode = mode;
        if (mode === 'dictionary') {
            // Opening the dictionary returns the left column to Input so that clicked words can be inserted (Presentation Profile Invariant 6.iii: selection feeds editing).
            state.currentLeftMode = 'input';
        }
    }
};

export interface ExecutionSurfaceChanges {
    readonly outputChanged: boolean;
    readonly stackChanged: boolean;
    readonly dictionaryChanged: boolean;
    readonly dictionarySheetId?: string;
}

export interface ApplyAreaStateDeps {
    readonly elements: GUIElements;
    readonly state: LayoutState;
    readonly mobile: MobileHandler;
    /** Select a dictionary sheet in the selector and show it. */
    readonly showDictionarySheet: (sheetId: string) => void;
}

const applyMobileAreaState = (deps: ApplyAreaStateDeps, mode: ViewMode): void => {
    deps.mobile.updateView(mode);
    document.body.dataset.activeArea = mode;
    syncMobileSelectorState(deps.elements, mode);
};

const applyDesktopAreaState = (deps: ApplyAreaStateDeps, mode: ViewMode): void => {
    updateDesktopModes(deps.state, mode);

    syncDesktopLayout(deps.elements, deps.state);
    document.body.dataset.activeArea = deps.state.currentRightMode;
    syncSelectorState(deps.elements, deps.state.currentLeftMode, deps.state.currentRightMode);
};

export const applyAreaState = (deps: ApplyAreaStateDeps, mode: ViewMode): void => {
    if (deps.mobile.isMobile()) {
        applyMobileAreaState(deps, mode);
    } else {
        applyDesktopAreaState(deps, mode);
    }
};

// Execution-driven transition (distinct from the manual-selection core in
// `updateDesktopModes`): the surfaces an execution touched decide where the
// layout moves, per the desktop intent —
//   * Stack changed       → right column shows Stack.
//   * Output changed       → left column shows Output.
//   * both changed         → left=Output, right=Stack.
//   * neither changed      → both columns stay as they were.
//   * Dictionary changed   → right column shows the changed Words sheet
//                            (Dictionary outranks Stack for the right column,
//                            since defining/importing a word is the more
//                            notable structural change).
// The single-surface (mobile) profile cannot show two surfaces at once, so it
// surfaces the single most notable change in the same priority order
// (Dictionary > Output > Stack); when nothing changed it stays put, mirroring
// the desktop "keep both" rule.
export const applyExecutionAreaState = (
    deps: ApplyAreaStateDeps,
    changes: ExecutionSurfaceChanges
): void => {
    if (deps.mobile.isMobile()) {
        let nextMode: ViewMode | null = null;
        if (changes.dictionaryChanged) {
            nextMode = 'dictionary';
        } else if (changes.outputChanged) {
            nextMode = 'output';
        } else if (changes.stackChanged) {
            nextMode = 'stack';
        }
        if (nextMode) {
            if (nextMode === 'dictionary' && changes.dictionarySheetId) {
                deps.showDictionarySheet(changes.dictionarySheetId);
            }
            deps.state.currentMode = nextMode;
            applyMobileAreaState(deps, nextMode);
        }
        return;
    }

    if (changes.outputChanged) {
        deps.state.currentLeftMode = 'output';
    }
    if (changes.stackChanged) {
        deps.state.currentRightMode = 'stack';
    }
    if (changes.dictionaryChanged) {
        deps.state.currentRightMode = 'dictionary';
        if (changes.dictionarySheetId) deps.showDictionarySheet(changes.dictionarySheetId);
    }

    deps.state.currentMode = deps.state.currentRightMode;
    syncDesktopLayout(deps.elements, deps.state);
    document.body.dataset.activeArea = deps.state.currentRightMode;
    syncSelectorState(deps.elements, deps.state.currentLeftMode, deps.state.currentRightMode);
};

export const updateEditorPlaceholder = (elements: GUIElements, mobile: MobileHandler): void => {
    elements.codeInput.placeholder = mobile.isMobile()
        ? MOBILE_EDITOR_PLACEHOLDER
        : DESKTOP_EDITOR_PLACEHOLDER;
};

export type LayoutController = {
    readonly setArea: (mode: ViewMode) => void;
    readonly handleResize: () => void;
};

// `setArea` realizes a Presentation Profile transition (SPEC Portability
// Profiles): selecting one observation surface (LANG.OBSERVATION.PROJECTIONS)
// drives the device-appropriate transition core via `applyAreaState`.
export const createLayoutController = (deps: ApplyAreaStateDeps): LayoutController => ({
    setArea: (mode) => {
        deps.state.currentMode = mode;
        applyAreaState(deps, mode);
    },
    handleResize: () => {
        applyAreaState(deps, deps.state.currentMode);
        updateEditorPlaceholder(deps.elements, deps.mobile);
    }
});
