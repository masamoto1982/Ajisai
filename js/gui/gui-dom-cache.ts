import type { DisplayElements } from './output-display-renderer';
import type { VocabularyElements } from './vocabulary-state-controller';
import type { MobileElements } from './mobile-view-switcher';

export interface GUIElements {
    readonly codeInput: HTMLTextAreaElement;
    readonly runBtn: HTMLButtonElement;
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
    readonly mobilePanelSelect: HTMLSelectElement;
    readonly copyOutputBtn: HTMLButtonElement;
}

export const cacheElements = (root: ParentNode = document): GUIElements => ({
    codeInput: root.querySelector('#code-input') as HTMLTextAreaElement,
    runBtn: root.querySelector('#run-btn') as HTMLButtonElement,
    clearBtn: root.querySelector('#clear-btn') as HTMLButtonElement,
    testBtn: root.querySelector('#test-btn') as HTMLButtonElement,
    exportBtn: root.querySelector('#export-btn') as HTMLButtonElement,
    importBtn: root.querySelector('#import-btn') as HTMLButtonElement,
    outputDisplay: root.querySelector('#output-display')!,
    stackDisplay: root.querySelector('#stack-display')!,
    builtInWordsDisplay: root.querySelector('#core-words-display')!,
    userWordsDisplay: root.querySelector('#user-words-display')!,
    builtInWordInfo: root.querySelector('#core-word-info')!,
    userWordInfo: root.querySelector('#user-word-info')!,
    userDictionarySelect: root.querySelector('#user-dictionary-select') as HTMLSelectElement,
    dictionarySearch: root.querySelector('#dictionary-search') as HTMLInputElement,
    dictionarySearchClearBtn: root.querySelector('#dictionary-search-clear-btn') as HTMLButtonElement,
    dictionarySheetSelect: root.querySelector('#dictionary-sheet-select') as HTMLSelectElement,
    inputArea: root.querySelector('.input-area')!,
    outputArea: root.querySelector('.output-area')!,
    stackArea: root.querySelector('.stack-area')!,
    dictionaryArea: root.querySelector('#dictionary-panel')!,
    editorPanel: root.querySelector('#editor-panel')!,
    statePanel: root.querySelector('#state-panel')!,
    leftPanelSelect: root.querySelector('#left-panel-select') as HTMLSelectElement,
    rightPanelSelect: root.querySelector('#right-panel-select') as HTMLSelectElement,
    mobilePanelSelect: root.querySelector('#mobile-panel-select') as HTMLSelectElement,
    copyOutputBtn: root.querySelector('#copy-output-btn') as HTMLButtonElement
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
