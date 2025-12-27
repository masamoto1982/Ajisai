// js/gui/editor.ts - エディタ管理（関数型スタイル）

// ============================================================
// 型定義
// ============================================================

export interface EditorCallbacks {
    readonly onContentChange?: (content: string) => void;
    readonly onSwitchToInputMode?: () => void;
}

export interface Editor {
    readonly getValue: () => string;
    readonly setValue: (value: string) => void;
    readonly clear: (switchView?: boolean) => void;
    readonly insertWord: (word: string) => void;
    readonly insertText: (text: string) => void;
    readonly focus: () => void;
    readonly setOnContentChange: (callback: (content: string) => void) => void;
}

// ============================================================
// 純粋関数: テキスト操作
// ============================================================

/**
 * テキストの指定位置に文字列を挿入した結果を返す
 */
const insertAt = (
    text: string,
    start: number,
    end: number,
    insertion: string
): string => text.substring(0, start) + insertion + text.substring(end);

/**
 * 最も内側の "[ ]" の位置を探す
 */
const findInnerBracketPosition = (text: string): number | null => {
    const pos = text.lastIndexOf('[ ]');
    return pos !== -1 ? pos + 2 : null; // "[ " の後の位置
};

/**
 * 新しいカーソル位置を計算
 */
const calculateCursorPosition = (
    basePosition: number,
    insertedText: string,
    preferInnerBracket: boolean
): number => {
    if (preferInnerBracket) {
        const innerPos = findInnerBracketPosition(insertedText);
        if (innerPos !== null) {
            return basePosition + innerPos;
        }
    }
    return basePosition + insertedText.length;
};

// ============================================================
// 副作用関数: DOM操作
// ============================================================

const setElementValue = (element: HTMLTextAreaElement, value: string): void => {
    element.value = value;
};

const focusElement = (element: HTMLTextAreaElement): void => {
    element.focus();
};

const setSelectionRange = (
    element: HTMLTextAreaElement,
    start: number,
    end: number
): void => {
    element.selectionStart = start;
    element.selectionEnd = end;
};

const getSelectionRange = (element: HTMLTextAreaElement): { start: number; end: number } => ({
    start: element.selectionStart,
    end: element.selectionEnd
});

// ============================================================
// ファクトリ関数: Editor作成
// ============================================================

export const createEditor = (
    element: HTMLTextAreaElement,
    callbacks: EditorCallbacks = {}
): Editor => {
    // コールバック（クロージャで保持）
    let onContentChangeCallback = callbacks.onContentChange;
    const switchToInputMode = callbacks.onSwitchToInputMode ?? (() => {});

    // イベントリスナーの設定
    const setupEventListeners = (): void => {
        element.addEventListener('focus', switchToInputMode);

        element.addEventListener('input', () => {
            if (onContentChangeCallback) {
                onContentChangeCallback(element.value);
            }
        });
    };

    // 初期化
    if (element.value.trim() === '') {
        setElementValue(element, '');
    }
    setupEventListeners();

    // 値の取得
    const getValue = (): string => element.value.trim();

    // 値の設定
    const setValue = (value: string): void => {
        setElementValue(element, value);
        switchToInputMode();
    };

    // クリア
    const clear = (switchView = true): void => {
        setElementValue(element, '');
        focusElement(element);
        if (switchView) {
            switchToInputMode();
        }
    };

    // ワードの挿入（カーソル位置に）
    const insertWord = (word: string): void => {
        const { start, end } = getSelectionRange(element);
        const newText = insertAt(element.value, start, end, word);

        setElementValue(element, newText);

        const newPos = start + word.length;
        setSelectionRange(element, newPos, newPos);

        focusElement(element);
        switchToInputMode();
    };

    // 入力支援テキストの挿入（最も内側の [ ] にカーソル配置）
    const insertText = (text: string): void => {
        const { start, end } = getSelectionRange(element);
        const newText = insertAt(element.value, start, end, text);

        setElementValue(element, newText);

        const cursorPos = calculateCursorPosition(start, text, true);
        setSelectionRange(element, cursorPos, cursorPos);

        focusElement(element);
        switchToInputMode();
    };

    // フォーカス
    const focus = (): void => {
        focusElement(element);
        switchToInputMode();
    };

    // コールバックの設定
    const setOnContentChange = (callback: (content: string) => void): void => {
        onContentChangeCallback = callback;
    };

    return {
        getValue,
        setValue,
        clear,
        insertWord,
        insertText,
        focus,
        setOnContentChange
    };
};

// 純粋関数をエクスポート（テスト用）
export const editorUtils = {
    insertAt,
    findInnerBracketPosition,
    calculateCursorPosition
};
