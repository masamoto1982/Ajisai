// js/gui/gui-application.ts

import { createDisplay, Display, DisplayElements } from './display';
import { createVocabularyManager, VocabularyManager, VocabularyElements } from './dictionary';
import { createEditor, Editor } from './editor';
import { createMobileHandler, MobileHandler, MobileElements, ViewMode } from './mobile';
import { createModuleTabManager, ModuleTabManager } from './module-tabs';
import { createPersistence, Persistence } from './persistence';
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
    readonly jsonImportBtn: HTMLButtonElement;
    readonly outputDisplay: HTMLElement;
    readonly stackDisplay: HTMLElement;
    readonly builtInWordsDisplay: HTMLElement;
    readonly customWordsDisplay: HTMLElement;
    readonly builtInWordInfo: HTMLElement;
    readonly customWordInfo: HTMLElement;
    readonly dictionarySearch: HTMLInputElement;
    readonly dictionarySearchClearBtn: HTMLButtonElement;
    readonly dictionarySheetSwitcher: HTMLElement;
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly dictionaryArea: HTMLElement;
    readonly editorPanel: HTMLElement;
    readonly statePanel: HTMLElement;
    readonly leftPaneInputButton: HTMLButtonElement;
    readonly leftPaneOutputButton: HTMLButtonElement;
    readonly rightPaneStackButton: HTMLButtonElement;
    readonly rightPaneDictionaryButton: HTMLButtonElement;
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
    jsonImportBtn: document.getElementById('json-import-btn') as HTMLButtonElement,
    outputDisplay: document.getElementById('output-display')!,
    stackDisplay: document.getElementById('stack-display')!,
    builtInWordsDisplay: document.getElementById('core-words-display')!,
    customWordsDisplay: document.getElementById('custom-words-display')!,
    builtInWordInfo: document.getElementById('core-word-info')!,
    customWordInfo: document.getElementById('custom-word-info')!,
    dictionarySearch: document.getElementById('dictionary-search') as HTMLInputElement,
    dictionarySearchClearBtn: document.getElementById('dictionary-search-clear-btn') as HTMLButtonElement,
    dictionarySheetSwitcher: document.querySelector('.dictionary-sheet-switcher')!,
    inputArea: document.querySelector('.input-area')!,
    outputArea: document.querySelector('.output-area')!,
    stackArea: document.querySelector('.stack-area')!,
    dictionaryArea: document.getElementById('dictionary-panel')!,
    editorPanel: document.getElementById('editor-panel')!,
    statePanel: document.getElementById('state-panel')!,
    leftPaneInputButton: document.getElementById('panel-switch-input') as HTMLButtonElement,
    leftPaneOutputButton: document.getElementById('panel-switch-output') as HTMLButtonElement,
    rightPaneStackButton: document.getElementById('panel-switch-stack') as HTMLButtonElement,
    rightPaneDictionaryButton: document.getElementById('panel-switch-dictionary') as HTMLButtonElement
});

const extractDisplayElements = (elements: GUIElements): DisplayElements => ({
    outputDisplay: elements.outputDisplay,
    stackDisplay: elements.stackDisplay
});

const extractVocabularyElements = (elements: GUIElements): VocabularyElements => ({
    builtInWordsDisplay: elements.builtInWordsDisplay,
    customWordsDisplay: elements.customWordsDisplay,
    builtInWordInfo: elements.builtInWordInfo,
    customWordInfo: elements.customWordInfo
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

type LeftPaneView = 'input' | 'output';
type RightPaneView = 'stack' | 'dictionary';
type DictionarySheetView = string;

const MOBILE_VIEW_MODES: ViewMode[] = ['input', 'output', 'stack', 'dictionary'];
const LEFT_PANE_VIEWS: LeftPaneView[] = ['input', 'output'];
const RIGHT_PANE_VIEWS: RightPaneView[] = ['stack', 'dictionary'];


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
    const customWords = window.ajisaiInterpreter.collect_custom_words_info().map(word => word[0]);

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
    let activeMobileView: ViewMode = 'input';
    let activeLeftPaneView: LeftPaneView = 'input';
    let activeRightPaneView: RightPaneView = 'stack';
    let activeDictionarySheet: DictionarySheetView = 'core';


    const updateEditorPlaceholder = (): void => {
        if (!elements?.codeInput) return;
        elements.codeInput.placeholder = mobile.isMobile()
            ? MOBILE_EDITOR_PLACEHOLDER
            : DESKTOP_EDITOR_PLACEHOLDER;
    };

    const collectPaneSwitchButtons = (): Record<ViewMode, HTMLButtonElement> => ({
        input: elements.leftPaneInputButton,
        output: elements.leftPaneOutputButton,
        stack: elements.rightPaneStackButton,
        dictionary: elements.rightPaneDictionaryButton
    });

    const updatePaneSwitchState = (activeModes: Set<ViewMode>): void => {
        const switchButtons = collectPaneSwitchButtons();
        MOBILE_VIEW_MODES.forEach((viewMode) => {
            const button = switchButtons[viewMode];
            const isActive = activeModes.has(viewMode);
            button.classList.toggle('active', isActive);
            button.setAttribute('aria-pressed', String(isActive));
        });
    };

    const switchDictionarySheet = (sheetId: string): void => {
        activeDictionarySheet = sheetId;
        const allSheets = elements.dictionaryArea.querySelectorAll('.dictionary-sheet');
        allSheets.forEach(sheet => {
            (sheet as HTMLElement).hidden = true;
            sheet.classList.remove('active');
        });

        const allDictionaryButtons = elements.dictionarySheetSwitcher.querySelectorAll<HTMLButtonElement>('.dictionary-sheet-button');
        allDictionaryButtons.forEach(button => {
            const isActive = button.dataset.sheet === sheetId;
            button.classList.toggle('active', isActive);
            button.setAttribute('aria-pressed', String(isActive));
        });

        const target = document.getElementById(`dictionary-sheet-${sheetId}`);
        if (target) {
            (target as HTMLElement).hidden = false;
            target.classList.add('active');
        }
    };

    const syncDesktopLayout = (): void => {
        elements.editorPanel.style.display = 'flex';
        elements.statePanel.style.display = 'flex';
        elements.editorPanel.style.flex = '1';
        elements.statePanel.style.flex = '1';

        elements.inputArea.hidden = activeLeftPaneView !== 'input';
        elements.outputArea.hidden = activeLeftPaneView !== 'output';
        elements.stackArea.hidden = activeRightPaneView !== 'stack';
        elements.dictionaryArea.hidden = activeRightPaneView !== 'dictionary';
    };

    const checkIsRightPaneView = (mode: ViewMode): mode is RightPaneView =>
        RIGHT_PANE_VIEWS.includes(mode as RightPaneView);

    const updateDesktopPaneViews = (mode: ViewMode): void => {
        if (LEFT_PANE_VIEWS.includes(mode as LeftPaneView)) {
            activeLeftPaneView = mode as LeftPaneView;
        }
        if (checkIsRightPaneView(mode)) {
            activeRightPaneView = mode;
            if (mode === 'dictionary') {
                activeLeftPaneView = 'input';
            }
        }
    };

    const fallbackIfModuleTabRemoved = (): void => {
        // If current dictionary sheet is a module sheet that was removed, fallback to core
        if (activeDictionarySheet?.startsWith('module-') && !moduleTabManager.lookupModuleArea(activeDictionarySheet)) {
            switchDictionarySheet('core');
        }
    };

    const applyAreaState = (mode: ViewMode): void => {
        if (mobile.isMobile()) {
            elements.inputArea.hidden = mode !== 'input';
            elements.outputArea.hidden = mode !== 'output';
            elements.stackArea.hidden = mode !== 'stack';
            elements.dictionaryArea.hidden = mode !== 'dictionary';
            mobile.updateView(mode);
            document.body.dataset.activeArea = mode;
            updatePaneSwitchState(new Set([mode]));
            return;
        }

        updateDesktopPaneViews(mode);
        fallbackIfModuleTabRemoved();
        syncDesktopLayout();
        document.body.dataset.activeArea = activeRightPaneView;
        updatePaneSwitchState(new Set([activeLeftPaneView, activeRightPaneView]));
    };

    const switchVisibleArea = (mode: ViewMode): void => {
        activeMobileView = mode;
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
                if (activeRightPaneView !== 'dictionary' || (mobile.isMobile() && activeMobileView !== 'dictionary')) {
                    switchVisibleArea('dictionary');
                }
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

        const paneSwitchButtons = collectPaneSwitchButtons();
        MOBILE_VIEW_MODES.forEach((mode) => {
            const button = paneSwitchButtons[mode];
            button.addEventListener('click', () => switchVisibleArea(mode));
            button.addEventListener('keydown', (e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    switchVisibleArea(mode);
                }
            });
        });

        elements.dictionarySheetSwitcher.addEventListener('keydown', (event) => {
            const keyboardEvent = event as KeyboardEvent;
            if (keyboardEvent.key !== 'Enter' && keyboardEvent.key !== ' ') return;
            const target = keyboardEvent.target as HTMLElement | null;
            const sheetId = target?.getAttribute('data-sheet');
            if (!sheetId) return;
            keyboardEvent.preventDefault();
            switchDictionarySheet(sheetId);
        });

        elements.testBtn?.addEventListener('click', () => {
            switchVisibleArea('output');
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
        elements.jsonImportBtn?.addEventListener('click', () => persistence.importJsonAsVector());

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
            applyAreaState(activeMobileView);
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
            onModeChange: (mode) => switchVisibleArea(mode)
        });
        display = createDisplay(extractDisplayElements(elements));
        display.init();
        updateEditorPlaceholder();

        moduleTabManager = createModuleTabManager({
            switcherEl: elements.dictionarySheetSwitcher,
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
            onUpdateDisplays: () => updateAllDisplays(),
            onSaveState: () => persistence.saveCurrentState(),
            showInfo: (text: string, append: boolean) => display.renderInfo(text, append)
        });

        persistence = createPersistence({
            showError: (error) => display.renderError(error),
            updateDisplays: updateAllDisplays,
            showInfo: (text, append) => display.renderInfo(text, append)
        });
        await persistence.init();

        editor = createEditor(elements.codeInput, {
            onContentChange: updateHighlights,
            onSwitchToInputMode: () => switchVisibleArea('input'),
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
            updateView: (mode) => switchVisibleArea(mode)
        });

        setupEventListeners();
        vocabulary.renderBuiltInWords();
        updateAllDisplays();
        switchDictionarySheet(activeDictionarySheet);
        switchVisibleArea('input');

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
