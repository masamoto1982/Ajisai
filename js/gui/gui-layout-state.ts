import type { ViewMode } from './mobile-view-switcher';
import type { MobileHandler } from './mobile-view-switcher';
import type { GUIElements } from './gui-dom-cache';
import type { ModuleTabManager } from './module-selector-sheets';

export const LEFT_TAB_MODES: ViewMode[] = ['input', 'output'];
export const RIGHT_TAB_MODES: ViewMode[] = ['stack', 'dictionary'];

export const checkStackHighlightAll = (content: string): boolean => /(\s|^)\.\.(\s|$)/.test(content);
export const checkStackHighlightTop = (content: string): boolean => /(\s|^)\.(\s|$)/.test(content);

export const DESKTOP_EDITOR_PLACEHOLDER = [
    'Enter code here',
    '',
    'Run → Shift+Enter',
    'Step → Ctrl+Enter',
    'Abort → Escape',
    'Reset → Ctrl+Alt+Enter',
    'Autocomplete → Ctrl+Space / Tab / ↑↓'
].join('\n');

export const MOBILE_EDITOR_PLACEHOLDER = [
    'Enter code here',
    '',
    'Run → Tap the Run button',
    'Autocomplete → Tap suggestions while typing'
].join('\n');

export interface LayoutState {
    currentMode: ViewMode;
    currentLeftMode: ViewMode;
    currentRightMode: ViewMode;
}

export const createLayoutState = (): LayoutState => ({
    currentMode: 'input',
    currentLeftMode: 'input',
    currentRightMode: 'stack'
});

export const syncSelectorState = (elements: GUIElements, leftMode: ViewMode, rightMode: ViewMode): void => {
    elements.leftPanelSelect.value = leftMode;
    elements.rightPanelSelect.value = rightMode;
};

export const syncMobileSelectorState = (elements: GUIElements, mode: ViewMode): void => {
    elements.mobilePanelSelect.value = mode;
};

export const syncDesktopLayout = (elements: GUIElements, state: LayoutState): void => {
    elements.editorPanel.hidden = false;
    elements.statePanel.hidden = false;
    elements.inputArea.hidden = state.currentLeftMode !== 'input';
    elements.outputArea.hidden = state.currentLeftMode !== 'output';
    elements.stackArea.hidden = state.currentRightMode !== 'stack';
    elements.dictionaryArea.hidden = state.currentRightMode !== 'dictionary';
};

export const updateDesktopModes = (state: LayoutState, mode: ViewMode): void => {
    if (LEFT_TAB_MODES.includes(mode)) {
        state.currentLeftMode = mode;
    }
    if (RIGHT_TAB_MODES.includes(mode)) {
        state.currentRightMode = mode;
        if (mode === 'dictionary') {
            state.currentLeftMode = 'input';
        }
    }
};

export const applyAreaState = (
    elements: GUIElements,
    state: LayoutState,
    mobile: MobileHandler,
    moduleTabManager: ModuleTabManager,
    switchDictionarySheet: (sheetId: string) => void,
    mode: ViewMode
): void => {
    if (mobile.isMobile()) {
        mobile.updateView(mode);
        document.body.dataset.activeArea = mode;
        syncMobileSelectorState(elements, mode);
        return;
    }

    updateDesktopModes(state, mode);

    const currentSheet = elements.dictionarySheetSelect?.value;
    if (currentSheet?.startsWith('module-') && !moduleTabManager.lookupModuleArea(currentSheet)) {
        elements.dictionarySheetSelect.value = 'core';
        switchDictionarySheet('core');
    }

    syncDesktopLayout(elements, state);
    document.body.dataset.activeArea = state.currentRightMode;
    syncSelectorState(elements, state.currentLeftMode, state.currentRightMode);
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
