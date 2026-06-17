import { createDisplay, Display } from './output-display-renderer';
import { createVocabularyManager, VocabularyManager } from './vocabulary-state-controller';
import { createEditor, Editor } from './code-input-editor';
import { createMobileHandler, MobileHandler } from './mobile-view-switcher';
import { createModuleTabManager, ModuleTabManager } from './module-selector-sheets';
import { createDictionarySheetSelector } from './dictionary-sheet-selector';
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
    updateHighlights,
    updateEditorPlaceholder,
    applyExecutionAreaState,
    LayoutState,
    ApplyAreaStateDeps
} from './gui-layout-state';
import { switchDictionarySheet } from './gui-dictionary-sheet';
import { bindGuiEvents } from './gui-event-bindings';
import { createGuiLayoutState } from './layout/layout-model';
import { createLayoutController, LayoutController } from './layout/layout-controller';
import { createInterpreterClient } from './interpreter/interpreter-client';

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

const INTERPRETER_CLIENT = createInterpreterClient();

const HIDDEN_AUTOCOMPLETE_ALIASES: ReadonlySet<string> = new Set([
    '+', '-', '*', '/', '=', '<', '>', '<=', '>=',
    '[', ']', '{', '}', '(', ')',
    '.', ',', "'", '"',
]);

// Matches a module import: a single-quoted module name, an optional selector
// vector (for IMPORT-ONLY), and the IMPORT / IMPORT-ONLY word. Deliberately
// does not match UNIMPORT / UNIMPORT-ONLY (no word boundary before IMPORT).
const MODULE_IMPORT_PATTERN = /'([^']+)'\s*(?:\[[^\]]*\]\s*)?(?:IMPORT-ONLY|IMPORT)\b/gi;

// Returns the (upper-cased) name of the last module imported by `code`, or
// null when the code performs no import.
const extractImportedModuleName = (code: string): string | null => {
    MODULE_IMPORT_PATTERN.lastIndex = 0;
    let lastModule: string | null = null;
    let match: RegExpExecArray | null;
    while ((match = MODULE_IMPORT_PATTERN.exec(code)) !== null) {
        lastModule = match[1]!.toUpperCase();
    }
    return lastModule;
};

export interface GUI {
    readonly init: () => Promise<void>;
    readonly updateAllDisplays: (executedCode?: string) => void;
    readonly extractElements: () => GUIElements;
    readonly extractDisplay: () => Display;
    readonly extractEditor: () => Editor;
    readonly extractVocabulary: () => VocabularyManager;
    readonly extractMobile: () => MobileHandler;
    readonly extractPersistence: () => Persistence;
    readonly extractExecutionController: () => ExecutionController;
}

// The full word list only changes when the vocabulary changes (after an
// execution). Without this cache the whole set — including several WASM
// round-trips per imported module — was rebuilt on every keystroke.
let autocompleteWordsCache: string[] | null = null;

const invalidateAutocompleteCache = (): void => {
    autocompleteWordsCache = null;
};

const collectAutocompleteWords = (): string[] => {
    if (autocompleteWordsCache) return autocompleteWordsCache;

    const interpreter = INTERPRETER_CLIENT.getOptional();
    if (!interpreter) return [];

    const coreWordsInfo = INTERPRETER_CLIENT.collectCoreWordsInfo();
    const coreWords: string[] = coreWordsInfo
        .map(word => word[0])
        .filter((w): w is string => w !== undefined && !HIDDEN_AUTOCOMPLETE_ALIASES.has(w));

    const userWordsInfo = INTERPRETER_CLIENT.collectUserWordsInfo();
    const userWords: string[] = userWordsInfo.flatMap(word => [
        word[1],
        `${word[0]}@${word[1]}`
    ]);

    const moduleWords: string[] = [];
    try {
        const importedModules: string[] = INTERPRETER_CLIENT.collectImportedModules();
        for (const moduleName of importedModules) {
            const words = INTERPRETER_CLIENT.collectModuleWordsInfo(moduleName);
            const prefix: string = `${moduleName}@`;
            for (const word of words) {
                const name: string = word[0] ?? '';
                moduleWords.push(name.startsWith(prefix) ? name.slice(prefix.length) : name);
            }
        }
    } catch {  }

    const allWords: Set<string> = new Set([...coreWords, ...userWords, ...moduleWords]);
    autocompleteWordsCache = Array.from(allWords).sort((a: string, b: string) => a.localeCompare(b));
    return autocompleteWordsCache;
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
    let layoutController: LayoutController;

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


    const revealDictionarySheet = (sheetId: string): void => {
        if (layoutState.currentRightMode !== 'dictionary'
            || (mobile.isMobile() && layoutState.currentMode !== 'dictionary')) {
            layoutController.setArea('dictionary');
        }
        elements.dictionarySheetSelect.value = sheetId;
        doSwitchDictionarySheet(sheetId);
    };

    const updateAllDisplays = (executedCode?: string): void => {
        if (!INTERPRETER_CLIENT.getOptional()) return;

        invalidateAutocompleteCache();

        try {
            display.renderStack(INTERPRETER_CLIENT.collectStack());
            vocabulary.updateUserWords(INTERPRETER_CLIENT.collectUserWordsInfo());

            moduleTabManager.syncModuleTabs();

            if (executedCode) {
                // Importing a module (by typed code) switches the right pane to
                // that module's dictionary. Sheets for every module already
                // exist, so this is purely a navigation convenience.
                const importedModule = extractImportedModuleName(executedCode);
                if (importedModule
                    && moduleTabManager.lookupModuleArea(`module-${importedModule}`)) {
                    revealDictionarySheet(`module-${importedModule}`);
                }
            }

            updateHighlights(elements, elements.codeInput.value);
        } catch (error) {
            console.error('Failed to update display:', error);
            display.renderError(new Error('Failed to update display.'));
        }
    };

    // Reference ページの用例から渡されたコードをエディタへ流し込む。
    // 受け渡し形式: <playground-url>#code=<encodeURIComponent したソース>
    // Ruby 公式トップのように、用例をそのまま試せる動線を実現するための入口。
    const applyPlaygroundCodeFromUrl = (): void => {
        const marker = '#code=';
        const hash = window.location.hash;
        if (!hash.startsWith(marker)) return;

        try {
            const code = decodeURIComponent(hash.slice(marker.length));
            if (code.trim().length === 0) return;
            // updateValue は入力モードへの切り替えも兼ねる。
            editor.updateValue(code);
            // 一度流し込んだら URL を綺麗にし、リロード時の再投入を防ぐ。
            window.history.replaceState(null, '', window.location.pathname + window.location.search);
        } catch (error) {
            console.warn('[GUI] Failed to apply playground code from URL:', error);
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

    const init = async (): Promise<void> => {
        console.log('[GUI] Initializing GUI...');

        elements = cacheElements();
        layoutState = createGuiLayoutState();
        mobile = createMobileHandler(extractMobileElements(elements), {
            onModeChange: (mode) => layoutController.setArea(mode)
        });
        display = createDisplay(extractDisplayElements(elements));
        display.init();
        updateEditorPlaceholder(elements, mobile);

        const dictionarySheetSelector = createDictionarySheetSelector(elements.dictionarySheetSelect, {
            onToggleModule: (name: string, active: boolean) => moduleTabManager.toggleModule(name, active),
        });

        moduleTabManager = createModuleTabManager({
            selector: dictionarySheetSelector,
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
            onUpdateDisplays: () => updateAllDisplays(),
            onSaveState: () => persistence.saveCurrentState(),
            showInfo: (text: string, append: boolean) => display.renderInfo(text, append),
            revealSheet: (sheetId: string) => revealDictionarySheet(sheetId),
            moduleActions: {
                IO: [{
                    label: 'JSON',
                    className: 'btn',
                    ariaLabel: 'Import JSON as vector',
                    onClick: () => persistence.importJsonAsVector(),
                }],
            },
        });


        layoutController = createLayoutController({
            state: layoutState,
            elements,
            mobile,
            buildApplyAreaStateDeps,
            syncDictionarySearchVisibility
        });

        persistence = createPersistence({
            showError: (error) => display.renderError(error),
            updateDisplays: updateAllDisplays,
            showInfo: (text, append) => display.renderInfo(text, append)
        });
        await persistence.init();

        editor = createEditor(elements.codeInput, {
            onContentChange: (content) => updateHighlights(elements, content),
            onSwitchToInputMode: () => layoutController.setArea('input'),
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

        executionController = createExecutionController(INTERPRETER_CLIENT.getRequired(), {
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
            updateView: (mode) => layoutController.setArea(mode),
            updateAfterExecution: (changes) => {
                applyExecutionAreaState(buildApplyAreaStateDeps(), changes);
                syncDictionarySearchVisibility();
            }
        });

        bindGuiEvents({
            elements,
            mobile,
            layoutState,
            moduleTabManager,
            vocabulary,
            display,
            editor,
            executionController,
            persistence,
            switchArea: (mode) => layoutController.setArea(mode),
            updateAllDisplays,
            doSwitchDictionarySheet,
            layoutController
        });
        vocabulary.renderBuiltInWords();
        updateAllDisplays();

        const restored = await persistence.loadDatabaseData();
        updateAllDisplays();

        if (restored.activeUserDictionary) {
            vocabulary.setSelectedDictionary(restored.activeUserDictionary);
        }
        if (restored.activeDictionarySheet) {
            const targetSheetEl = document.getElementById(`dictionary-sheet-${restored.activeDictionarySheet}`);
            if (targetSheetEl) {
                elements.dictionarySheetSelect.value = restored.activeDictionarySheet;
                doSwitchDictionarySheet(restored.activeDictionarySheet);
            }
        }

        await initializeWorkers();

        applyPlaygroundCodeFromUrl();

        // Focus the empty editor on load so the caret is visible and invites
        // typing. Skipped on mobile, where programmatic focus would pop the
        // virtual keyboard (and the symbol-assist panel) over the editor.
        if (!mobile.isMobile() && editor.extractValue() === '') {
            editor.focus();
        }

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
