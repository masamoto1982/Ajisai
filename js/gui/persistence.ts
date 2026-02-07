// js/gui/persistence.ts

import type { AjisaiInterpreter, Value, CustomWord } from '../wasm-types';
import type DB from '../db';
import { SAMPLE_CUSTOM_WORDS } from './sample-words';
import { Result, ok, err } from './fp-utils';

export interface InterpreterState {
    readonly stack: Value[];
    readonly customWords: CustomWord[];
}

export interface PersistenceCallbacks {
    readonly showError?: (error: Error) => void;
    readonly updateDisplays?: () => void;
    readonly showInfo?: (text: string, append: boolean) => void;
}

export interface Persistence {
    readonly init: () => Promise<void>;
    readonly saveCurrentState: () => Promise<void>;
    readonly loadDatabaseData: () => Promise<void>;
    readonly fullReset: () => Promise<void>;
    readonly exportCustomWords: () => void;
    readonly importCustomWords: () => void;
}

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
        AjisaiDB: typeof DB;
    }
}

const toCustomWord = (
    wordData: [string, string | null, boolean],
    getDefinition: (name: string) => string | null
): CustomWord => ({
    name: wordData[0],
    description: wordData[1],
    definition: getDefinition(wordData[0])
});

const getCurrentState = (interpreter: AjisaiInterpreter): InterpreterState => {
    const customWordsInfo = interpreter.get_custom_words_info();
    const customWords: CustomWord[] = customWordsInfo.map(wordData =>
        toCustomWord(wordData, name => interpreter.get_word_definition(name))
    );

    return {
        stack: interpreter.get_stack(),
        customWords
    };
};

const createExportData = (interpreter: AjisaiInterpreter): CustomWord[] => {
    const customWordsInfo = interpreter.get_custom_words_info();
    return customWordsInfo.map(wordData => ({
        name: wordData[0],
        description: wordData[1],
        definition: interpreter.get_word_definition(wordData[0])
    }));
};

const generateExportFilename = (): string => {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    return `ajisai_words_${timestamp}.json`;
};

const downloadJson = (data: unknown, filename: string): void => {
    const jsonString = JSON.stringify(data, null, 2);
    const blob = new Blob([jsonString], { type: 'application/json' });
    const url = URL.createObjectURL(blob);

    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
};

const openFileDialog = (
    accept: string,
    onFileSelected: (file: File) => void
): void => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept;

    input.onchange = (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (file) {
            onFileSelected(file);
        }
    };

    input.click();
};

const readFileAsText = (file: File): Promise<string> =>
    new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (event) => {
            const result = event.target?.result;
            if (typeof result === 'string') {
                resolve(result);
            } else {
                reject(new Error('Failed to read file'));
            }
        };
        reader.onerror = () => reject(new Error('Failed to read file'));
        reader.readAsText(file);
    });

const parseCustomWords = (jsonString: string): Result<CustomWord[], Error> => {
    try {
        const parsed = JSON.parse(jsonString);
        if (!Array.isArray(parsed)) {
            return err(new Error('Invalid file format. Expected an array of words.'));
        }
        return ok(parsed as CustomWord[]);
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
                await window.AjisaiDB.open();
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
        // すべてのリトライが失敗した場合
        console.warn('Persistence database initialization failed after all retries. Data will not be persisted.');
        showError?.(new Error('Failed to initialize database. Changes will not be saved.'));
    };

    const saveCurrentState = async (): Promise<void> => {
        if (!window.ajisaiInterpreter) return;
        if (!dbInitialized) {
            console.warn('Database not initialized, skipping state save.');
            return;
        }

        try {
            const state = getCurrentState(window.ajisaiInterpreter);
            await window.AjisaiDB.saveInterpreterState(state);
            console.log('State saved automatically.');
        } catch (error) {
            console.error('Failed to auto-save state:', error);
        }
    };

    const loadSampleWords = async (): Promise<void> => {
        try {
            await window.ajisaiInterpreter.restore_custom_words(SAMPLE_CUSTOM_WORDS);
            await saveCurrentState();
            console.log('Sample custom words loaded.');

            const wordNames = SAMPLE_CUSTOM_WORDS.map(w => w.name).join(', ');
            showInfo?.(`Sample words loaded: ${wordNames}`, false);
        } catch (error) {
            console.error('Failed to load sample words:', error);
        }
    };

    const loadDatabaseData = async (): Promise<void> => {
        if (!window.ajisaiInterpreter) return;
        if (!dbInitialized) {
            console.warn('Database not initialized, loading sample words instead.');
            await loadSampleWords();
            return;
        }

        try {
            const state = await window.AjisaiDB.loadInterpreterState();

            if (state) {
                if (state.stack) {
                    window.ajisaiInterpreter.restore_stack(state.stack);
                }

                if (state.customWords && state.customWords.length > 0) {
                    await window.ajisaiInterpreter.restore_custom_words(state.customWords);

                    // ユーザーが DEL で削除したエクステンションワードを反映する。
                    // new AjisaiInterpreter() は全エクステンションを登録するが、
                    // 保存データに含まれないワードは削除済みなので除去する。
                    const savedWordNames = new Set(state.customWords.map((w: CustomWord) => w.name.toUpperCase()));
                    const currentWords = window.ajisaiInterpreter.get_custom_words_info();
                    for (const [name] of currentWords) {
                        if (!savedWordNames.has(name.toUpperCase())) {
                            window.ajisaiInterpreter.remove_word(name);
                        }
                    }

                    console.log('Interpreter state restored.');
                } else {
                    await loadSampleWords();
                }
            } else {
                await loadSampleWords();
            }
        } catch (error) {
            console.error('Failed to load database data:', error);
            showError?.(error as Error);
        }
    };

    const exportCustomWords = (): void => {
        if (!window.ajisaiInterpreter) {
            showError?.(new Error('Interpreter not available'));
            return;
        }

        const exportData = createExportData(window.ajisaiInterpreter);
        const filename = generateExportFilename();

        downloadJson(exportData, filename);
        showInfo?.(`Custom words exported as ${filename}`, true);
    };

    const importCustomWords = (): void => {
        openFileDialog('.json', async (file) => {
            try {
                const jsonString = await readFileAsText(file);
                const parseResult = parseCustomWords(jsonString);

                if (!parseResult.ok) {
                    showError?.(parseResult.error);
                    return;
                }

                const importedWords = parseResult.value;
                await window.ajisaiInterpreter.restore_custom_words(importedWords);

                updateDisplays?.();
                await saveCurrentState();
                showInfo?.(`${importedWords.length} custom words imported and saved`, true);

            } catch (error) {
                showError?.(error as Error);
            }
        });
    };

    const fullReset = async (): Promise<void> => {
        try {
            if (dbInitialized) {
                await window.AjisaiDB.clearAll();
                console.log('IndexedDB cleared.');
            } else {
                console.warn('Database not initialized, skipping clear operation.');
            }
            await loadSampleWords();
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
        exportCustomWords,
        importCustomWords
    };
};

export const persistenceUtils = {
    toCustomWord,
    getCurrentState,
    createExportData,
    generateExportFilename,
    parseCustomWords
};
