import type { DisplayElements } from './output-display-renderer';
import type { VocabularyElements } from './vocabulary-state-controller';
import type { MobileElements } from './mobile-view-switcher';

export interface GUIElements {
    readonly codeInput: HTMLTextAreaElement;
    readonly editorClearBtn: HTMLButtonElement;
    readonly stackClearBtn: HTMLButtonElement;
    readonly editorFormatBtn: HTMLButtonElement;
    readonly exportBtn: HTMLButtonElement;
    readonly importBtn: HTMLButtonElement;
    readonly outputDisplay: HTMLElement;
    readonly stackDisplay: HTMLElement;
    readonly builtInWordsDisplay: HTMLElement;
    readonly userWordsDisplay: HTMLElement;
    readonly dictionarySearch: HTMLInputElement;
    readonly dictionarySearchClearBtn: HTMLButtonElement;
    readonly dictionarySheetSelect: HTMLElement;
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly dictionaryArea: HTMLElement;
    readonly leftPanelSelect: HTMLSelectElement;
    readonly rightPanelSelect: HTMLSelectElement;
    readonly mobilePanelSelect: HTMLSelectElement;
    readonly mobileDictionarySearch: HTMLInputElement;
    readonly mobileDictionarySearchClearBtn: HTMLButtonElement;
    readonly copyOutputBtn: HTMLButtonElement;
}

type ElementConstructor<T extends HTMLElement> = {
    new (...args: unknown[]): T;
    readonly name: string;
};

// Every element the GUI binds to is required at startup: a missing one is a
// broken page, reported once here rather than as a null somewhere later.
function requireElement<T extends HTMLElement>(selector: string, expectedConstructor: ElementConstructor<T>): T {
    const element = document.querySelector(selector);
    if (!element) {
        throw new Error(`Required GUI element ${selector} was not found.`);
    }
    if (!(element instanceof expectedConstructor)) {
        throw new Error(`Required GUI element ${selector} has unexpected type: ${element.constructor.name}.`);
    }
    return element;
}

export const cacheElements = (): GUIElements => ({
    codeInput: requireElement('#code-input', HTMLTextAreaElement),
    editorClearBtn: requireElement('#editor-clear-btn', HTMLButtonElement),
    stackClearBtn: requireElement('#stack-clear-btn', HTMLButtonElement),
    editorFormatBtn: requireElement('#editor-format-btn', HTMLButtonElement),
    exportBtn: requireElement('#export-btn', HTMLButtonElement),
    importBtn: requireElement('#import-btn', HTMLButtonElement),
    outputDisplay: requireElement('#output-display', HTMLElement),
    stackDisplay: requireElement('#stack-display', HTMLElement),
    builtInWordsDisplay: requireElement('#core-words-display', HTMLElement),
    userWordsDisplay: requireElement('#user-words-display', HTMLElement),
    dictionarySearch: requireElement('#dictionary-search', HTMLInputElement),
    dictionarySearchClearBtn: requireElement('#dictionary-search-clear-btn', HTMLButtonElement),
    dictionarySheetSelect: requireElement('#dictionary-sheet-select', HTMLElement),
    inputArea: requireElement('.input-area', HTMLElement),
    outputArea: requireElement('.output-area', HTMLElement),
    stackArea: requireElement('.stack-area', HTMLElement),
    dictionaryArea: requireElement('#dictionary-panel', HTMLElement),
    leftPanelSelect: requireElement('#left-panel-select', HTMLSelectElement),
    rightPanelSelect: requireElement('#right-panel-select', HTMLSelectElement),
    mobilePanelSelect: requireElement('#mobile-panel-select', HTMLSelectElement),
    mobileDictionarySearch: requireElement('#mobile-dictionary-search', HTMLInputElement),
    mobileDictionarySearchClearBtn: requireElement('#mobile-dictionary-search-clear-btn', HTMLButtonElement),
    copyOutputBtn: requireElement('#copy-output-btn', HTMLButtonElement)
});

export const extractDisplayElements = (elements: GUIElements): DisplayElements => ({
    outputDisplay: elements.outputDisplay,
    stackDisplay: elements.stackDisplay
});

export const extractVocabularyElements = (elements: GUIElements): VocabularyElements => ({
    builtInWordsDisplay: elements.builtInWordsDisplay,
    userWordsDisplay: elements.userWordsDisplay
});

export const extractMobileElements = (elements: GUIElements): MobileElements => ({
    inputArea: elements.inputArea,
    outputArea: elements.outputArea,
    stackArea: elements.stackArea,
    dictionaryArea: elements.dictionaryArea
});
