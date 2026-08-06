import { createDisplay, Display } from './output-display-renderer';
import { createVocabularyManager, formatDictionaryTabName, VocabularyManager } from './vocabulary-state-controller';
import { createEditor, Editor } from './code-input-editor';
import { createMobileHandler, MobileHandler } from './mobile-view-switcher';
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
    '+', '-', '*', '/', '=', '<', '>',
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

// The full word list only changes when the vocabulary changes (after an
// execution). Without this cache the whole set — including a WASM round-trip
// per query — was rebuilt on every keystroke.
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
    // Bare names only: a `DICT@NAME` completion no longer resolves to anything,
    // so suggesting one only offered code that fails to run.
    const userWords: string[] = userWordsInfo.map(word => word[1]);

    const allWords: Set<string> = new Set([...coreWords, ...userWords]);
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
    let layoutState: LayoutState;
    let layoutController: LayoutController;

    const doSwitchDictionarySheet = (sheetId: string): void => {
        switchDictionarySheet(elements.dictionaryArea, sheetId);
    };

    const buildApplyAreaStateDeps = (): ApplyAreaStateDeps => ({
        elements,
        state: layoutState,
        mobile,
        switchDictionarySheet: doSwitchDictionarySheet,
    });


    const updateAllDisplays = (): void => {
        if (!INTERPRETER_CLIENT.getOptional()) return;

        invalidateAutocompleteCache();

        try {
            display.renderStack(INTERPRETER_CLIENT.collectStack());
            vocabulary.updateUserWords(INTERPRETER_CLIENT.collectUserWordsInfo());

            updateHighlights(elements, elements.codeInput.value);
        } catch (error) {
            console.error('Failed to update display:', error);
            display.renderError(new Error('Failed to update display.'));
        }
    };

    // Clearing the stack keeps the dictionary — that is the whole point of
    // having it apart from Reset — so it is the interpreter's `clear_stack` and
    // nothing else, followed by a redraw and a save. One definition for all
    // three routes to it: the Stack area's `×`, `Shift+Alt+C`, and the `CLEAR`
    // host command typed in the editor.
    const clearStack = (): void => {
        const interpreter = INTERPRETER_CLIENT.getOptional();
        if (!interpreter) return;
        interpreter.clear_stack();
        updateAllDisplays();
        display.renderInfo('Stack cleared', false);
        void persistence.saveCurrentState();
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

        // The dictionary has two tiers (LANG.DICTIONARY.RESOLUTION), so the
        // sheet list is fixed: Core and User.
        const dictionarySheetSelector = createDictionarySheetSelector(elements.dictionarySheetSelect);
        dictionarySheetSelector.setEntries([
            { sheetId: 'core', label: formatDictionaryTabName('CORE'), kind: 'core' },
            { sheetId: 'user', label: formatDictionaryTabName('USER'), kind: 'user' },
        ]);


        layoutController = createLayoutController({
            state: layoutState,
            elements,
            mobile,
            buildApplyAreaStateDeps
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
            highlightSourceRange: (start, end) => editor.revealRange(start, end),
            clearStack: () => clearStack(),
            showDocumentation: (text) => display.renderDocumentation(text),
            showError: (error, precedingOutput) => display.renderError(error, precedingOutput),
            showExecutionResult: (result) => display.renderExecutionResult(result),
            updateDisplays: updateAllDisplays,
            saveState: () => persistence.saveCurrentState(),
            fullReset: () => persistence.fullReset(),
            updateView: (mode) => layoutController.setArea(mode),
            updateAfterExecution: (changes) => {
                applyExecutionAreaState(buildApplyAreaStateDeps(), changes);
            }
        });

        bindGuiEvents({
            elements,
            mobile,
            layoutState,
            vocabulary,
            display,
            editor,
            executionController,
            persistence,
            switchArea: (mode) => layoutController.setArea(mode),
            updateAllDisplays,
            clearStack,
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
