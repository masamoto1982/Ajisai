import type { ViewMode } from './mobile-view-switcher';
import type { MobileHandler } from './mobile-view-switcher';
import type { GUIElements } from './gui-dom-cache';
import type { ModuleTabManager } from './module-selector-sheets';

const LEFT_TAB_MODES: ViewMode[] = ['input', 'output'];
const RIGHT_TAB_MODES: ViewMode[] = ['stack', 'dictionary'];

// A stack modifier is a maximal run of the modifier characters `.` `,` `;`,
// bounded by whitespace or the start/end of the source. Reading whole tokens
// (rather than scanning for a bare `.` or `..`) lets the highlight recognize
// the combined forms SPEC §6.3 allows — `.,,` (TOP KEEP), `..,,` (STAK KEEP) —
// and the `;` / `;;` sugar (`;` = `. ,` TOP-EAT, `;;` = `.. ,,` STAK-KEEP),
// while never mistaking a decimal such as `.5` or `5.` for a modifier.
const STACK_MODIFIER_TOKEN = /(?:^|\s)([.,;]+)(?=\s|$)/g;

export interface StackModifierState {
    /** STAK target (`..` / `;;`): the whole stack is the operand, not just the top. */
    readonly stak: boolean;
    /** KEEP consumption (`,,` / `;;`): operands are retained rather than eaten. */
    readonly keep: boolean;
}

// Mirror the runtime defaults (SPEC §6.1, §6.2): TOP and EAT. A token reads as
// STAK if it carries `..` or the `;;` sugar, and as KEEP if it carries `,,` or
// `;;`. The two axes are independent (SPEC §6.3), so a program is summarized by
// whether *any* token selects the non-default on each axis — matching the
// existing "any occurrence wins" behavior of the target highlight.
export const analyzeStackModifiers = (content: string): StackModifierState => {
    let stak = false;
    let keep = false;
    for (const match of content.matchAll(STACK_MODIFIER_TOKEN)) {
        const token = match[1] ?? '';
        if (token.includes('..') || token.includes(';;')) stak = true;
        if (token.includes(',,') || token.includes(';;')) keep = true;
    }
    return { stak, keep };
};

// Plain-text placeholder cheat sheet shown in the empty editor. Desktop lists
// keyboard shortcuts; mobile lists the equivalent touch gestures. A non-empty
// placeholder also drives the :placeholder-shown CSS that hides the inline
// clear/format buttons while the field is empty.
const DESKTOP_EDITOR_PLACEHOLDER = [
    'Enter code here',
    '',
    'Run → Shift+Enter',
    'Step → Ctrl+Enter',
    'Format → Shift+Alt+F',
    'Suggestions → Ctrl+Space',
    'Reset → Ctrl+Alt+Enter',
    'Abort → Escape'
].join('\n');

const MOBILE_EDITOR_PLACEHOLDER = [
    'Enter code here',
    '',
    'Run → Triple-tap the editor',
    'Stack → Output → Double-tap Stack area',
    'Output → Editor → Double-tap Output area',
    'Input assist → Tap words below',
    'Autocomplete → Tap suggestions while typing'
].join('\n');

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
    const { stak, keep } = analyzeStackModifiers(content);
    const classes = elements.stackDisplay.classList;

    // Target axis (lemon background): STAK paints every stack item, otherwise the
    // default TOP paints only the top item — answering "which values?".
    classes.toggle('highlight-all', stak);
    classes.toggle('highlight-top', !stak);

    // Consumption axis (border on those same operand nodes): KEEP draws a solid
    // border (operands remain), the default EAT a dashed border (operands are
    // removed) — answering "what becomes of them?". The two channels are
    // independent so any TOP/STAK x EAT/KEEP combination reads at a glance.
    classes.toggle('consume-keep', keep);
    classes.toggle('consume-eat', !keep);

    classes.remove('blink-all');
    classes.remove('blink-top');
};

export const updateEditorPlaceholder = (elements: GUIElements, mobile: MobileHandler): void => {
    if (!elements?.codeInput) return;
    elements.codeInput.placeholder = mobile.isMobile()
        ? MOBILE_EDITOR_PLACEHOLDER
        : DESKTOP_EDITOR_PLACEHOLDER;
};
