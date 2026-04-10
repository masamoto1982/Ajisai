
import type { AjisaiInterpreter, Value, UserWord } from '../wasm-interpreter-types';
import { DEMO_USER_WORDS, DEMO_WORDS_VERSION } from './demo-words';
import { Result, ok, err } from './functional-result-helpers';
import type { AjisaiRuntime } from '../core/ajisai-runtime-types';
import type { FilePort } from '../platform/file-port';
import type { StoragePort } from '../platform/storage-port';

export interface InterpreterState {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly demoWordsVersion?: number;
}

export interface PersistenceCallbacks {
    readonly runtime: AjisaiRuntime;
    readonly root?: ParentNode;
    readonly files: FilePort;
    readonly storage: StoragePort;
    readonly showError?: (error: Error) => void;
    readonly updateDisplays?: () => void;
    readonly showInfo?: (text: string, append: boolean) => void;
}

export interface Persistence {
    readonly init: () => Promise<void>;
    readonly saveCurrentState: () => Promise<void>;
    readonly loadDatabaseData: () => Promise<void>;
    readonly fullReset: () => Promise<void>;
    readonly exportUserWords: () => Promise<void>;
    readonly importUserWords: () => Promise<void>;
    readonly importJsonAsVector: () => Promise<void>;
}

const toUserWord = (
    wordData: [string, string, string | null, boolean],
    getDefinition: (name: string) => string | null
): UserWord => ({
    dictionary: wordData[0],
    name: wordData[1],
    description: wordData[2],
    definition: getDefinition(`${wordData[0]}@${wordData[1]}`)
});

const collectCurrentState = (interpreter: AjisaiInterpreter): InterpreterState => {
    const userWordsInfo = interpreter.collect_user_words_info();
    const userWords: UserWord[] = userWordsInfo.map(wordData =>
        toUserWord(wordData, name => interpreter.lookup_word_definition(name))
    );

    return {
        stack: interpreter.collect_stack(),
        userWords,
        demoWordsVersion: DEMO_WORDS_VERSION
    };
};

const createExportData = (interpreter: AjisaiInterpreter, dictionaryName: string): UserWord[] => {
    const userWordsInfo = interpreter.collect_user_words_info();
    return userWordsInfo
        .filter(([dictionary]) => dictionary === dictionaryName)
        .map(wordData => ({
            dictionary: wordData[0],
            name: wordData[1],
            description: wordData[2],
            definition: interpreter.lookup_word_definition(`${wordData[0]}@${wordData[1]}`)
        }));
};

const buildExportFilename = (name: string): string => `${name}.json`;

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

export const createPersistence = (callbacks: PersistenceCallbacks): Persistence => {
    const { runtime, root = document, files, storage, showError, updateDisplays, showInfo } = callbacks;
    let dbInitialized = false;
    const MAX_RETRY_COUNT = 3;
    const RETRY_DELAY_MS = 1000;

    const sleep = (ms: number): Promise<void> =>
        new Promise(resolve => setTimeout(resolve, ms));

    const init = async (): Promise<void> => {
        for (let attempt = 1; attempt <= MAX_RETRY_COUNT; attempt++) {
            try {
                await storage.open();
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

    const saveCurrentState = async (): Promise<void> => {
        if (!dbInitialized) {
            console.warn('Database not initialized, skipping state save.');
            return;
        }

        try {
            const state = collectCurrentState({
                collect_user_words_info: runtime.collectUserWordsInfo,
                lookup_word_definition: runtime.lookupWordDefinition,
                collect_stack: runtime.collectStack
            } as AjisaiInterpreter);
            await storage.saveInterpreterState(state);
            console.log('State saved automatically.');
        } catch (error) {
            console.error('Failed to auto-save state:', error);
        }
    };

    const loadDemoWords = async (): Promise<void> => {
        try {
            await runtime.restoreUserWords(DEMO_USER_WORDS);
            await saveCurrentState();
            console.log('Demo user words loaded.');

            const wordNames = DEMO_USER_WORDS.map(w => w.name).join(', ');
            showInfo?.(`Sample words loaded: ${wordNames}`, false);
        } catch (error) {
            console.error('Failed to load sample words:', error);
        }
    };

    const loadDatabaseData = async (): Promise<void> => {
        if (!dbInitialized) {
            console.warn('Database not initialized, loading sample words instead.');
            await loadDemoWords();
            return;
        }

        try {
            const state = await storage.loadInterpreterState();

            if (state) {
                if (state.stack) {
                    runtime.restoreStack(state.stack);
                }

                if (state.userWords && state.userWords.length > 0) {

                    const savedVersion = state.demoWordsVersion || 0;
                    let wordsToRestore = state.userWords;

                    if (savedVersion < DEMO_WORDS_VERSION) {

                        const oldSampleNames = new Set([

                            'C4', 'D4', 'E4', 'F4', 'G4', 'A4', 'B4', 'C5',

                            'GREETING', 'WORLD', 'HELLO-WORLD',
                        ]);
                        const newSampleWordNames = new Set(
                            DEMO_USER_WORDS.map(w => w.name.toUpperCase())
                        );


                        const userWords = state.userWords.filter(
                            (w: UserWord) =>
                                !oldSampleNames.has(w.name.toUpperCase()) &&
                                !newSampleWordNames.has(w.name.toUpperCase())
                        );
                        wordsToRestore = [...DEMO_USER_WORDS, ...userWords];
                        console.log(`Sample words migrated: v${savedVersion} → v${DEMO_WORDS_VERSION}`);
                    }

                    await runtime.restoreUserWords(wordsToRestore);




                    const savedWordNames = new Set(wordsToRestore.map((w: UserWord) => w.name.toUpperCase()));
                    const currentWords = runtime.collectUserWordsInfo();
                    for (const [name] of currentWords) {
                        if (!savedWordNames.has(name.toUpperCase())) {
                            runtime.removeWord(name);
                        }
                    }


                    if (savedVersion < DEMO_WORDS_VERSION) {
                        await saveCurrentState();
                    }

                    console.log('Interpreter state restored.');
                } else {
                    await loadDemoWords();
                }
            } else {
                await loadDemoWords();
            }
        } catch (error) {
            console.error('Failed to load database data:', error);
            showError?.(error as Error);
        }
    };

    const exportUserWords = async (): Promise<void> => {
        const selectedDictionary = (root.querySelector('#user-dictionary-select') as HTMLSelectElement | null)?.value || 'DEMO';
        const exportData = createExportData({
            collect_user_words_info: runtime.collectUserWordsInfo,
            lookup_word_definition: runtime.lookupWordDefinition
        } as AjisaiInterpreter, selectedDictionary);
        const filename = buildExportFilename(selectedDictionary.toLowerCase());

        const saved = await files.saveTextFile({
            suggestedName: filename,
            text: JSON.stringify(exportData, null, 2),
            title: 'Export user words'
        });

        if (!saved) {
            return;
        }
        showInfo?.(`User words exported as ${filename}`, true);
    };

    const importUserWords = async (): Promise<void> => {
        const fileResult = await files.pickTextFile({
            accept: '.json',
            title: 'Import user words JSON'
        });

        if (!fileResult) {
            return;
        }

        try {
            const parseResult = parseUserWords(fileResult.text);

            if (!parseResult.ok) {
                showError?.(parseResult.error);
                return;
            }

            const inferredDictionary = fileResult.name?.replace(/\.json$/i, '') ?? 'IMPORTED';
            const importedWords = parseResult.value.map(word => ({
                ...word,
                dictionary: (word.dictionary || inferredDictionary).toUpperCase()
            }));
            await runtime.restoreUserWords(importedWords);

            updateDisplays?.();
            await saveCurrentState();
            showInfo?.(`${importedWords.length} user words imported and saved`, true);

        } catch (error) {
            showError?.(error as Error);
        }
    };

    const importJsonAsVector = async (): Promise<void> => {
        const fileResult = await files.pickTextFile({
            accept: '.json',
            title: 'Import JSON as vector'
        });

        if (!fileResult) {
            return;
        }

        try {
            try {
                JSON.parse(fileResult.text);
            } catch {
                showError?.(new Error('Invalid JSON file.'));
                return;
            }

            const result = runtime.pushJsonString(fileResult.text);

            if (result.status === 'OK') {
                updateDisplays?.();
                await saveCurrentState();
                showInfo?.(`JSON loaded from ${fileResult.name ?? 'selected file'}`, true);
            } else {
                showError?.(new Error(result.message || 'Failed to parse JSON'));
            }
        } catch (error) {
            showError?.(error as Error);
        }
    };

    const fullReset = async (): Promise<void> => {
        try {
            if (dbInitialized) {
                await storage.clearAll();
                console.log('IndexedDB cleared.');
            } else {
                console.warn('Database not initialized, skipping clear operation.');
            }
            await loadDemoWords();
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

export const persistenceUtils = {
    toUserWord,
    collectCurrentState,
    createExportData,
    buildExportFilename,
    parseUserWords
};
