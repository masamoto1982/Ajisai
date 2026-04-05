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

export const cacheElements = (): GUIElements => ({
    codeInput: document.getElementById('code-input') as HTMLTextAreaElement,
    runBtn: document.getElementById('run-btn') as HTMLButtonElement,
    clearBtn: document.getElementById('clear-btn') as HTMLButtonElement,
    testBtn: document.getElementById('test-btn') as HTMLButtonElement,
    exportBtn: document.getElementById('export-btn') as HTMLButtonElement,
    importBtn: document.getElementById('import-btn') as HTMLButtonElement,
    outputDisplay: document.getElementById('output-display')!,
    stackDisplay: document.getElementById('stack-display')!,
    builtInWordsDisplay: document.getElementById('core-words-display')!,
    userWordsDisplay: document.getElementById('user-words-display')!,
    builtInWordInfo: document.getElementById('core-word-info')!,
    userWordInfo: document.getElementById('user-word-info')!,
    userDictionarySelect: document.getElementById('user-dictionary-select') as HTMLSelectElement,
    dictionarySearch: document.getElementById('dictionary-search') as HTMLInputElement,
    dictionarySearchClearBtn: document.getElementById('dictionary-search-clear-btn') as HTMLButtonElement,
    dictionarySheetSelect: document.getElementById('dictionary-sheet-select') as HTMLSelectElement,
    inputArea: document.querySelector('.input-area')!,
    outputArea: document.querySelector('.output-area')!,
    stackArea: document.querySelector('.stack-area')!,
    dictionaryArea: document.getElementById('dictionary-panel')!,
    editorPanel: document.getElementById('editor-panel')!,
    statePanel: document.getElementById('state-panel')!,
    leftPanelSelect: document.getElementById('left-panel-select') as HTMLSelectElement,
    rightPanelSelect: document.getElementById('right-panel-select') as HTMLSelectElement,
    mobilePanelSelect: document.getElementById('mobile-panel-select') as HTMLSelectElement,
    copyOutputBtn: document.getElementById('copy-output-btn') as HTMLButtonElement
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
