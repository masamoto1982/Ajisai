import { createDisplay, Display, DisplayElements, StackEditCallback } from './output-display-renderer';
import { createVocabularyManager, VocabularyManager, VocabularyElements } from './vocabulary-state-controller';
import { createEditor, Editor } from './code-input-editor';
import { createMobileHandler, MobileHandler, MobileElements, ViewMode } from './mobile-view-switcher';
import { createModuleTabManager, ModuleTabManager } from './module-selector-sheets';
import { createPersistence, Persistence } from './interpreter-state-persistence';
import { createExecutionController, ExecutionController } from './execution-controller';
import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import type { AjisaiInterpreter } from '../wasm-interpreter-types';

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

export interface GUI {
    readonly init: () => Promise<void>;
    readonly updateAllDisplays: () => void;
    readonly extractElements: () => GUIElements;
    readonly extractDisplay: () => Display;
    readonly extractEditor: () => Editor;
    readonly extractVocabulary: () => VocabularyManager;
    readonly extractMobile: () => MobileHandler;
    readonly extractPersistence: () => Persistence;
    readonly extractExecutionController: () => ExecutionController;
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

const extractDisplayElements = (elements: GUIElements): DisplayElements => ({
    outputDisplay: elements.outputDisplay,
    stackDisplay: elements.stackDisplay
});

const extractVocabularyElements = (elements: GUIElements): VocabularyElements => ({
    builtInWordsDisplay: elements.builtInWordsDisplay,
    userWordsDisplay: elements.userWordsDisplay,
    builtInWordInfo: elements.builtInWordInfo,
    userWordInfo: elements.userWordInfo,
    userDictionarySelect: elements.userDictionarySelect
});

const extractMobileElements = (elements: GUIElements): MobileElements => ({
    inputArea: elements.inputArea,
    outputArea: elements.outputArea,
    stackArea: elements.stackArea,
    dictionaryArea: elements.dictionaryArea
});

const checkStackHighlightAll = (content: string): boolean => /(\s|^)\.\.(\s|$)/.test(content);
const checkStackHighlightTop = (content: string): boolean => /(\s|^)\.(\s|$)/.test(content);

const LEFT_TAB_MODES: ViewMode[] = ['input', 'output'];
const RIGHT_TAB_MODES: ViewMode[] = ['stack', 'dictionary'];


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

const collectAutocompleteWords = (): string[] => {
    if (!window.ajisaiInterpreter) return [];

    const coreWordsInfo = window.ajisaiInterpreter.collect_core_words_info();
    const coreWords: string[] = coreWordsInfo.map(word => word[0]).filter((w): w is string => w !== undefined);

    const userWordsInfo = window.ajisaiInterpreter.collect_user_words_info();
    const userWords: string[] = userWordsInfo.flatMap(word => [
        word[1],
        `${word[0]}@${word[1]}`
    ]);

    const moduleWords: string[] = [];
    try {
        const importedModules: string[] = window.ajisaiInterpreter.collect_imported_modules();
        for (const moduleName of importedModules) {
            const words = window.ajisaiInterpreter.collect_module_words_info(moduleName);
            const prefix: string = `${moduleName}@`;
            for (const word of words) {
                const name: string = word[0] ?? '';
                moduleWords.push(name.startsWith(prefix) ? name.slice(prefix.length) : name);
            }
            const sampleWords = window.ajisaiInterpreter.collect_module_sample_words_info(moduleName);
            for (const word of sampleWords) {
                const sampleName: string = word[0] ?? '';
                moduleWords.push(sampleName);
            }
        }
    } catch { /* modules not imported */ }

    const allWords: Set<string> = new Set([...coreWords, ...userWords, ...moduleWords]);
    return Array.from(allWords).sort((a: string, b: string) => a.localeCompare(b));
};

export const createGUI = (): GUI => {
    let elements: GUIElements;
    let display: Display;
    let editor: Editor;
    let vocabulary: VocabularyManager;
    let mobile: MobileHandler;
    let persistence: Persistence;
    let executionController: ExecutionController;
    let moduleTabManager: ModuleTabManager;
    let currentMode: ViewMode = 'input';
    let currentLeftMode: ViewMode = 'input';
    let currentRightMode: ViewMode = 'stack';


    const updateEditorPlaceholder = (): void => {
        if (!elements?.codeInput) return;
        elements.codeInput.placeholder = mobile.isMobile()
            ? MOBILE_EDITOR_PLACEHOLDER
            : DESKTOP_EDITOR_PLACEHOLDER;
    };

    const syncSelectorState = (leftMode: ViewMode, rightMode: ViewMode): void => {
        elements.leftPanelSelect.value = leftMode;
        elements.rightPanelSelect.value = rightMode;
    };

    const syncMobileSelectorState = (mode: ViewMode): void => {
        elements.mobilePanelSelect.value = mode;
    };

    const switchDictionarySheet = (sheetId: string): void => {
        const allSheets = elements.dictionaryArea.querySelectorAll('.dictionary-sheet');
        allSheets.forEach(sheet => {
            (sheet as HTMLElement).hidden = true;
            sheet.classList.remove('active');
        });

        const target = document.getElementById(`dictionary-sheet-${sheetId}`);
        if (target) {
            target.hidden = false;
            target.classList.add('active');
        }
    };

    const syncDesktopLayout = (): void => {
        elements.editorPanel.hidden = false;
        elements.statePanel.hidden = false;
        elements.inputArea.hidden = currentLeftMode !== 'input';
        elements.outputArea.hidden = currentLeftMode !== 'output';
        elements.stackArea.hidden = currentRightMode !== 'stack';
        elements.dictionaryArea.hidden = currentRightMode !== 'dictionary';
    };

    const isRightMode = (mode: ViewMode): boolean =>
        RIGHT_TAB_MODES.includes(mode);

    const updateDesktopModes = (mode: ViewMode): void => {
        if (LEFT_TAB_MODES.includes(mode)) {
            currentLeftMode = mode;
        }
        if (isRightMode(mode)) {
            currentRightMode = mode;
            if (mode === 'dictionary') {
                currentLeftMode = 'input';
            }
        }
    };

    const fallbackIfModuleTabRemoved = (): void => {
        const currentSheet = elements.dictionarySheetSelect?.value;
        if (currentSheet?.startsWith('module-') && !moduleTabManager.lookupModuleArea(currentSheet)) {
            elements.dictionarySheetSelect.value = 'core';
            switchDictionarySheet('core');
        }
    };

    const applyAreaState = (mode: ViewMode): void => {
        if (mobile.isMobile()) {
            mobile.updateView(mode);
            document.body.dataset.activeArea = mode;
            syncMobileSelectorState(mode);
            return;
        }

        updateDesktopModes(mode);
        fallbackIfModuleTabRemoved();
        syncDesktopLayout();
        document.body.dataset.activeArea = currentRightMode;
        syncSelectorState(currentLeftMode, currentRightMode);
    };

    const switchArea = (mode: ViewMode): void => {
        currentMode = mode;
        applyAreaState(mode);
    };

    const updateHighlights = (content: string): void => {
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

    const updateAllDisplays = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            display.renderStack(window.ajisaiInterpreter.collect_stack());
            vocabulary.updateUserWords(window.ajisaiInterpreter.collect_user_words_info());

            const newSheetIds: string[] = moduleTabManager.syncModuleTabs();

            if (newSheetIds.length > 0) {
                const lastSheetId: string = newSheetIds[newSheetIds.length - 1]!;
                if (currentRightMode !== 'dictionary' || (mobile.isMobile() && currentMode !== 'dictionary')) {
                    switchArea('dictionary');
                }
                elements.dictionarySheetSelect.value = lastSheetId;
                switchDictionarySheet(lastSheetId);
            }

            updateHighlights(elements.codeInput.value);
        } catch (error) {
            console.error('Failed to update display:', error);
            display.renderError(new Error('Failed to update display.'));
        }
    };

    const initializeWorkers = async (): Promise<void> => {
        try {
            display.renderInfo('Initializing...', false);
            await WORKER_MANAGER.init();
            display.renderInfo('Ready', true);
        } catch (error) {
            console.error('[GUI] Failed to initialize workers:', error);
            display.renderError(new Error(`Failed to initialize parallel execution: ${error}`));
        }
    };

    const debounce = <T extends (...args: unknown[]) => void>(
        fn: T,
        delay: number
    ): ((...args: Parameters<T>) => void) => {
        let timeoutId: ReturnType<typeof setTimeout> | null = null;
        return (...args: Parameters<T>) => {
            if (timeoutId) clearTimeout(timeoutId);
            timeoutId = setTimeout(() => fn(...args), delay);
        };
    };

    const setupEventListeners = (): void => {
        elements.runBtn.addEventListener('click', () => {
            executionController.executeCode(editor.extractValue());
        });

        const applySearchFilter = (filter: string): void => {
            elements.dictionarySearch.value = filter;
            vocabulary.updateSearchFilter(filter);
            moduleTabManager.updateSearchFilter(filter);
        };

        const applySearchInput = debounce(() => {
            applySearchFilter(elements.dictionarySearch.value);
        }, 150);

        elements.dictionarySearch.addEventListener('input', applySearchInput);

        elements.dictionarySearchClearBtn.addEventListener('click', () => {
            applySearchFilter('');
        });

        elements.clearBtn.addEventListener('click', () => {
            editor.clear();
        });

        elements.leftPanelSelect.addEventListener('change', () => {
            switchArea(elements.leftPanelSelect.value as ViewMode);
        });
        elements.rightPanelSelect.addEventListener('change', () => {
            switchArea(elements.rightPanelSelect.value as ViewMode);
        });
        elements.mobilePanelSelect.addEventListener('change', () => {
            switchArea(elements.mobilePanelSelect.value as ViewMode);
        });

        elements.dictionarySheetSelect.addEventListener('change', () => {
            const selectedValue = elements.dictionarySheetSelect.value;
            switchDictionarySheet(selectedValue);
        });

        elements.testBtn?.addEventListener('click', async () => {
            switchArea('output');
            const { createTestRunner } = await import('./gui-test-runner');
            const testRunner = createTestRunner({
                showInfo: (text: string, append: boolean) => display.renderInfo(text, append),
                showError: (error: Error | string) => display.renderError(error),
                updateDisplays: updateAllDisplays
            });
            testRunner.runAllTests();
        });

        elements.outputArea.addEventListener('click', (e: MouseEvent) => {
            if ((e.target as HTMLElement).closest('button, a')) return;
            if (!mobile.isMobile() && currentLeftMode === 'output') {
                switchArea('input');
            }
        });

        elements.copyOutputBtn.addEventListener('click', (e: MouseEvent) => {
            e.stopPropagation();
            const text = display.extractState().mainOutput;
            navigator.clipboard.writeText(text).then(() => {
                const btn = elements.copyOutputBtn;
                const original = btn.textContent;
                btn.textContent = 'Copied!';
                setTimeout(() => { btn.textContent = original; }, 1500);
            });
        });

        elements.exportBtn?.addEventListener('click', () => persistence.exportUserWords());
        elements.importBtn?.addEventListener('click', () => persistence.importUserWords());

        elements.codeInput.addEventListener('keydown', (e: KeyboardEvent) => {
            if (e.key === 'Enter' && e.shiftKey) {
                e.preventDefault();
                executionController.executeCode(editor.extractValue());
            }
            if (e.key === 'Enter' && e.ctrlKey && !e.altKey && !e.shiftKey) {
                e.preventDefault();
                executionController.executeStep();
            }
        });

        window.addEventListener('resize', () => {
            applyAreaState(currentMode);
            updateEditorPlaceholder();
        });

        window.addEventListener('keydown', (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                WORKER_MANAGER.abortAll();
                executionController.abortExecution();
                e.preventDefault();
                e.stopImmediatePropagation();
            }
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
        mobile = createMobileHandler(extractMobileElements(elements), {
            onModeChange: (mode) => switchArea(mode)
        });
        const onStackEdit: StackEditCallback = (updatedStack) => {
            if (!window.ajisaiInterpreter) return;
            window.ajisaiInterpreter.restore_stack(updatedStack);
            display.renderStack(window.ajisaiInterpreter.collect_stack());
        };
        display = createDisplay(extractDisplayElements(elements), onStackEdit);
        display.init();
        updateEditorPlaceholder();

        moduleTabManager = createModuleTabManager({
            selectEl: elements.dictionarySheetSelect,
            sheetContainerEl: elements.dictionaryArea,
            onWordClick: (word: string) => {
                if (!mobile.isMobile()) {
                    editor.insertWord(word);
                }
            },
            onBackgroundClick: () => {
                if (!mobile.isMobile()) {
                    editor.insertWord(' ');
                }
            },
            onBackgroundDoubleClick: () => {
                if (!mobile.isMobile()) {
                    editor.removeLastWord();
                }
            },
            onSheetChange: (sheetId: string) => switchDictionarySheet(sheetId),
            onSearchInput: (filter: string) => {
                elements.dictionarySearch.value = filter;
                vocabulary.updateSearchFilter(filter);
                moduleTabManager.updateSearchFilter(filter);
            },
            onUpdateDisplays: () => updateAllDisplays(),
            onSaveState: () => persistence.saveCurrentState(),
            showInfo: (text: string, append: boolean) => display.renderInfo(text, append),
            moduleActions: {
                IO: [{
                    label: 'JSON',
                    className: 'btn-primary',
                    ariaLabel: 'Import JSON as vector',
                    onClick: () => persistence.importJsonAsVector(),
                }],
            },
        });

        persistence = createPersistence({
            showError: (error) => display.renderError(error),
            updateDisplays: updateAllDisplays,
            showInfo: (text, append) => display.renderInfo(text, append)
        });
        await persistence.init();

        editor = createEditor(elements.codeInput, {
            onContentChange: updateHighlights,
            onSwitchToInputMode: () => switchArea('input'),
            onRequestSuggestions: () => collectAutocompleteWords()
        });

        vocabulary = createVocabularyManager(extractVocabularyElements(elements), {
            onWordClick: (word) => {
                if (!mobile.isMobile()) {
                    editor.insertWord(word);
                }
            },
            onBackgroundClick: () => {
                if (!mobile.isMobile()) {
                    editor.insertWord(' ');
                }
            },
            onBackgroundDoubleClick: () => {
                if (!mobile.isMobile()) {
                    editor.removeLastWord();
                }
            },
            onUpdateDisplays: updateAllDisplays,
            onSaveState: () => persistence.saveCurrentState(),
            showInfo: (text, append) => display.renderInfo(text, append)
        });

        executionController = createExecutionController(window.ajisaiInterpreter, {
            extractEditorValue: () => editor.extractValue(),
            clearEditor: (switchView) => { editor.clear(switchView); },
            updateEditorValue: (value) => editor.updateValue(value),
            insertEditorText: (text) => editor.insertText(text),
            showInfo: (text, append) => display.renderInfo(text, append),
            showError: (error) => display.renderError(error),
            showExecutionResult: (result) => display.renderExecutionResult(result),
            updateDisplays: updateAllDisplays,
            saveState: () => persistence.saveCurrentState(),
            fullReset: () => persistence.fullReset(),
            updateView: (mode) => switchArea(mode)
        });

        setupEventListeners();
        vocabulary.renderBuiltInWords();
        updateAllDisplays();
        switchArea('input');

        await persistence.loadDatabaseData();
        updateAllDisplays();
        await initializeWorkers();

        console.log('[GUI] GUI initialization completed');
    };

    const extractElements = (): GUIElements => elements;
    const extractDisplay = (): Display => display;
    const extractEditor = (): Editor => editor;
    const extractVocabulary = (): VocabularyManager => vocabulary;
    const extractMobile = (): MobileHandler => mobile;
    const extractPersistence = (): Persistence => persistence;
    const extractExecutionController = (): ExecutionController => executionController;

    return {
        init,
        updateAllDisplays,
        extractElements,
        extractDisplay,
        extractEditor,
        extractVocabulary,
        extractMobile,
        extractPersistence,
        extractExecutionController
    };
};

export const GUI_INSTANCE = createGUI();

export const guiUtils = {
    cacheElements,
    extractDisplayElements,
    extractVocabularyElements,
    extractMobileElements,
    checkStackHighlightAll,
    checkStackHighlightTop
};
