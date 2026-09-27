import type { AjisaiInterpreter } from '../wasm-interpreter-types';
import { createDisplay } from './output-display-renderer';
import { createVocabularyManager, formatDictionaryTabName } from './vocabulary-state-controller';
import { createEditor } from './code-input-editor';
import { createMobileHandler } from './mobile-view-switcher';
import { createDictionarySheetSelector } from './dictionary-sheet-selector';
import { createPersistence } from './interpreter-state-persistence';
import { createExecutionController } from './execution-controller';
import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import {
    cacheElements,
    extractDisplayElements,
    extractVocabularyElements,
    extractMobileElements
} from './gui-dom-cache';
import {
    applyExecutionAreaState,
    createLayoutController,
    createLayoutState,
    updateEditorPlaceholder,
    type ApplyAreaStateDeps
} from './gui-layout-state';
import { switchDictionarySheet } from './gui-dictionary-sheet';
import { bindGuiEvents } from './gui-event-bindings';

/**
 * How the Reference's 「Playgroundで開く」 links hand a sample over:
 * `<playground-url>#code=<encodeURIComponent-encoded source>`.
 *
 * Exported because the splash screen also keys off it — arriving this way says
 * the visitor already knows what they came to do (entry-common.ts
 * initSplashScreen()) — and a second copy of the literal would drift.
 */
export const PLAYGROUND_CODE_HASH_MARKER = '#code=';

export interface GUI {
    readonly init: () => Promise<void>;
}

export const createGUI = (interpreter: AjisaiInterpreter): GUI => {
    // The full word list only changes when the vocabulary changes (after an
    // execution). Without this cache the whole set — including a WASM
    // round-trip per query — would be rebuilt on every keystroke.
    let autocompleteWordsCache: string[] | null = null;

    const collectAutocompleteWords = (): string[] => {
        if (autocompleteWordsCache) return autocompleteWordsCache;
        // Canonical names only: the Core list carries no alias, and a User
        // Word is addressed by its bare name.
        const coreWords = interpreter.collect_core_words_info().map(([name]) => name);
        const userWords = interpreter.collect_user_words_info().map(([, name]) => name);
        autocompleteWordsCache = [...new Set([...coreWords, ...userWords])].sort((a, b) => a.localeCompare(b));
        return autocompleteWordsCache;
    };

    const init = async (): Promise<void> => {
        console.log('[GUI] Initializing GUI...');

        const elements = cacheElements();
        const layoutState = createLayoutState();
        const display = createDisplay(extractDisplayElements(elements));
        display.init();

        // The dictionary has two tiers (LANG.DICTIONARY.RESOLUTION), so the
        // sheet list is fixed: Core and User.
        const sheetSelector = createDictionarySheetSelector(elements.dictionarySheetSelect, {
            onChange: (sheetId) => {
                switchDictionarySheet(elements.dictionaryArea, sheetId);
                void persistence.saveCurrentState();
            }
        });
        sheetSelector.setEntries([
            { sheetId: 'core', label: formatDictionaryTabName('CORE'), kind: 'core' },
            { sheetId: 'user', label: formatDictionaryTabName('USER'), kind: 'user' },
        ]);
        const showDictionarySheet = (sheetId: string): void => {
            sheetSelector.select(sheetId);
            switchDictionarySheet(elements.dictionaryArea, sheetId);
        };

        const mobile = createMobileHandler(extractMobileElements(elements), {
            currentMode: () => layoutState.currentMode,
            onModeChange: (mode) => layoutController.setArea(mode)
        });
        updateEditorPlaceholder(elements, mobile);

        const layoutDeps: ApplyAreaStateDeps = { elements, state: layoutState, mobile, showDictionarySheet };
        const layoutController = createLayoutController(layoutDeps);

        const updateAllDisplays = (): void => {
            autocompleteWordsCache = null;
            try {
                display.renderStack(interpreter.collect_stack());
                vocabulary.updateUserWords(interpreter.collect_user_words_info());
            } catch (error) {
                console.error('Failed to update display:', error);
                display.renderError(new Error('Failed to update display.'));
            }
        };

        const persistence = createPersistence(interpreter, {
            showError: (error) => display.renderError(error),
            updateDisplays: updateAllDisplays,
            showInfo: (text, append) => display.renderInfo(text, append),
            readActiveDictionarySheet: () => sheetSelector.current()
        });
        await persistence.init();

        const editor = createEditor(elements.codeInput, {
            onSwitchToInputMode: () => layoutController.setArea('input'),
            onRequestSuggestions: () => collectAutocompleteWords()
        });

        const vocabulary = createVocabularyManager(interpreter, extractVocabularyElements(elements), {
            // One behaviour in both presentations, as the mobile placeholder
            // advertises (`tap a Dictionary word too`).
            onWordClick: (word) => editor.insertWord(word),
            onBackgroundClick: () => editor.insertWord(' '),
            onBackgroundDoubleClick: () => editor.removeLastWord(),
            onUpdateDisplays: updateAllDisplays,
            onSaveState: () => persistence.saveCurrentState(),
            showInfo: (text, append) => display.renderInfo(text, append)
        });

        // Clearing the stack keeps the dictionary — that is what separates it
        // from Reset — so it is the interpreter's `clear_stack` and nothing
        // else, followed by a redraw and a save. One definition for both
        // routes to it: the Stack area's `×` and `Ctrl+Alt+S`. It has no typed
        // spelling (spec/gui-semantics.md, "Operations without a typed spelling").
        const clearStack = (): void => {
            interpreter.clear_stack();
            updateAllDisplays();
            display.renderInfo('Stack cleared', false);
            void persistence.saveCurrentState();
        };

        const executionController = createExecutionController(interpreter, {
            // Step mode (the sole consumer of this callback) splits the
            // extracted source on whitespace and feeds each piece to the
            // interpreter on its own — the same whitespace-only split the
            // tokenizer itself requires around `[` and `]` (SPEC
            // LANG.SOURCE.TEXT). Formatting first, exactly like `runEditorCode`
            // does for a normal run, guarantees every piece is one the
            // tokenizer accepts even when the author wrote brackets glued to
            // other text.
            extractEditorValue: () => { editor.format(); return editor.extractValue(); },
            clearEditor: (switchView) => { editor.clear(switchView); },
            showInfo: (text, append) => display.renderInfo(text, append),
            showFoldedInfo: (label, text) => display.renderFoldedInfo(label, text),
            highlightSourceRange: (start, end) => editor.revealRange(start, end),
            showDocumentation: (text) => display.renderDocumentation(text),
            showError: (error, precedingOutput) => display.renderError(error, precedingOutput),
            showExecutionResult: (result) => display.renderExecutionResult(result),
            updateDisplays: updateAllDisplays,
            saveState: () => persistence.saveCurrentState(),
            fullReset: () => persistence.fullReset(),
            updateView: (mode) => layoutController.setArea(mode),
            updateAfterExecution: (changes) => applyExecutionAreaState(layoutDeps, changes)
        });

        bindGuiEvents({
            elements,
            mobile,
            layoutState,
            layoutController,
            vocabulary,
            display,
            editor,
            executionController,
            persistence,
            clearStack
        });
        vocabulary.renderBuiltInWords();
        updateAllDisplays();

        const restored = await persistence.loadDatabaseData();
        updateAllDisplays();
        if (restored.activeDictionarySheet) {
            showDictionarySheet(restored.activeDictionarySheet);
        }

        try {
            display.renderInfo('Initializing...', false);
            await WORKER_MANAGER.init();
            display.renderInfo('Ready', true);
        } catch (error) {
            console.error('[GUI] Failed to initialize workers:', error);
            display.renderError(new Error(`Failed to initialize parallel execution: ${error}`));
        }

        // A sample handed over by a Reference 「Playgroundで開く」 link, loaded
        // into the editor once; the fragment is stripped so a reload does not
        // load it again.
        const hash = window.location.hash;
        if (hash.startsWith(PLAYGROUND_CODE_HASH_MARKER)) {
            try {
                const code = decodeURIComponent(hash.slice(PLAYGROUND_CODE_HASH_MARKER.length));
                if (code.trim().length > 0) {
                    editor.updateValue(code);
                    window.history.replaceState(null, '', window.location.pathname + window.location.search);
                }
            } catch (error) {
                console.warn('[GUI] Failed to apply playground code from URL:', error);
            }
        }

        console.log('[GUI] GUI initialization completed');
    };

    return { init };
};
