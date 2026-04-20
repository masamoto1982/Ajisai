import type { ViewMode } from './mobile-view-switcher';
import type { MobileHandler } from './mobile-view-switcher';
import type { GUIElements } from './gui-dom-cache';
import type { ModuleTabManager } from './module-selector-sheets';

const LEFT_TAB_MODES: ViewMode[] = ['input', 'output'];
const RIGHT_TAB_MODES: ViewMode[] = ['stack', 'dictionary'];

const checkStackHighlightAll = (content: string): boolean => /(\s|^)\.\.(\s|$)/.test(content);
const checkStackHighlightTop = (content: string): boolean => /(\s|^)\.(\s|$)/.test(content);

const DESKTOP_EDITOR_PLACEHOLDER = [
    'Enter code here',
    '',
    'Run → Shift+Enter',
    'Step → Ctrl+Enter',
    'Abort → Escape',
    'Reset → Ctrl+Alt+Enter',
    'Autocomplete → Ctrl+Space / Tab / ↑↓'
].join('\n');

const MOBILE_EDITOR_PLACEHOLDER = [
    'Enter code here',
    '',
    'Run → Tap the Run button',
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

const updateDesktopModes = (state: LayoutState, mode: ViewMode): void => {
    if (LEFT_TAB_MODES.includes(mode)) {
        state.currentLeftMode = mode;
    }
    if (RIGHT_TAB_MODES.includes(mode)) {
        state.currentRightMode = mode;
        if (mode === 'dictionary') {
            // Opening the dictionary returns the left column to Input so that clicked words can be inserted.
            state.currentLeftMode = 'input';
        }
    }
};

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

export const updateEditorPlaceholder = (elements: GUIElements, mobile: MobileHandler): void => {
    if (!elements?.codeInput) return;
    elements.codeInput.placeholder = mobile.isMobile()
        ? MOBILE_EDITOR_PLACEHOLDER
        : DESKTOP_EDITOR_PLACEHOLDER;
};
