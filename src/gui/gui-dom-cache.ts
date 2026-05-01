import type { DisplayElements } from './output-display-renderer';
import type { VocabularyElements } from './vocabulary-state-controller';
import type { MobileElements } from './mobile-view-switcher';

export interface GUIElements {
    readonly codeInput: HTMLTextAreaElement;
    readonly clearBtn: HTMLButtonElement;
    readonly testBtn: HTMLButtonElement;
    readonly exportBtn: HTMLButtonElement;
    readonly importBtn: HTMLButtonElement;
    readonly outputDisplay: HTMLElement;
    readonly stackDisplay: HTMLElement;
    readonly builtInWordsDisplay: HTMLElement;
    readonly userWordsDisplay: HTMLElement;
    readonly builtInWordInfo: HTMLElement;
    readonly userWordInfo: HTMLElement;
    readonly userDictionarySelect: HTMLSelectElement;
    readonly dictionarySearch: HTMLInputElement;
    readonly dictionarySearchClearBtn: HTMLButtonElement;
    readonly dictionarySheetSelect: HTMLSelectElement;
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly dictionaryArea: HTMLElement;
    readonly editorPanel: HTMLElement;
    readonly statePanel: HTMLElement;
    readonly leftPanelSelect: HTMLSelectElement;
    readonly rightPanelSelect: HTMLSelectElement;
    readonly rightPanelDictionarySearch: HTMLElement;
    readonly mobilePanelSelect: HTMLSelectElement;
    readonly mobilePanelDictionarySearch: HTMLElement;
    readonly mobileDictionarySearch: HTMLInputElement;
    readonly mobileDictionarySearchClearBtn: HTMLButtonElement;
    readonly copyOutputBtn: HTMLButtonElement;
}

type ElementConstructor<T extends HTMLElement> = {
    new (...args: unknown[]): T;
    readonly name: string;
};

function requireElementById<T extends HTMLElement>(id: string, expectedConstructor: ElementConstructor<T>): T {
    const element = document.getElementById(id);

    if (!element) {
        throw new Error(`Required GUI element #${id} was not found.`);
    }

    if (!(element instanceof expectedConstructor)) {
        throw new Error(`Required GUI element #${id} has unexpected type: ${element.constructor.name}.`);
    }

    return element as T;
}

function requireElementBySelector<T extends HTMLElement>(selector: string, expectedConstructor: ElementConstructor<T>): T {
    const element = document.querySelector(selector);

    if (!element) {
        throw new Error(`Required GUI element selector '${selector}' was not found.`);
    }

    if (!(element instanceof expectedConstructor)) {
        throw new Error(`Required GUI element selector '${selector}' has unexpected type: ${element.constructor.name}.`);
    }

    return element as T;
}

export const cacheElements = (): GUIElements => ({
    codeInput: requireElementById('code-input', HTMLTextAreaElement),
    clearBtn: requireElementById('clear-btn', HTMLButtonElement),
    testBtn: requireElementById('test-btn', HTMLButtonElement),
    exportBtn: requireElementById('export-btn', HTMLButtonElement),
    importBtn: requireElementById('import-btn', HTMLButtonElement),
    outputDisplay: requireElementById('output-display', HTMLElement),
    stackDisplay: requireElementById('stack-display', HTMLElement),
    builtInWordsDisplay: requireElementById('core-words-display', HTMLElement),
    userWordsDisplay: requireElementById('user-words-display', HTMLElement),
    builtInWordInfo: requireElementById('core-word-info', HTMLElement),
    userWordInfo: requireElementById('user-word-info', HTMLElement),
    userDictionarySelect: requireElementById('user-dictionary-select', HTMLSelectElement),
    dictionarySearch: requireElementById('dictionary-search', HTMLInputElement),
    dictionarySearchClearBtn: requireElementById('dictionary-search-clear-btn', HTMLButtonElement),
    dictionarySheetSelect: requireElementById('dictionary-sheet-select', HTMLSelectElement),
    inputArea: requireElementBySelector('.input-area', HTMLElement),
    outputArea: requireElementBySelector('.output-area', HTMLElement),
    stackArea: requireElementBySelector('.stack-area', HTMLElement),
    dictionaryArea: requireElementById('dictionary-panel', HTMLElement),
    editorPanel: requireElementById('editor-panel', HTMLElement),
    statePanel: requireElementById('state-panel', HTMLElement),
    leftPanelSelect: requireElementById('left-panel-select', HTMLSelectElement),
    rightPanelSelect: requireElementById('right-panel-select', HTMLSelectElement),
    rightPanelDictionarySearch: requireElementById('right-panel-dictionary-search', HTMLElement),
    mobilePanelSelect: requireElementById('mobile-panel-select', HTMLSelectElement),
    mobilePanelDictionarySearch: requireElementById('mobile-panel-dictionary-search', HTMLElement),
    mobileDictionarySearch: requireElementById('mobile-dictionary-search', HTMLInputElement),
    mobileDictionarySearchClearBtn: requireElementById('mobile-dictionary-search-clear-btn', HTMLButtonElement),
    copyOutputBtn: requireElementById('copy-output-btn', HTMLButtonElement)
});

export const extractDisplayElements = (elements: GUIElements): DisplayElements => ({
    outputDisplay: elements.outputDisplay,
    stackDisplay: elements.stackDisplay
});

export const extractVocabularyElements = (elements: GUIElements): VocabularyElements => ({
    builtInWordsDisplay: elements.builtInWordsDisplay,
    userWordsDisplay: elements.userWordsDisplay,
    builtInWordInfo: elements.builtInWordInfo,
    userWordInfo: elements.userWordInfo,
    userDictionarySelect: elements.userDictionarySelect
});

export const extractMobileElements = (elements: GUIElements): MobileElements => ({
    inputArea: elements.inputArea,
    outputArea: elements.outputArea,
    stackArea: elements.stackArea,
    dictionaryArea: elements.dictionaryArea
});
