// js/gui/main.ts

import { createDisplay, Display, DisplayElements } from './display';
import { createVocabularyManager, VocabularyManager, VocabularyElements } from './dictionary';
import { createEditor, Editor } from './editor';
import { createMobileHandler, MobileHandler, MobileElements, ViewMode } from './mobile';
import { createModuleTabManager, ModuleTabManager } from './module-tabs';
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
    readonly jsonImportBtn: HTMLButtonElement;
    readonly outputDisplay: HTMLElement;
    readonly stackDisplay: HTMLElement;
    readonly builtInWordsDisplay: HTMLElement;
    readonly customWordsDisplay: HTMLElement;
    readonly builtInWordInfo: HTMLElement;
    readonly customWordInfo: HTMLElement;
    readonly builtInWordSearch: HTMLInputElement;
    readonly builtInSearchClearBtn: HTMLButtonElement;
    readonly customWordSearch: HTMLInputElement;
    readonly customSearchClearBtn: HTMLButtonElement;
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly builtInArea: HTMLElement;
    readonly customArea: HTMLElement;
    readonly editorPanel: HTMLElement;
    readonly statePanel: HTMLElement;
    readonly tabInputBtn: HTMLElement;
    readonly tabOutputBtn: HTMLElement;
    readonly tabStackBtn: HTMLElement;
    readonly tabBuiltInBtn: HTMLElement;
    readonly tabCustomBtn: HTMLElement;
}

export interface GUI {
    readonly init: () => Promise<void>;
    readonly updateAllDisplays: () => void;
    readonly getElements: () => GUIElements;
    readonly getDisplay: () => Display;
    readonly getEditor: () => Editor;
    readonly getVocabulary: () => VocabularyManager;
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
    jsonImportBtn: document.getElementById('json-import-btn') as HTMLButtonElement,
    outputDisplay: document.getElementById('output-display')!,
    stackDisplay: document.getElementById('stack-display')!,
    builtInWordsDisplay: document.getElementById('core-words-display')!,
    customWordsDisplay: document.getElementById('idiolect-words-display')!,
    builtInWordInfo: document.getElementById('core-word-info')!,
    customWordInfo: document.getElementById('idiolect-word-info')!,
    builtInWordSearch: document.getElementById('word-search') as HTMLInputElement,
    builtInSearchClearBtn: document.getElementById('search-clear-btn') as HTMLButtonElement,
    customWordSearch: document.getElementById('idiolect-word-search') as HTMLInputElement,
    customSearchClearBtn: document.getElementById('idiolect-search-clear-btn') as HTMLButtonElement,
    inputArea: document.querySelector('.input-area')!,
    outputArea: document.querySelector('.output-area')!,
    stackArea: document.querySelector('.stack-area')!,
    builtInArea: document.getElementById('core-panel')!,
    customArea: document.getElementById('idiolect-panel')!,
    editorPanel: document.getElementById('editor-panel')!,
    statePanel: document.getElementById('state-panel')!,
    tabInputBtn: document.getElementById('tab-input')!,
    tabOutputBtn: document.getElementById('tab-output')!,
    tabStackBtn: document.getElementById('tab-stack')!,
    tabBuiltInBtn: document.getElementById('tab-core')!,
    tabCustomBtn: document.getElementById('tab-idiolect')!
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
    builtInArea: elements.builtInArea,
    customArea: elements.customArea
});

const checkStackHighlight = (content: string): boolean => {
    const stackRegex = /(\s|^)\.\.(\s|$)/;
    return stackRegex.test(content);
};

const TAB_MODES: ViewMode[] = ['input', 'output', 'stack', 'core', 'idiolect'];
const LEFT_TAB_MODES: ViewMode[] = ['input', 'output'];
const RIGHT_TAB_MODES: ViewMode[] = ['stack', 'core', 'idiolect'];


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

const getAutocompleteWords = (): string[] => {
    if (!window.ajisaiInterpreter) return [];

    const coreWords = window.ajisaiInterpreter.get_core_words_info().map(word => word[0]);
    const idiolectWords = window.ajisaiInterpreter.get_idiolect_words_info().map(word => word[0]);

    const moduleWords: string[] = [];
    try {
        const importedModules = window.ajisaiInterpreter.get_imported_modules();
        for (const moduleName of importedModules) {
            const words = window.ajisaiInterpreter.get_module_words_info(moduleName);
            const prefix = `${moduleName}::`;
            for (const word of words) {
                const name = word[0];
                moduleWords.push(name.startsWith(prefix) ? name.slice(prefix.length) : name);
            }
            // Also include module sample words
            const sampleWords = window.ajisaiInterpreter.get_module_sample_words_info(moduleName);
            for (const word of sampleWords) {
                moduleWords.push(word[0]);
            }
        }
    } catch { /* no modules imported */ }

    return Array.from(new Set([...coreWords, ...idiolectWords, ...moduleWords])).sort((a, b) => a.localeCompare(b));
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

    const getTabButtons = (): Record<string, HTMLElement> => ({
        input: elements.tabInputBtn,
        output: elements.tabOutputBtn,
        stack: elements.tabStackBtn,
        core: elements.tabBuiltInBtn,
        idiolect: elements.tabCustomBtn
    });

    const updateTabState = (activeModes: Set<ViewMode>): void => {
        const tabs = getTabButtons();
        TAB_MODES.forEach((key) => {
            const tab = tabs[key]!;
            const isActive = activeModes.has(key);
            tab.classList.toggle('active', isActive);
            tab.setAttribute('aria-selected', String(isActive));
            tab.setAttribute('tabindex', isActive ? '0' : '-1');
        });

        // Update module tab states
        for (const modTab of moduleTabManager.getTabs()) {
            const isActive = activeModes.has(modTab.viewMode);
            modTab.tabBtn.classList.toggle('active', isActive);
            modTab.tabBtn.setAttribute('aria-selected', String(isActive));
            modTab.tabBtn.setAttribute('tabindex', isActive ? '0' : '-1');
        }
    };

    const syncDesktopLayout = (): void => {
        elements.editorPanel.style.display = 'flex';
        elements.statePanel.style.display = 'flex';
        elements.editorPanel.style.flex = '1';
        elements.statePanel.style.flex = '1';

        elements.inputArea.style.display = currentLeftMode === 'input' ? 'flex' : 'none';
        elements.outputArea.style.display = currentLeftMode === 'output' ? 'flex' : 'none';
        elements.stackArea.style.display = currentRightMode === 'stack' ? 'flex' : 'none';
        elements.builtInArea.style.display = currentRightMode === 'core' ? 'flex' : 'none';
        elements.customArea.style.display = currentRightMode === 'idiolect' ? 'flex' : 'none';

        // Module tab areas
        for (const tab of moduleTabManager.getTabs()) {
            const isActive = currentRightMode === tab.viewMode;
            tab.areaEl.style.display = isActive ? 'flex' : 'none';
        }
    };

    const isRightMode = (mode: ViewMode): boolean =>
        RIGHT_TAB_MODES.includes(mode) || mode.startsWith('module:');

    const setDesktopModes = (mode: ViewMode): void => {
        if (LEFT_TAB_MODES.includes(mode)) {
            currentLeftMode = mode;
        }
        if (isRightMode(mode)) {
            currentRightMode = mode;
            // When a non-stack right tab is selected, switch left to Input
            if (mode !== 'stack') {
                currentLeftMode = 'input';
            }
        }
    };

    const fallbackIfModuleTabRemoved = (): void => {
        if (currentRightMode.startsWith('module:') && !moduleTabManager.getModuleArea(currentRightMode)) {
            currentRightMode = 'core';
        }
    };

    const applyAreaState = (mode: ViewMode): void => {
        if (mobile.isMobile()) {
            // For module modes on mobile, show the module area
            if (mode.startsWith('module:')) {
                mobile.updateView(mode);
                // Show the module area
                for (const tab of moduleTabManager.getTabs()) {
                    tab.areaEl.style.display = tab.viewMode === mode ? 'flex' : 'none';
                }
            } else {
                mobile.updateView(mode);
                // Hide all module areas
                for (const tab of moduleTabManager.getTabs()) {
                    tab.areaEl.style.display = 'none';
                }
            }
            document.body.dataset.activeArea = mode;
            updateTabState(new Set([mode]));
            return;
        }

        setDesktopModes(mode);
        fallbackIfModuleTabRemoved();
        syncDesktopLayout();
        document.body.dataset.activeArea = currentRightMode;
        updateTabState(new Set([currentLeftMode, currentRightMode]));
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
            display.updateStack(window.ajisaiInterpreter.get_stack());
            vocabulary.updateCustomWords(window.ajisaiInterpreter.get_idiolect_words_info());

            // Sync module tabs based on imported modules
            const newModules = moduleTabManager.syncModuleTabs();

            // Focus the newly imported module tab
            if (newModules.length > 0) {
                switchArea(newModules[newModules.length - 1]!);
            }

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
            executionController.runCode(editor.getValue());
        });

        // 辞書検索: デバウンス付きでフィルタリング
        const applySearchFilter = (filter: string): void => {
            elements.builtInWordSearch.value = filter;
            elements.customWordSearch.value = filter;
            vocabulary.setSearchFilter(filter);
            moduleTabManager.setSearchFilter(filter);
        };

        const handleCoreSearchInput = debounce(() => {
            applySearchFilter(elements.builtInWordSearch.value);
        }, 150);
        const handleIdiolectSearchInput = debounce(() => {
            applySearchFilter(elements.customWordSearch.value);
        }, 150);

        elements.builtInWordSearch.addEventListener('input', handleCoreSearchInput);
        elements.customWordSearch.addEventListener('input', handleIdiolectSearchInput);

        // 検索窓の×ボタンでクリア
        elements.builtInSearchClearBtn.addEventListener('click', () => {
            applySearchFilter('');
        });
        elements.customSearchClearBtn.addEventListener('click', () => {
            applySearchFilter('');
        });

        elements.clearBtn.addEventListener('click', () => {
            editor.clear();
        });

        const tabs = getTabButtons();
        TAB_MODES.forEach((mode) => {
            const tab = tabs[mode]!;
            tab.addEventListener('click', () => switchArea(mode));
            tab.addEventListener('keydown', (e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    switchArea(mode);
                }
            });
        });

        elements.testBtn?.addEventListener('click', () => {
            switchArea('output');
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
        elements.jsonImportBtn?.addEventListener('click', () => persistence.importJsonAsVector());

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
            tabGroupEl: elements.tabBuiltInBtn.parentElement!,
            areaContainerEl: elements.statePanel,
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
                    editor.deleteLastWord();
                }
            },
            onTabClick: (mode: ViewMode) => switchArea(mode),
            onSearchInput: (filter: string) => {
                elements.builtInWordSearch.value = filter;
                elements.customWordSearch.value = filter;
                vocabulary.setSearchFilter(filter);
                moduleTabManager.setSearchFilter(filter);
            },
            coreTabBtn: elements.tabBuiltInBtn,
            onUpdateDisplays: () => updateAllDisplays(),
            onSaveState: () => persistence.saveCurrentState(),
            showInfo: (text: string, append: boolean) => display.showInfo(text, append)
        });

        persistence = createPersistence({
            showError: (error) => display.showError(error),
            updateDisplays: updateAllDisplays,
            showInfo: (text, append) => display.showInfo(text, append)
        });
        await persistence.init();

        editor = createEditor(elements.codeInput, {
            onContentChange: updateHighlights,
            onSwitchToInputMode: () => switchArea('input'),
            onRequestSuggestions: () => getAutocompleteWords()
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
                    editor.deleteLastWord();
                }
            },
            onUpdateDisplays: updateAllDisplays,
            onSaveState: () => persistence.saveCurrentState(),
            showInfo: (text, append) => display.showInfo(text, append)
        });

        executionController = createExecutionController(window.ajisaiInterpreter, {
            getEditorValue: () => editor.getValue(),
            clearEditor: (switchView) => { editor.clear(switchView); },
            setEditorValue: (value) => editor.setValue(value),
            insertEditorText: (text) => editor.insertText(text),
            showInfo: (text, append) => display.showInfo(text, append),
            showError: (error) => display.showError(error),
            showExecutionResult: (result) => display.showExecutionResult(result),
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

    const getElements = (): GUIElements => elements;
    const getDisplay = (): Display => display;
    const getEditor = (): Editor => editor;
    const getVocabulary = (): VocabularyManager => vocabulary;
    const getMobile = (): MobileHandler => mobile;
    const getPersistence = (): Persistence => persistence;
    const getExecutionController = (): ExecutionController => executionController;

    return {
        init,
        updateAllDisplays,
        getElements,
        getDisplay,
        getEditor,
        getVocabulary,
        getMobile,
        getPersistence,
        getExecutionController
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
