import type { ViewMode } from './mobile-view-switcher';
import type { MobileHandler } from './mobile-view-switcher';
import type { GUIElements } from './gui-dom-cache';
import type { ModuleTabManager } from './module-selector-sheets';

const LEFT_TAB_MODES: ViewMode[] = ['input', 'output'];
const RIGHT_TAB_MODES: ViewMode[] = ['stack', 'dictionary'];

const checkStackHighlightAll = (content: string): boolean => /(\s|^)\.\.(\s|$)/.test(content);
const checkStackHighlightTop = (content: string): boolean => /(\s|^)\.(\s|$)/.test(content);

// The editor's shortcut/gesture hints now live in an HTML table overlay
// (#shortcuts-hint in index.html), which a textarea placeholder cannot hold.
// The textarea keeps only a blank placeholder so the :placeholder-shown CSS
// state — which reveals that overlay and hides the inline clear/format buttons
// while the field is empty — keeps working.
const EDITOR_PLACEHOLDER = ' ';

export interface LayoutState {
    /** Last mode passed to `switchArea`. Shared between desktop and mobile; used to re-apply layout on resize and to drive mobile-only behaviors. */
    currentMode: ViewMode;
    /** Desktop left column state. Always 'input' or 'output'. Mobile does not read this. */
    currentLeftMode: ViewMode;
    /** Desktop right column state. Always 'stack' or 'dictionary'. Mobile does not read this. */
    currentRightMode: ViewMode;
}

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

// SPEC §12.3 (Observation surfaces) / Portability Profiles "Presentation Profile".
// Pure transition core of the desktop presentation profile: it maps a selection
// of one observation surface onto the (left, right) column configuration. The two
// coupling rules below are the spec's Semantic-coupling invariant (Invariant 6),
// not layout cosmetics — they keep the surfaces that conflict in intent (Output
// vs. Dictionary) out of the reachable configuration space, which is exactly what
// makes the reachable subspace closed under idempotent selection (Invariant 5).
// Exported so the conformance suite (layout/presentation-profile.test.ts) can
// verify the shipped logic is a model of the Presentation Profile LTS.
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
    readonly moduleTabManager: ModuleTabManager;
    readonly switchDictionarySheet: (sheetId: string) => void;
}

const applyMobileAreaState = (deps: ApplyAreaStateDeps, mode: ViewMode): void => {
    deps.mobile.updateView(mode);
    document.body.dataset.activeArea = mode;
    syncMobileSelectorState(deps.elements, mode);
};

const applyDesktopAreaState = (deps: ApplyAreaStateDeps, mode: ViewMode): void => {
    updateDesktopModes(deps.state, mode);

    const currentSheet = deps.elements.dictionarySheetSelect?.value;
    if (currentSheet?.startsWith('module-') && !deps.moduleTabManager.lookupModuleArea(currentSheet)) {
        deps.elements.dictionarySheetSelect.value = 'core';
        deps.switchDictionarySheet('core');
    }

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

const revealChangedDictionarySheet = (deps: ApplyAreaStateDeps, sheetId?: string): void => {
    if (!sheetId) return;
    deps.elements.dictionarySheetSelect.value = sheetId;
    deps.switchDictionarySheet(sheetId);
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
            if (nextMode === 'dictionary') {
                revealChangedDictionarySheet(deps, changes.dictionarySheetId);
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
        revealChangedDictionarySheet(deps, changes.dictionarySheetId);
    }

    deps.state.currentMode = deps.state.currentRightMode;
    syncDesktopLayout(deps.elements, deps.state);
    document.body.dataset.activeArea = deps.state.currentRightMode;
    syncSelectorState(deps.elements, deps.state.currentLeftMode, deps.state.currentRightMode);
};

export const updateHighlights = (elements: GUIElements, content: string): void => {
    const hasStackAllWord = checkStackHighlightAll(content);
    const hasStackTopWord = checkStackHighlightTop(content) || !hasStackAllWord;

    if (hasStackAllWord) {
        elements.stackDisplay.classList.add('highlight-all');
    } else {
        elements.stackDisplay.classList.remove('highlight-all');
    }

    if (hasStackTopWord && !hasStackAllWord) {
        elements.stackDisplay.classList.add('highlight-top');
    } else {
        elements.stackDisplay.classList.remove('highlight-top');
    }

    elements.stackDisplay.classList.remove('blink-all');
    elements.stackDisplay.classList.remove('blink-top');
};

export const updateEditorPlaceholder = (elements: GUIElements, _mobile: MobileHandler): void => {
    if (!elements?.codeInput) return;
    elements.codeInput.placeholder = EDITOR_PLACEHOLDER;
};
