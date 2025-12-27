// js/gui/persistence.ts - 永続化管理（関数型スタイル）

import type { AjisaiInterpreter, Value, CustomWord } from '../wasm-types';
import type DB from '../db';
import { SAMPLE_CUSTOM_WORDS } from './sample-words';
import { Result, ok, err } from './fp-utils';

// ============================================================
// 型定義
// ============================================================

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
    readonly exportCustomWords: () => void;
    readonly importCustomWords: () => void;
}

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
        AjisaiDB: typeof DB;
    }
}

// ============================================================
// 純粋関数: データ変換
// ============================================================

/**
 * カスタムワード情報を CustomWord 型に変換
 */
const toCustomWord = (
    wordData: [string, string | null, boolean],
    getDefinition: (name: string) => string | null
): CustomWord => ({
    name: wordData[0],
    description: wordData[1],
    definition: getDefinition(wordData[0])
});

/**
 * インタープリタの現在の状態を取得
 */
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

/**
 * エクスポート用のカスタムワードデータを作成
 */
const createExportData = (interpreter: AjisaiInterpreter): CustomWord[] => {
    const customWordsInfo = interpreter.get_custom_words_info();
    return customWordsInfo.map(wordData => ({
        name: wordData[0],
        description: wordData[1],
        definition: interpreter.get_word_definition(wordData[0])
    }));
};

/**
 * ファイル名を生成（タイムスタンプ付き）
 */
const generateExportFilename = (): string => {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    return `ajisai_words_${timestamp}.json`;
};

// ============================================================
// 副作用関数: ファイル操作
// ============================================================

/**
 * JSONファイルをダウンロード
 */
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

/**
 * ファイル選択ダイアログを開く
 */
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

/**
 * ファイルをテキストとして読み込む
 */
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

/**
 * JSONをパースしてカスタムワード配列として検証
 */
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

// ============================================================
// ファクトリ関数: Persistence作成
// ============================================================

export const createPersistence = (callbacks: PersistenceCallbacks = {}): Persistence => {
    const { showError, updateDisplays, showInfo } = callbacks;

    // データベース初期化
    const init = async (): Promise<void> => {
        try {
            await window.AjisaiDB.open();
            console.log('Database initialized successfully for Persistence.');
        } catch (error) {
            console.error('Failed to initialize persistence database:', error);
        }
    };

    // 現在の状態を保存
    const saveCurrentState = async (): Promise<void> => {
        if (!window.ajisaiInterpreter) return;

        try {
            const state = getCurrentState(window.ajisaiInterpreter);
            await window.AjisaiDB.saveInterpreterState(state);
            console.log('State saved automatically.');
        } catch (error) {
            console.error('Failed to auto-save state:', error);
        }
    };

    // サンプルワードを読み込む
    const loadSampleWords = async (): Promise<void> => {
        try {
            await window.ajisaiInterpreter.restore_custom_words(SAMPLE_CUSTOM_WORDS);
            await saveCurrentState();
            console.log('Sample custom words loaded.');

            // サンプルワード読み込み完了メッセージを表示
            const wordNames = SAMPLE_CUSTOM_WORDS.map(w => w.name).join(', ');
            showInfo?.(`Sample words loaded: ${wordNames}`, false);
        } catch (error) {
            console.error('Failed to load sample words:', error);
        }
    };

    // データベースからデータを読み込む
    const loadDatabaseData = async (): Promise<void> => {
        if (!window.ajisaiInterpreter) return;

        try {
            const state = await window.AjisaiDB.loadInterpreterState();

            if (state) {
                if (state.stack) {
                    window.ajisaiInterpreter.restore_stack(state.stack);
                }

                if (state.customWords && state.customWords.length > 0) {
                    await window.ajisaiInterpreter.restore_custom_words(state.customWords);
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

    // カスタムワードをエクスポート
    const exportCustomWords = (): void => {
        if (!window.ajisaiInterpreter) {
            showError?.(new Error('Interpreter not available'));
            return;
        }

        const exportData = createExportData(window.ajisaiInterpreter);
        const filename = generateExportFilename();

        downloadJson(exportData, filename);
        showInfo?.(`Custom words exported as ${filename}.`, true);
    };

    // カスタムワードをインポート
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
                showInfo?.(`${importedWords.length} custom words imported and saved.`, true);

            } catch (error) {
                showError?.(error as Error);
            }
        });
    };

    return {
        init,
        saveCurrentState,
        loadDatabaseData,
        exportCustomWords,
        importCustomWords
    };
};

// 純粋関数をエクスポート（テスト用）
export const persistenceUtils = {
    toCustomWord,
    getCurrentState,
    createExportData,
    generateExportFilename,
    parseCustomWords
};
