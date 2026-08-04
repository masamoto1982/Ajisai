

import type { AjisaiInterpreter, Value, UserWord } from '../wasm-interpreter-types';
import { EXAMPLE_USER_WORDS } from './example-words';
import { getPlatform } from '../platform';
import { Result, ok, err } from './functional-result-helpers';

// The persisted session document. `stateVersion` identifies the format; a
// document that does not carry the current version is not migrated — the beta
// reads one format only (SPEC §2.3).
export const STATE_FORMAT_VERSION = 3;

export interface InterpreterState {
    readonly stateVersion: number;
    // The observation-format stack, persisted for display only.
    readonly stack: Value[];
    // The lossless stack snapshot (opaque string) and the only format restore
    // accepts.
    readonly stackSnapshot: string;
    readonly userWords: UserWord[];
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

// Words are addressed by bare name (LANG.DICTIONARY.RESOLUTION: two tiers, and
// User is one of them). A `DICT@NAME` prefix resolves to nothing, so composing
// one here persisted every definition as null and lost the user's words on the
// next load.
const toUserWord = (
    wordData: [string, string, boolean],
    getDefinition: (name: string) => string | null
): UserWord => ({
    dictionary: wordData[0],
    name: wordData[1],
    definition: getDefinition(wordData[1])
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
        stateVersion: STATE_FORMAT_VERSION,
        // The lossless snapshot is what restore reads; `stack` is display data.
        stack: interpreter.collect_stack(),
        stackSnapshot: interpreter.snapshot_stack(),
        userWords,
        activeDictionarySheet: selections.activeDictionarySheet,
        activeUserDictionary: selections.activeUserDictionary
    };
};

// Identity-keyed export/import (SPECIFICATION.html §8.6). The export document
// carries each word's content identity so a shared group is content-addressed:
// re-importing it is recognised as a no-op (deduplicated), and a definition
// edited without re-exporting is detected via an identity mismatch.
const EXPORT_FORMAT_VERSION = 3;

interface ExportWord {
    readonly name: string;
    readonly definition: string | null;
    readonly id?: string;
}

interface ExportDocument {
    readonly formatVersion: number;
    readonly dictionary: string;
    readonly words: ExportWord[];
}

// Keyed by bare name, matching `collect_word_identities`, which reports the
// User-tier key itself rather than a `DICT@NAME` composite.
const collectWordIdentityMap = (interpreter: AjisaiInterpreter): Map<string, string> => {
    const map = new Map<string, string>();
    for (const [fqName, id] of interpreter.collect_word_identities()) {
        map.set(buildWordKey(fqName), id);
    }
    return map;
};

const createExportData = (interpreter: AjisaiInterpreter, dictionaryName: string): ExportDocument => {
    const identities = collectWordIdentityMap(interpreter);
    const words: ExportWord[] = interpreter.collect_user_words_info()
        .filter(([dictionary]) => dictionary === dictionaryName)
        .map(([, name]) => {
            const id = identities.get(buildWordKey(name));
            return {
                name,
                definition: interpreter.lookup_word_definition(name),
                ...(id ? { id } : {})
            };
        });
    return { formatVersion: EXPORT_FORMAT_VERSION, dictionary: dictionaryName, words };
};

export interface ParsedImport {
    readonly words: UserWord[];
    // Embedded content identities keyed by upper-cased word name, or null when
    // the document carries none.
    readonly embeddedIds: Map<string, string> | null;
}

const buildExportFilename = (name: string): string => `${name}.json`;
// The whole key of a User-tier word: its normalized name.
const buildWordKey = (name: string): string => name.toUpperCase();
const filenameToDictionaryName = (filename: string): string => filename.replace(/\.json$/i, '').toUpperCase();

// Validate a single raw word entry from an (untrusted) import file. Returns a
// normalized word, or null when the entry is malformed. A word is only usable
// downstream if it has a string `name`; `definition` and `id` are optional and
// must be strings when present. This keeps `parseImportDocument` a total
// function — a hostile or hand-corrupted file with `null`, numeric, or
// name-less entries previously threw a TypeError out of the `.map` (and
// would have thrown again at `word.name.toUpperCase()` in `importUserWords`)
// instead of being parsed or cleanly rejected.
const normalizeImportWord = (
    raw: unknown
): { name: string; definition: string | null; id?: string } | null => {
    if (!raw || typeof raw !== 'object') return null;
    const word = raw as Record<string, unknown>;
    if (typeof word.name !== 'string') return null;
    const definition = typeof word.definition === 'string' ? word.definition : null;
    const id = typeof word.id === 'string' ? word.id : undefined;
    return id ? { name: word.name, definition, id } : { name: word.name, definition };
};

export const parseImportDocument = (jsonString: string): Result<ParsedImport, Error> => {
    let parsed: unknown;
    try {
        parsed = JSON.parse(jsonString);
    } catch (e) {
        return err(e instanceof Error ? e : new Error(String(e)));
    }

    // Both branches drop malformed entries rather than throwing or forwarding
    // them to `restore_user_words`; valid words in a partially-corrupt file
    // still import.
    const collect = (rawWords: unknown[]): ParsedImport => {
        const embeddedIds = new Map<string, string>();
        const words: UserWord[] = [];
        for (const raw of rawWords) {
            const word = normalizeImportWord(raw);
            if (!word) continue;
            if (word.id) embeddedIds.set(word.name.toUpperCase(), word.id);
            words.push({ name: word.name, definition: word.definition });
        }
        return { words, embeddedIds: embeddedIds.size > 0 ? embeddedIds : null };
    };

    // A versioned export document carrying per-word content identities is the
    // only accepted form; the bare array of the alpha format is not read.
    if (parsed && typeof parsed === 'object' && Array.isArray((parsed as ExportDocument).words)) {
        return ok(collect((parsed as ExportDocument).words as unknown[]));
    }

    return err(new Error('Invalid file format. Expected a versioned export document with a `words` array.'));
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
                // One accepted format: a document of the current version whose
                // stack is the lossless snapshot. Anything else — an alpha
                // document, a hand-edited one — starts a fresh session with the
                // Example Words rather than being migrated (SPEC §2.3).
                if (state.stateVersion !== STATE_FORMAT_VERSION) {
                    console.warn(
                        `Saved state is format ${String(state.stateVersion)}, not ${STATE_FORMAT_VERSION}; loading Example Words.`
                    );
                    await loadExampleWords();
                    return {};
                }
                if (typeof state.stackSnapshot === 'string') {
                    window.ajisaiInterpreter.restore_stack_snapshot(state.stackSnapshot);
                }

                if (state.userWords && state.userWords.length > 0) {
                    const wordsToRestore = state.userWords;

                    await window.ajisaiInterpreter.restore_user_words(wordsToRestore);

                    const savedWordKeys = new Set(
                        wordsToRestore.map((w: UserWord) => buildWordKey(w.name))
                    );
                    const currentWords = window.ajisaiInterpreter.collect_user_words_info();
                    for (const [, name] of currentWords) {
                        if (!savedWordKeys.has(buildWordKey(name))) {
                            window.ajisaiInterpreter.remove_word(name);
                        }
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
                const parseResult = parseImportDocument(openedFile.text);

                if (!parseResult.ok) {
                    showError?.(parseResult.error);
                    return;
                }

                const dictionary = filenameToDictionaryName(openedFile.filename);

                const { words, embeddedIds } = parseResult.value;
                const importedWords = words.map(word => ({
                    ...word,
                    dictionary
                }));

                // Content-addressed dedup (§8.6): compare identities before and
                // after the merge. Words whose identity is unchanged were already
                // present with identical content and count as deduplicated.
                const before = collectWordIdentityMap(window.ajisaiInterpreter);
                await window.ajisaiInterpreter.restore_user_words(importedWords);
                const after = collectWordIdentityMap(window.ajisaiInterpreter);

                let added = 0;
                let deduplicated = 0;
                const idMismatches: string[] = [];
                for (const word of importedWords) {
                    const fqName = buildWordKey(word.name);
                    if (before.has(fqName) && before.get(fqName) === after.get(fqName)) {
                        deduplicated++;
                    } else {
                        added++;
                    }
                    if (embeddedIds) {
                        const expected = embeddedIds.get(word.name.toUpperCase());
                        const actual = after.get(fqName);
                        if (expected && actual && expected !== actual) {
                            idMismatches.push(word.name);
                        }
                    }
                }

                updateDisplays?.();
                await saveCurrentState();

                const summary = deduplicated > 0
                    ? `${added} user words imported, ${deduplicated} unchanged (deduplicated by content identity)`
                    : `${added} user words imported and saved`;
                showInfo?.(summary, true);
                if (idMismatches.length > 0) {
                    showInfo?.(
                        `Content identity mismatch (definition edited without re-export): ${idMismatches.join(', ')}`,
                        true
                    );
                }

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
