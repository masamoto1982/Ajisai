// js/gui/gui-application.ts

import { createDisplay, Display, DisplayElements } from './output-display-renderer';
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
    readonly customWordsDisplay: HTMLElement;
    readonly builtInWordInfo: HTMLElement;
    readonly customWordInfo: HTMLElement;
    readonly customDictionarySelect: HTMLSelectElement;
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
    customWordsDisplay: document.getElementById('custom-words-display')!,
    builtInWordInfo: document.getElementById('core-word-info')!,
    customWordInfo: document.getElementById('custom-word-info')!,
    customDictionarySelect: document.getElementById('custom-dictionary-select') as HTMLSelectElement,
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
    mobilePanelSelect: document.getElementById('mobile-panel-select') as HTMLSelectElement
});

const extractDisplayElements = (elements: GUIElements): DisplayElements => ({
    outputDisplay: elements.outputDisplay,
    stackDisplay: elements.stackDisplay
});

const extractVocabularyElements = (elements: GUIElements): VocabularyElements => ({
    builtInWordsDisplay: elements.builtInWordsDisplay,
    customWordsDisplay: elements.customWordsDisplay,
    builtInWordInfo: elements.builtInWordInfo,
    customWordInfo: elements.customWordInfo,
    customDictionarySelect: elements.customDictionarySelect
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

    const coreWords = window.ajisaiInterpreter.collect_core_words_info().map(word => word[0]);
    const customWords = window.ajisaiInterpreter.collect_custom_words_info().flatMap(word => [
        word[1],
        `${word[0]}::${word[1]}`
    ]);

    const moduleWords: string[] = [];
    try {
        const importedModules = window.ajisaiInterpreter.collect_imported_modules();
        for (const moduleName of importedModules) {
            const words = window.ajisaiInterpreter.collect_module_words_info(moduleName);
            const prefix = `${moduleName}::`;
            for (const word of words) {
                const name = word[0];
                moduleWords.push(name.startsWith(prefix) ? name.slice(prefix.length) : name);
            }
            // Also include module sample words
            const sampleWords = window.ajisaiInterpreter.collect_module_sample_words_info(moduleName);
            for (const word of sampleWords) {
                moduleWords.push(word[0]);
            }
        }
    } catch { /* no modules imported */ }

    return Array.from(new Set([...coreWords, ...customWords, ...moduleWords])).sort((a, b) => a.localeCompare(b));
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
        // If current dictionary sheet is a module sheet that was removed, fallback to core
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
            display.renderStack(window.ajisaiInterpreter.collect_stack());
            vocabulary.updateCustomWords(window.ajisaiInterpreter.collect_custom_words_info());

            // Sync module sheets based on imported modules
            const newSheetIds = moduleTabManager.syncModuleTabs();

            // Focus the newly imported module's sheet
            if (newSheetIds.length > 0) {
                const lastSheetId = newSheetIds[newSheetIds.length - 1]!;
                // Switch to Dictionary tab if not already there
                if (currentRightMode !== 'dictionary' || (mobile.isMobile() && currentMode !== 'dictionary')) {
                    switchArea('dictionary');
                }
                // Switch to the new module sheet
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

    // デバウンス用ユーティリティ
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

        // 辞書検索: デバウンス付きでフィルタリング
        const applySearchFilter = (filter: string): void => {
            elements.dictionarySearch.value = filter;
            vocabulary.updateSearchFilter(filter);
            moduleTabManager.updateSearchFilter(filter);
        };

        const applySearchInput = debounce(() => {
            applySearchFilter(elements.dictionarySearch.value);
        }, 150);

        elements.dictionarySearch.addEventListener('input', applySearchInput);

        // 検索窓の×ボタンでクリア
        elements.dictionarySearchClearBtn.addEventListener('click', () => {
            applySearchFilter('');
        });

        elements.clearBtn.addEventListener('click', () => {
            editor.clear();
        });

        // Area selectors (desktop: left/right, mobile: single)
        elements.leftPanelSelect.addEventListener('change', () => {
            switchArea(elements.leftPanelSelect.value as ViewMode);
        });
        elements.rightPanelSelect.addEventListener('change', () => {
            switchArea(elements.rightPanelSelect.value as ViewMode);
        });
        elements.mobilePanelSelect.addEventListener('change', () => {
            switchArea(elements.mobilePanelSelect.value as ViewMode);
        });

        // Dictionary sheet selector
        elements.dictionarySheetSelect.addEventListener('change', () => {
            const selectedValue = elements.dictionarySheetSelect.value;
            switchDictionarySheet(selectedValue);
        });

        elements.testBtn?.addEventListener('click', () => {
            switchArea('output');
            import('./gui-test-runner').then(({ createTestRunner }) => {
                const testRunner = createTestRunner({
                    showInfo: (text, append) => display.renderInfo(text, append),
                    showError: (error) => display.renderError(error),
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
                executionController.executeCode(editor.extractValue());
            }
            // Ctrl+Enter: step execution
            if (e.key === 'Enter' && e.ctrlKey && !e.altKey && !e.shiftKey) {
                e.preventDefault();
                executionController.executeStep();
            }
        });

        window.addEventListener('resize', () => {
            applyAreaState(currentMode);
            updateEditorPlaceholder();
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
        mobile = createMobileHandler(extractMobileElements(elements), {
            onModeChange: (mode) => switchArea(mode)
        });
        display = createDisplay(extractDisplayElements(elements));
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
                    className: 'json-import-btn',
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

        // データの読み込みとボタン描画をワーカー初期化より先に行う。
        // ワーカーにはメインスレッドでコンパイル済みのWebAssembly.Moduleを
        // 転送するため、ワーカー側での再コンパイルは発生しない。
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
    checkStackHighlight
};
