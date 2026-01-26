// js/gui/main.ts

import { createDisplay, Display, DisplayElements } from './display';
import { createDictionary, Dictionary, DictionaryElements } from './dictionary';
import { createEditor, Editor } from './editor';
import { createMobileHandler, MobileHandler, MobileElements } from './mobile';
import { createPersistence, Persistence } from './persistence';
import { createExecutionController, ExecutionController } from './execution-controller';
import { WORKER_MANAGER } from '../workers/worker-manager';
import type { AjisaiInterpreter } from '../wasm-types';

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

export interface GUIElements {
    readonly codeInput: HTMLTextAreaElement;
    readonly runBtn: HTMLButtonElement;
    readonly clearBtn: HTMLButtonElement;
    readonly testBtn: HTMLButtonElement;
    readonly exportBtn: HTMLButtonElement;
    readonly importBtn: HTMLButtonElement;
    readonly outputDisplay: HTMLElement;
    readonly stackDisplay: HTMLElement;
    readonly builtinWordsDisplay: HTMLElement;
    readonly customWordsDisplay: HTMLElement;
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly dictionaryArea: HTMLElement;
}

export interface GUI {
    readonly init: () => Promise<void>;
    readonly updateAllDisplays: () => void;
    readonly getElements: () => GUIElements;
    readonly getDisplay: () => Display;
    readonly getEditor: () => Editor;
    readonly getDictionary: () => Dictionary;
    readonly getMobile: () => MobileHandler;
    readonly getPersistence: () => Persistence;
    readonly getExecutionController: () => ExecutionController;
}

const cacheElements = (): GUIElements => ({
    codeInput: document.getElementById('code-input') as HTMLTextAreaElement,
    runBtn: document.getElementById('run-btn') as HTMLButtonElement,
    clearBtn: document.getElementById('clear-btn') as HTMLButtonElement,
    testBtn: document.getElementById('test-btn') as HTMLButtonElement,
    exportBtn: document.getElementById('export-btn') as HTMLButtonElement,
    importBtn: document.getElementById('import-btn') as HTMLButtonElement,
    outputDisplay: document.getElementById('output-display')!,
    stackDisplay: document.getElementById('stack-display')!,
    builtinWordsDisplay: document.getElementById('builtin-words-display')!,
    customWordsDisplay: document.getElementById('custom-words-display')!,
    inputArea: document.querySelector('.input-area')!,
    outputArea: document.querySelector('.output-area')!,
    stackArea: document.querySelector('.stack-area')!,
    dictionaryArea: document.querySelector('.dictionary-area')!
});

const extractDisplayElements = (elements: GUIElements): DisplayElements => ({
    outputDisplay: elements.outputDisplay,
    stackDisplay: elements.stackDisplay
});

const extractDictionaryElements = (elements: GUIElements): DictionaryElements => ({
    builtinWordsDisplay: elements.builtinWordsDisplay,
    customWordsDisplay: elements.customWordsDisplay
});

const extractMobileElements = (elements: GUIElements): MobileElements => ({
    inputArea: elements.inputArea,
    outputArea: elements.outputArea,
    stackArea: elements.stackArea,
    dictionaryArea: elements.dictionaryArea
});

const checkStackHighlight = (content: string): boolean => {
    const stackRegex = /(\s|^)\.\.(\s|$)/;
    return stackRegex.test(content);
};

export const createGUI = (): GUI => {
    let elements: GUIElements;
    let display: Display;
    let editor: Editor;
    let dictionary: Dictionary;
    let mobile: MobileHandler;
    let persistence: Persistence;
    let executionController: ExecutionController;

    const updateHighlights = (content: string): void => {
        const hasStackWord = checkStackHighlight(content);

        if (hasStackWord) {
            elements.stackDisplay.classList.add('highlight-all');
        } else {
            elements.stackDisplay.classList.remove('highlight-all');
        }
    };

    const updateAllDisplays = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            display.updateStack(window.ajisaiInterpreter.get_stack());
            dictionary.updateCustomWords(window.ajisaiInterpreter.get_custom_words_info());
            updateHighlights(elements.codeInput.value);
        } catch (error) {
            console.error('Failed to update display:', error);
            display.showError(new Error('Failed to update display.'));
        }
    };

    const initializeWorkers = async (): Promise<void> => {
        try {
            display.showInfo('Initializing...', false);
            await WORKER_MANAGER.init();
            display.showInfo('Ready', true);
        } catch (error) {
            console.error('[GUI] Failed to initialize workers:', error);
            display.showError(new Error(`Failed to initialize parallel execution: ${error}`));
        }
    };

    const setupEventListeners = (): void => {
        elements.runBtn.addEventListener('click', () => {
            executionController.runCode(editor.getValue());
        });

        elements.clearBtn.addEventListener('click', () => editor.clear());

        elements.testBtn?.addEventListener('click', () => {
            mobile.updateView('execution');
            import('./test').then(({ createTestRunner }) => {
                const testRunner = createTestRunner({
                    showInfo: (text, append) => display.showInfo(text, append),
                    showError: (error) => display.showError(error),
                    updateDisplays: updateAllDisplays
                });
                testRunner.runAllTests();
            });
        });

        elements.exportBtn?.addEventListener('click', () => persistence.exportCustomWords());
        elements.importBtn?.addEventListener('click', () => persistence.importCustomWords());

        elements.codeInput.addEventListener('keydown', (e) => {
            // Shift+Enter: run
            if (e.key === 'Enter' && e.shiftKey) {
                e.preventDefault();
                executionController.runCode(editor.getValue());
            }
            // Ctrl+Enter: step execution
            if (e.key === 'Enter' && e.ctrlKey && !e.altKey && !e.shiftKey) {
                e.preventDefault();
                executionController.executeStep();
            }
        });

        window.addEventListener('keydown', (e) => {
            // Escape: abort
            if (e.key === 'Escape') {
                WORKER_MANAGER.abortAll();
                executionController.abortExecution();
                e.preventDefault();
                e.stopImmediatePropagation();
            }
            // Ctrl+Alt+Enter: reset
            if (e.key === 'Enter' && e.ctrlKey && e.altKey) {
                if (confirm('Are you sure you want to reset the system?')) {
                    executionController.executeReset();
                }
                e.preventDefault();
                e.stopImmediatePropagation();
            }
        }, true);
    };

    const init = async (): Promise<void> => {
        console.log('[GUI] Initializing GUI...');

        elements = cacheElements();
        mobile = createMobileHandler(extractMobileElements(elements));
        display = createDisplay(extractDisplayElements(elements));
        display.init();

        persistence = createPersistence({
            showError: (error) => display.showError(error),
            updateDisplays: updateAllDisplays,
            showInfo: (text, append) => display.showInfo(text, append)
        });
        await persistence.init();

        editor = createEditor(elements.codeInput, {
            onContentChange: updateHighlights,
            onSwitchToInputMode: () => mobile.updateView('input')
        });

        dictionary = createDictionary(extractDictionaryElements(elements), {
            onWordClick: (word) => editor.insertWord(word),
            onUpdateDisplays: updateAllDisplays,
            onSaveState: () => persistence.saveCurrentState(),
            showInfo: (text, append) => display.showInfo(text, append)
        });

        executionController = createExecutionController(window.ajisaiInterpreter, {
            getEditorValue: () => editor.getValue(),
            clearEditor: (switchView) => editor.clear(switchView),
            setEditorValue: (value) => editor.setValue(value),
            insertEditorText: (text) => editor.insertText(text),
            showInfo: (text, append) => display.showInfo(text, append),
            showError: (error) => display.showError(error),
            showExecutionResult: (result) => display.showExecutionResult(result),
            updateDisplays: updateAllDisplays,
            saveState: () => persistence.saveCurrentState(),
            fullReset: () => persistence.fullReset(),
            updateView: (mode) => mobile.updateView(mode)
        });

        setupEventListeners();
        dictionary.renderBuiltinWords();
        updateAllDisplays();
        mobile.updateView('input');
        await initializeWorkers();
        await persistence.loadDatabaseData();
        updateAllDisplays();

        console.log('[GUI] GUI initialization completed');
    };

    const getElements = (): GUIElements => elements;
    const getDisplay = (): Display => display;
    const getEditor = (): Editor => editor;
    const getDictionary = (): Dictionary => dictionary;
    const getMobile = (): MobileHandler => mobile;
    const getPersistence = (): Persistence => persistence;
    const getExecutionController = (): ExecutionController => executionController;

    return {
        init,
        updateAllDisplays,
        getElements,
        getDisplay,
        getEditor,
        getDictionary,
        getMobile,
        getPersistence,
        getExecutionController
    };
};

export const GUI_INSTANCE = createGUI();

export const guiUtils = {
    cacheElements,
    extractDisplayElements,
    extractDictionaryElements,
    extractMobileElements,
    checkStackHighlight
};
