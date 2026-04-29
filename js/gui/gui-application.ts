import { createDisplay, Display } from './output-display-renderer';
import { createVocabularyManager, VocabularyManager } from './vocabulary-state-controller';
import { createEditor, Editor } from './code-input-editor';
import { createMobileHandler, MobileHandler, ViewMode } from './mobile-view-switcher';
import { createModuleTabManager, ModuleTabManager } from './module-selector-sheets';
import { createPersistence, Persistence } from './interpreter-state-persistence';
import { createExecutionController, ExecutionController } from './execution-controller';
import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import type { AjisaiInterpreter } from '../wasm-interpreter-types';
import {
    GUIElements,
    cacheElements,
    extractDisplayElements,
    extractVocabularyElements,
    extractMobileElements
} from './gui-dom-cache';
import {
    createLayoutState,
    applyAreaState,
    updateHighlights,
    updateEditorPlaceholder,
    LayoutState,
    ApplyAreaStateDeps
} from './gui-layout-state';
import { switchDictionarySheet } from './gui-dictionary-sheet';

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

export type { GUIElements };

const HIDDEN_AUTOCOMPLETE_ALIASES: ReadonlySet<string> = new Set([
    '+', '-', '*', '/', '=', '<', '>', '<=', '>=',
    '[', ']', '{', '}', '(', ')',
    '.', ',', "'", '"',
]);

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

const collectAutocompleteWords = (): string[] => {
    if (!window.ajisaiInterpreter) return [];

    const coreWordsInfo = window.ajisaiInterpreter.collect_core_words_info();
    const coreWords: string[] = coreWordsInfo
        .map(word => word[0])
        .filter((w): w is string => w !== undefined && !HIDDEN_AUTOCOMPLETE_ALIASES.has(w));

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
    } catch {  }

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
    let layoutState: LayoutState;

    const doSwitchDictionarySheet = (sheetId: string): void => {
        switchDictionarySheet(elements.dictionaryArea, sheetId);
    };

    // The parent selectors (.area-selector-right / .area-selector-mobile) are already
    // hidden at the off-breakpoint by CSS. This JS control is for the in-breakpoint case:
    // within a single breakpoint the selector stays visible across all right/mobile modes,
    // so the search input must be hidden whenever the active mode is not 'dictionary'.
    const syncDictionarySearchVisibility = (): void => {
        const isDesktopDictionary = !mobile.isMobile() && layoutState.currentRightMode === 'dictionary';
        const isMobileDictionary = mobile.isMobile() && layoutState.currentMode === 'dictionary';
        elements.rightPanelDictionarySearch.hidden = !isDesktopDictionary;
        elements.mobilePanelDictionarySearch.hidden = !isMobileDictionary;
    };

    const buildApplyAreaStateDeps = (): ApplyAreaStateDeps => ({
        elements,
        state: layoutState,
        mobile,
        moduleTabManager,
        switchDictionarySheet: doSwitchDictionarySheet,
    });

    const switchArea = (mode: ViewMode): void => {
        layoutState.currentMode = mode;
        applyAreaState(buildApplyAreaStateDeps(), mode);
        syncDictionarySearchVisibility();
    };

    const updateAllDisplays = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            display.renderStack(window.ajisaiInterpreter.collect_stack());
            vocabulary.updateUserWords(window.ajisaiInterpreter.collect_user_words_info());

            const newSheetIds: string[] = moduleTabManager.syncModuleTabs();

            if (newSheetIds.length > 0) {
                const lastSheetId: string = newSheetIds[newSheetIds.length - 1]!;
                if (layoutState.currentRightMode !== 'dictionary' || (mobile.isMobile() && layoutState.currentMode !== 'dictionary')) {
                    switchArea('dictionary');
                }
                elements.dictionarySheetSelect.value = lastSheetId;
                doSwitchDictionarySheet(lastSheetId);
            }

            updateHighlights(elements, elements.codeInput.value);
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

    const setupLayoutEventListeners = (): void => {
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
            doSwitchDictionarySheet(selectedValue);
        });

        const setupDoubleTapToTransition = (
            target: HTMLElement,
            activeMode: ViewMode,
            nextMode: ViewMode
        ): void => {
            const MULTI_TAP_INTERVAL_MS = 500;
            let tapCount = 0;
            let lastTapAt = 0;

            target.addEventListener('click', (e: MouseEvent) => {
                if (!mobile.isMobile()) return;
                if (layoutState.currentMode !== activeMode) return;
                if ((e.target as HTMLElement).closest('button, a')) return;

                const now = Date.now();
                if (now - lastTapAt <= MULTI_TAP_INTERVAL_MS) {
                    tapCount += 1;
                } else {
                    tapCount = 1;
                }
                lastTapAt = now;

                if (tapCount >= 2) {
                    switchArea(nextMode);
                    tapCount = 0;
                    lastTapAt = 0;
                }
            });
        };

        setupDoubleTapToTransition(elements.stackDisplay, 'stack', 'output');
        setupDoubleTapToTransition(elements.outputDisplay, 'output', 'input');

        window.addEventListener('resize', () => {
            applyAreaState(buildApplyAreaStateDeps(), layoutState.currentMode);
            syncDictionarySearchVisibility();
            updateEditorPlaceholder(elements, mobile);
        });
    };

    const setupInteractionEventListeners = (): void => {
        const applySearchFilter = (filter: string): void => {
            elements.dictionarySearch.value = filter;
            elements.mobileDictionarySearch.value = filter;
            vocabulary.updateSearchFilter(filter);
            moduleTabManager.updateSearchFilter(filter);
        };

        const applySearchInput = debounce(() => {
            applySearchFilter(elements.dictionarySearch.value);
        }, 150);

        const applyMobileSearchInput = debounce(() => {
            applySearchFilter(elements.mobileDictionarySearch.value);
        }, 150);

        elements.dictionarySearch.addEventListener('input', applySearchInput);
        elements.mobileDictionarySearch.addEventListener('input', applyMobileSearchInput);

        elements.dictionarySearchClearBtn.addEventListener('click', () => {
            applySearchFilter('');
        });

        elements.mobileDictionarySearchClearBtn.addEventListener('click', () => {
            applySearchFilter('');
        });

        elements.clearBtn.addEventListener('click', () => {
            editor.clear();
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

        elements.outputArea.addEventListener('dblclick', (e: MouseEvent) => {
            if ((e.target as HTMLElement).closest('button, a')) return;
            if (!mobile.isMobile() && layoutState.currentLeftMode === 'output') {
                switchArea('input');
                editor.focus();
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

        {
            const MULTI_TAP_INTERVAL_MS = 500;
            let tapCount = 0;
            let lastTapAt = 0;

            elements.codeInput.addEventListener('touchend', (e: TouchEvent) => {
                if (!mobile.isMobile()) return;
                if (e.changedTouches.length === 0) return;

                const now = Date.now();
                if (now - lastTapAt <= MULTI_TAP_INTERVAL_MS) {
                    tapCount += 1;
                } else {
                    tapCount = 1;
                }

                if (tapCount >= 3) {
                    executionController.executeCode(editor.extractValue());
                    switchArea('stack');
                    tapCount = 0;
                    lastTapAt = 0;
                    return;
                }

                lastTapAt = now;
            }, { passive: true });
        }

        {
            const MULTI_CLICK_INTERVAL_MS = 500;
            let clickCount = 0;
            let lastClickAt = 0;

            elements.codeInput.addEventListener('click', () => {
                if (mobile.isMobile()) return;

                const now = Date.now();
                if (now - lastClickAt <= MULTI_CLICK_INTERVAL_MS) {
                    clickCount += 1;
                } else {
                    clickCount = 1;
                }

                if (clickCount >= 3) {
                    executionController.executeCode(editor.extractValue());
                    clickCount = 0;
                    lastClickAt = 0;
                    return;
                }

                lastClickAt = now;
            });
        }

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
        layoutState = createLayoutState();
        mobile = createMobileHandler(extractMobileElements(elements), {
            onModeChange: (mode) => switchArea(mode)
        });
        display = createDisplay(extractDisplayElements(elements));
        display.init();
        updateEditorPlaceholder(elements, mobile);

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
            onSheetChange: (sheetId: string) => doSwitchDictionarySheet(sheetId),
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

        // Bind layout-only listeners now so panel selectors, swipe, tap, and
        // resize remain responsive while the heavier persistence/worker init
        // continues below. These listeners only depend on `elements`,
        // `layoutState`, `mobile`, `moduleTabManager`, and `display` — all ready.
        setupLayoutEventListeners();

        persistence = createPersistence({
            showError: (error) => display.renderError(error),
            updateDisplays: updateAllDisplays,
            showInfo: (text, append) => display.renderInfo(text, append)
        });
        await persistence.init();

        editor = createEditor(elements.codeInput, {
            onContentChange: (content) => updateHighlights(elements, content),
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

        setupInteractionEventListeners();
        vocabulary.renderBuiltInWords();
        updateAllDisplays();

        await persistence.loadDatabaseData();
        updateAllDisplays();
        await initializeWorkers();

        console.log('[GUI] GUI initialization completed');
    };

    return {
        init,
        updateAllDisplays,
        extractElements: () => elements,
        extractDisplay: () => display,
        extractEditor: () => editor,
        extractVocabulary: () => vocabulary,
        extractMobile: () => mobile,
        extractPersistence: () => persistence,
        extractExecutionController: () => executionController
    };
};

export const GUI_INSTANCE = createGUI();
