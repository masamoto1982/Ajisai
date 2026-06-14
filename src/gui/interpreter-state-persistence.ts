

import type { AjisaiInterpreter, Value, UserWord, ImportStateEntry } from '../wasm-interpreter-types';
import { EXAMPLE_USER_WORDS, EXAMPLE_WORDS_VERSION } from './example-words';
import { getPlatform } from '../platform';
import { Result, ok, err } from './functional-result-helpers';

export interface InterpreterState {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules?: string[];
    // Detailed per-module import state. Preferred over `importedModules` on
    // restore so partial imports (IMPORT-ONLY / UNIMPORT-ONLY) survive reload.
    readonly importState?: ImportStateEntry[];
    readonly exampleWordsVersion?: number;
    readonly activeDictionarySheet?: string;
    readonly activeUserDictionary?: string;
}

export interface RestoredSelection {
    readonly activeDictionarySheet?: string;
    readonly activeUserDictionary?: string;
}

export interface PersistenceCallbacks {
    readonly showError?: (error: Error) => void;
    readonly updateDisplays?: () => void;
    readonly showInfo?: (text: string, append: boolean) => void;
}

export interface Persistence {
    readonly init: () => Promise<void>;
    readonly saveCurrentState: () => Promise<void>;
    readonly loadDatabaseData: () => Promise<RestoredSelection>;
    readonly fullReset: () => Promise<void>;
    readonly exportUserWords: () => void;
    readonly importUserWords: () => void;
    readonly importJsonAsVector: () => void;
}

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

const toUserWord = (
    wordData: [string, string, boolean],
    getDefinition: (name: string) => string | null
): UserWord => ({
    dictionary: wordData[0],
    name: wordData[1],
    definition: getDefinition(`${wordData[0]}@${wordData[1]}`)
});

const readActiveSelections = (): {
    activeDictionarySheet?: string;
    activeUserDictionary?: string;
} => {
    const sheetSelect = document.getElementById('dictionary-sheet-select') as HTMLSelectElement | null;
    const userDictSelect = document.getElementById('user-dictionary-select') as HTMLSelectElement | null;
    return {
        activeDictionarySheet: sheetSelect?.value || undefined,
        activeUserDictionary: userDictSelect?.value || undefined
    };
};

const collectCurrentState = (interpreter: AjisaiInterpreter): InterpreterState => {
    const userWordsInfo = interpreter.collect_user_words_info();
    const userWords: UserWord[] = userWordsInfo.map(wordData =>
        toUserWord(wordData, name => interpreter.lookup_word_definition(name))
    );

    const selections = readActiveSelections();

    return {
        stack: interpreter.collect_stack(),
        userWords,
        importedModules: interpreter.collect_imported_modules(),
        importState: interpreter.collect_import_state(),
        exampleWordsVersion: EXAMPLE_WORDS_VERSION,
        activeDictionarySheet: selections.activeDictionarySheet,
        activeUserDictionary: selections.activeUserDictionary
    };
};

const createExportData = (interpreter: AjisaiInterpreter, dictionaryName: string): UserWord[] => {
    const userWordsInfo = interpreter.collect_user_words_info();
    return userWordsInfo
        .filter(([dictionary]) => dictionary === dictionaryName)
        .map(wordData => ({
            dictionary: wordData[0],
            name: wordData[1],
            definition: interpreter.lookup_word_definition(`${wordData[0]}@${wordData[1]}`)
        }));
};

const buildExportFilename = (name: string): string => `${name}.json`;
const buildWordKey = (dictionary: string, name: string): string => `${dictionary}@${name}`.toUpperCase();
const REMOVED_USER_WORD_DICTIONARIES = new Set(['DEMO']);
const filenameToDictionaryName = (filename: string): string => filename.replace(/\.json$/i, '').toUpperCase();
const isRemovedUserWordDictionary = (dictionary: string | null | undefined): boolean =>
    REMOVED_USER_WORD_DICTIONARIES.has((dictionary || '').toUpperCase());
const containsRemovedUserWordDictionary = (words: readonly UserWord[]): boolean =>
    words.some(word => isRemovedUserWordDictionary(word.dictionary));

const parseUserWords = (jsonString: string): Result<UserWord[], Error> => {
    try {
        const parsed = JSON.parse(jsonString);
        if (!Array.isArray(parsed)) {
            return err(new Error('Invalid file format. Expected an array of words.'));
        }
        return ok(parsed as UserWord[]);
    } catch (e) {
        return err(e instanceof Error ? e : new Error(String(e)));
    }
};

export const createPersistence = (callbacks: PersistenceCallbacks = {}): Persistence => {
    const { showError, updateDisplays, showInfo } = callbacks;
    let dbInitialized = false;
    const MAX_RETRY_COUNT = 3;
    const RETRY_DELAY_MS = 1000;

    const sleep = (ms: number): Promise<void> =>
        new Promise(resolve => setTimeout(resolve, ms));

    const init = async (): Promise<void> => {
        for (let attempt = 1; attempt <= MAX_RETRY_COUNT; attempt++) {
            try {
                await getPlatform().persistence.open();
                dbInitialized = true;
                console.log('Database initialized successfully for Persistence.');
                return;
            } catch (error) {
                console.error(`Failed to initialize persistence database (attempt ${attempt}/${MAX_RETRY_COUNT}):`, error);
                if (attempt < MAX_RETRY_COUNT) {
                    await sleep(RETRY_DELAY_MS * attempt);
                }
            }
        }

        console.warn('Persistence database initialization failed after all retries. Data will not be persisted.');
        showError?.(new Error('Failed to initialize database. Changes will not be saved.'));
    };

    const SAVE_DEBOUNCE_MS = 400;
    let saveTimer: ReturnType<typeof setTimeout> | null = null;
    let pendingSave: Promise<void> | null = null;
    let resolvePendingSave: (() => void) | null = null;

    const performSave = async (): Promise<void> => {
        if (!window.ajisaiInterpreter) return;
        if (!dbInitialized) {
            console.warn('Database not initialized, skipping state save.');
            return;
        }

        try {
            const state = collectCurrentState(window.ajisaiInterpreter);
            await getPlatform().persistence.saveInterpreterState(state);
            console.log('State saved automatically.');
        } catch (error) {
            console.error('Failed to auto-save state:', error);
        }
    };

    // Auto-save fires after every execution, word edit and dictionary switch.
    // Debouncing coalesces bursts of those operations into a single snapshot
    // + IndexedDB write. The snapshot is taken when the timer fires, so the
    // most recent interpreter state is always the one persisted.
    const flushPendingSave = (): void => {
        if (!saveTimer) return;
        clearTimeout(saveTimer);
        saveTimer = null;
        const resolve = resolvePendingSave;
        pendingSave = null;
        resolvePendingSave = null;
        void performSave().finally(() => resolve?.());
    };

    const saveCurrentState = (): Promise<void> => {
        if (!pendingSave) {
            pendingSave = new Promise<void>(resolve => { resolvePendingSave = resolve; });
        }
        if (saveTimer) clearTimeout(saveTimer);
        saveTimer = setTimeout(flushPendingSave, SAVE_DEBOUNCE_MS);
        return pendingSave;
    };

    // A debounced save would otherwise be lost if the tab is hidden or closed
    // within the debounce window, so flush it immediately on those events.
    if (typeof document !== 'undefined') {
        document.addEventListener('visibilitychange', () => {
            if (document.visibilityState === 'hidden') flushPendingSave();
        });
        window.addEventListener('pagehide', flushPendingSave);
    }

    const loadExampleWords = async (): Promise<void> => {
        try {
            await window.ajisaiInterpreter.restore_user_words(EXAMPLE_USER_WORDS);
            await saveCurrentState();
            console.log('Example Words loaded.');

            const wordNames = EXAMPLE_USER_WORDS.map(w => w.name).join(', ');
            showInfo?.(`Example Words loaded: ${wordNames}`, false);
        } catch (error) {
            console.error('Failed to load Example Words:', error);
        }
    };

    const loadDatabaseData = async (): Promise<RestoredSelection> => {
        if (!window.ajisaiInterpreter) return {};
        if (!dbInitialized) {
            console.warn('Database not initialized, loading Example Words instead.');
            await loadExampleWords();
            return {};
        }

        try {
            const state = await getPlatform().persistence.loadInterpreterState();

            if (state) {
                if (state.stack) {
                    window.ajisaiInterpreter.restore_stack(state.stack);
                }
                // Prefer the detailed import state (preserves partial imports);
                // fall back to the legacy module-name list for older snapshots.
                if (state.importState && state.importState.length > 0) {
                    window.ajisaiInterpreter.restore_import_state(state.importState);
                } else if (state.importedModules && state.importedModules.length > 0) {
                    window.ajisaiInterpreter.restore_imported_modules(state.importedModules);
                }

                if (state.userWords && state.userWords.length > 0) {
                    if (containsRemovedUserWordDictionary(state.userWords)) {
                        console.warn('Removed legacy user word dictionary found in saved state; resetting to Example Words.');
                        await loadExampleWords();
                        return {
                            activeDictionarySheet: state.activeDictionarySheet,
                            activeUserDictionary: 'EXAMPLE'
                        };
                    }

                    const savedVersion = state.exampleWordsVersion || 0;
                    const wordsToRestore = state.userWords;

                    await window.ajisaiInterpreter.restore_user_words(wordsToRestore);


                    const savedWordKeys = new Set(
                        wordsToRestore.map((w: UserWord) => buildWordKey(w.dictionary || 'EXAMPLE', w.name))
                    );
                    const currentWords = window.ajisaiInterpreter.collect_user_words_info();
                    for (const [dictionary, name] of currentWords) {
                        const currentWordKey = buildWordKey(dictionary, name);
                        if (!savedWordKeys.has(currentWordKey)) {
                            window.ajisaiInterpreter.remove_word(`${dictionary}@${name}`);
                        }
                    }


                    if (savedVersion < EXAMPLE_WORDS_VERSION) {
                        await saveCurrentState();
                    }

                    console.log('Interpreter state restored.');
                } else {
                    await loadExampleWords();
                }

                return {
                    activeDictionarySheet: state.activeDictionarySheet,
                    activeUserDictionary: state.activeUserDictionary
                };
            } else {
                await loadExampleWords();
                return {};
            }
        } catch (error) {
            console.error('Failed to load database data:', error);
            showError?.(error as Error);
            return {};
        }
    };

    const exportUserWords = (): void => {
        if (!window.ajisaiInterpreter) {
            showError?.(new Error('Interpreter not available'));
            return;
        }

        const selectedDictionary = (document.getElementById('user-dictionary-select') as HTMLSelectElement | null)?.value || 'EXAMPLE';
        const suggestedName = selectedDictionary.toLowerCase();
        const requestedName = window.prompt('Export file name', suggestedName)?.trim();
        if (!requestedName) {
            return;
        }
        const exportData = createExportData(window.ajisaiInterpreter, selectedDictionary);
        const filename = buildExportFilename(requestedName);

        getPlatform().fileIO.saveJson(filename, exportData)
            .then(() => showInfo?.(`User words exported as ${filename}`, true))
            .catch((error) => showError?.(error as Error));
    };

    const importUserWords = (): void => {
        getPlatform().fileIO.openJsonFile().then(async (openedFile) => {
            if (!openedFile) {
                return;
            }

            try {
                const parseResult = parseUserWords(openedFile.text);

                if (!parseResult.ok) {
                    showError?.(parseResult.error);
                    return;
                }

                const dictionary = filenameToDictionaryName(openedFile.filename);
                if (isRemovedUserWordDictionary(dictionary)) {
                    showError?.(new Error('DEMO is no longer accepted as a User Words label. Rename the file before importing.'));
                    return;
                }

                const importedWords = parseResult.value.map(word => ({
                    ...word,
                    dictionary
                }));
                await window.ajisaiInterpreter.restore_user_words(importedWords);

                updateDisplays?.();
                await saveCurrentState();
                showInfo?.(`${importedWords.length} user words imported and saved`, true);

            } catch (error) {
                showError?.(error as Error);
            }
        });
    };

    const importJsonAsVector = (): void => {
        getPlatform().fileIO.openJsonFile().then(async (openedFile) => {
            if (!openedFile) {
                return;
            }

            try {
                try {
                    JSON.parse(openedFile.text);
                } catch {
                    showError?.(new Error('Invalid JSON file.'));
                    return;
                }

                const result = window.ajisaiInterpreter.push_json_string(openedFile.text);

                if (result.status === 'OK') {
                    updateDisplays?.();
                    await saveCurrentState();
                    showInfo?.(`JSON loaded from ${openedFile.filename}`, true);
                } else {
                    showError?.(new Error(result.message || 'Failed to parse JSON'));
                }
            } catch (error) {
                showError?.(error as Error);
            }
        });
    };

    const fullReset = async (): Promise<void> => {
        try {
            if (dbInitialized) {
                await getPlatform().persistence.clearAll();
                console.log('IndexedDB cleared.');
            } else {
                console.warn('Database not initialized, skipping clear operation.');
            }
            await loadExampleWords();
            updateDisplays?.();
        } catch (error) {
            console.error('Failed to perform full reset:', error);
            showError?.(error as Error);
        }
    };

    return {
        init,
        saveCurrentState,
        loadDatabaseData,
        fullReset,
        exportUserWords,
        importUserWords,
        importJsonAsVector
    };
};
