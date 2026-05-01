import { createDisplay, Display } from './output-display-renderer';
import { createVocabularyManager, VocabularyManager } from './vocabulary-state-controller';
import { createEditor, Editor } from './code-input-editor';
import { createMobileHandler, MobileHandler } from './mobile-view-switcher';
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
    updateHighlights,
    updateEditorPlaceholder,
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

export type { GUIElements };

const INTERPRETER_CLIENT = createInterpreterClient();

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
            const sampleWords = INTERPRETER_CLIENT.collectModuleSampleWordsInfo(moduleName);
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


    const updateAllDisplays = (): void => {
        if (!INTERPRETER_CLIENT.getOptional()) return;

        try {
            display.renderStack(INTERPRETER_CLIENT.collectStack());
            vocabulary.updateUserWords(INTERPRETER_CLIENT.collectUserWordsInfo());

            const newSheetIds: string[] = moduleTabManager.syncModuleTabs();

            if (newSheetIds.length > 0) {
                const lastSheetId: string = newSheetIds[newSheetIds.length - 1]!;
                if (layoutState.currentRightMode !== 'dictionary' || (mobile.isMobile() && layoutState.currentMode !== 'dictionary')) {
                    layoutController.setArea('dictionary');
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
            updateView: (mode) => layoutController.setArea(mode)
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
