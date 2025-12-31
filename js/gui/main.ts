// js/gui/main.ts - GUIメインモジュール（関数型スタイル）

import { createDisplay, Display, DisplayElements } from './display';
import { createDictionary, Dictionary, DictionaryElements } from './dictionary';
import { createEditor, Editor } from './editor';
import { createMobileHandler, MobileHandler, MobileElements } from './mobile';
import { createPersistence, Persistence } from './persistence';
import { createExecutionController, ExecutionController } from './execution-controller';
import { WORKER_MANAGER } from '../workers/worker-manager';
import type { AjisaiInterpreter } from '../wasm-types';

// ============================================================
// 型定義
// ============================================================

declare global {
    interface Window {
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

export interface GUIElements {
    readonly codeInput: HTMLTextAreaElement;
    readonly runBtn: HTMLButtonElement;
    readonly clearBtn: HTMLButtonElement;
    readonly testBtn: HTMLButtonElement;
    readonly exportBtn: HTMLButtonElement;
    readonly importBtn: HTMLButtonElement;
    readonly outputDisplay: HTMLElement;
    readonly stackDisplay: HTMLElement;
    readonly builtinWordsDisplay: HTMLElement;
    readonly customWordsDisplay: HTMLElement;
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly dictionaryArea: HTMLElement;
}

export interface GUI {
    readonly init: () => Promise<void>;
    readonly updateAllDisplays: () => void;
    readonly getElements: () => GUIElements;
    readonly getDisplay: () => Display;
    readonly getEditor: () => Editor;
    readonly getDictionary: () => Dictionary;
    readonly getMobile: () => MobileHandler;
    readonly getPersistence: () => Persistence;
    readonly getExecutionController: () => ExecutionController;
}

// ============================================================
// 純粋関数: DOM要素の取得
// ============================================================

const cacheElements = (): GUIElements => ({
    codeInput: document.getElementById('code-input') as HTMLTextAreaElement,
    runBtn: document.getElementById('run-btn') as HTMLButtonElement,
    clearBtn: document.getElementById('clear-btn') as HTMLButtonElement,
    testBtn: document.getElementById('test-btn') as HTMLButtonElement,
    exportBtn: document.getElementById('export-btn') as HTMLButtonElement,
    importBtn: document.getElementById('import-btn') as HTMLButtonElement,
    outputDisplay: document.getElementById('output-display')!,
    stackDisplay: document.getElementById('stack-display')!,
    builtinWordsDisplay: document.getElementById('builtin-words-display')!,
    customWordsDisplay: document.getElementById('custom-words-display')!,
    inputArea: document.querySelector('.input-area')!,
    outputArea: document.querySelector('.output-area')!,
    stackArea: document.querySelector('.stack-area')!,
    dictionaryArea: document.querySelector('.dictionary-area')!
});

/**
 * Display用の要素を抽出
 */
const extractDisplayElements = (elements: GUIElements): DisplayElements => ({
    outputDisplay: elements.outputDisplay,
    stackDisplay: elements.stackDisplay
});

/**
 * Dictionary用の要素を抽出
 */
const extractDictionaryElements = (elements: GUIElements): DictionaryElements => ({
    builtinWordsDisplay: elements.builtinWordsDisplay,
    customWordsDisplay: elements.customWordsDisplay
});

/**
 * Mobile用の要素を抽出
 */
const extractMobileElements = (elements: GUIElements): MobileElements => ({
    inputArea: elements.inputArea,
    outputArea: elements.outputArea,
    stackArea: elements.stackArea,
    dictionaryArea: elements.dictionaryArea
});

/**
 * ハイライト更新ロジック
 */
const checkStackHighlight = (content: string): boolean => {
    const stackRegex = /(\s|^)\.\.(\s|$)/;
    return stackRegex.test(content);
};

// ============================================================
// ファクトリ関数: GUI作成
// ============================================================

export const createGUI = (): GUI => {
    // 要素のキャッシュ（遅延初期化）
    let elements: GUIElements;

    // モジュールのインスタンス（遅延初期化）
    let display: Display;
    let editor: Editor;
    let dictionary: Dictionary;
    let mobile: MobileHandler;
    let persistence: Persistence;
    let executionController: ExecutionController;

    // ハイライト更新
    const updateHighlights = (content: string): void => {
        const hasStackWord = checkStackHighlight(content);

        if (hasStackWord) {
            elements.stackDisplay.classList.add('highlight-all');
        } else {
            elements.stackDisplay.classList.remove('highlight-all');
        }
    };

    // 全表示更新
    const updateAllDisplays = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            display.updateStack(window.ajisaiInterpreter.get_stack());
            dictionary.updateCustomWords(window.ajisaiInterpreter.get_custom_words_info());
            updateHighlights(elements.codeInput.value);
        } catch (error) {
            console.error('Failed to update display:', error);
            display.showError(new Error('Failed to update display.'));
        }
    };

    // ワーカー初期化
    const initializeWorkers = async (): Promise<void> => {
        try {
            display.showInfo('Initializing...', false);
            await WORKER_MANAGER.init();
            display.showInfo('Ready', true);
        } catch (error) {
            console.error('[GUI] Failed to initialize workers:', error);
            display.showError(new Error(`Failed to initialize parallel execution: ${error}`));
        }
    };

    // イベントリスナー設定
    const setupEventListeners = (): void => {
        // 実行ボタン
        elements.runBtn.addEventListener('click', () => {
            executionController.runCode(editor.getValue());
        });

        // クリアボタン
        elements.clearBtn.addEventListener('click', () => editor.clear());

        // テストボタン（存在する場合）
        elements.testBtn?.addEventListener('click', () => {
            mobile.updateView('execution');
            // TestRunnerは別途処理
            import('./test').then(({ createTestRunner }) => {
                const testRunner = createTestRunner({
                    showInfo: (text, append) => display.showInfo(text, append),
                    showError: (error) => display.showError(error),
                    updateDisplays: updateAllDisplays
                });
                testRunner.runAllTests();
            });
        });

        // エクスポート/インポートボタン
        elements.exportBtn?.addEventListener('click', () => persistence.exportCustomWords());
        elements.importBtn?.addEventListener('click', () => persistence.importCustomWords());

        // キーボードショートカット（コードエリア）
        elements.codeInput.addEventListener('keydown', (e) => {
            // Shift+Enter: 実行
            if (e.key === 'Enter' && e.shiftKey) {
                e.preventDefault();
                executionController.runCode(editor.getValue());
            }
            // Ctrl+Enter: ステップ実行
            if (e.key === 'Enter' && e.ctrlKey && !e.altKey && !e.shiftKey) {
                e.preventDefault();
                executionController.executeStep();
            }
        });

        // グローバルキーボードショートカット
        window.addEventListener('keydown', (e) => {
            // Escape: 中断
            if (e.key === 'Escape') {
                WORKER_MANAGER.abortAll();
                executionController.abortExecution();
                e.preventDefault();
                e.stopImmediatePropagation();
            }
            // Ctrl+Alt+Enter: リセット
            if (e.key === 'Enter' && e.ctrlKey && e.altKey) {
                if (confirm('Are you sure you want to reset the system?')) {
                    executionController.executeReset();
                }
                e.preventDefault();
                e.stopImmediatePropagation();
            }
        }, true);
    };

    // 初期化
    const init = async (): Promise<void> => {
        console.log('[GUI] Initializing GUI...');

        // DOM要素のキャッシュ
        elements = cacheElements();

        // Mobileハンドラの作成（先に作成してupdateViewを利用可能にする）
        mobile = createMobileHandler(extractMobileElements(elements));

        // Displayの作成と初期化
        display = createDisplay(extractDisplayElements(elements));
        display.init();

        // Persistenceの作成
        persistence = createPersistence({
            showError: (error) => display.showError(error),
            updateDisplays: updateAllDisplays,
            showInfo: (text, append) => display.showInfo(text, append)
        });
        await persistence.init();

        // Editorの作成
        editor = createEditor(elements.codeInput, {
            onContentChange: updateHighlights,
            onSwitchToInputMode: () => mobile.updateView('input')
        });

        // Dictionaryの作成
        dictionary = createDictionary(extractDictionaryElements(elements), {
            onWordClick: (word) => editor.insertWord(word),
            onUpdateDisplays: updateAllDisplays,
            onSaveState: () => persistence.saveCurrentState(),
            showInfo: (text, append) => display.showInfo(text, append)
        });

        // ExecutionControllerの作成
        executionController = createExecutionController(window.ajisaiInterpreter, {
            getEditorValue: () => editor.getValue(),
            clearEditor: (switchView) => editor.clear(switchView),
            setEditorValue: (value) => editor.setValue(value),
            insertEditorText: (text) => editor.insertText(text),
            showInfo: (text, append) => display.showInfo(text, append),
            showError: (error) => display.showError(error),
            showExecutionResult: (result) => display.showExecutionResult(result),
            updateDisplays: updateAllDisplays,
            saveState: () => persistence.saveCurrentState(),
            fullReset: () => persistence.fullReset(),
            updateView: (mode) => mobile.updateView(mode)
        });

        // イベントリスナーの設定
        setupEventListeners();

        // 組み込みワードのレンダリング
        dictionary.renderBuiltinWords();

        // 表示の更新
        updateAllDisplays();

        // 初期表示は入力モード
        mobile.updateView('input');

        // ワーカーの初期化
        await initializeWorkers();

        // データベースからの読み込み
        await persistence.loadDatabaseData();

        // 再度表示を更新（読み込み後）
        updateAllDisplays();

        console.log('[GUI] GUI initialization completed');
    };

    // ゲッター
    const getElements = (): GUIElements => elements;
    const getDisplay = (): Display => display;
    const getEditor = (): Editor => editor;
    const getDictionary = (): Dictionary => dictionary;
    const getMobile = (): MobileHandler => mobile;
    const getPersistence = (): Persistence => persistence;
    const getExecutionController = (): ExecutionController => executionController;

    return {
        init,
        updateAllDisplays,
        getElements,
        getDisplay,
        getEditor,
        getDictionary,
        getMobile,
        getPersistence,
        getExecutionController
    };
};

// シングルトンインスタンス（後方互換性のため）
export const GUI_INSTANCE = createGUI();

// 純粋関数をエクスポート（テスト用）
export const guiUtils = {
    cacheElements,
    extractDisplayElements,
    extractDictionaryElements,
    extractMobileElements,
    checkStackHighlight
};
