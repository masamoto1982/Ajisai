// js/gui/editor.ts

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

const insertAt = (
    text: string,
    start: number,
    end: number,
    insertion: string
): string => text.substring(0, start) + insertion + text.substring(end);

const findInnerBracketPosition = (text: string): number | null => {
    const pos = text.lastIndexOf('[ ]');
    return pos !== -1 ? pos + 2 : null;
};

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

const MOBILE_BREAKPOINT = 768;
const isMobile = (): boolean => window.innerWidth <= MOBILE_BREAKPOINT;

export const createEditor = (
    element: HTMLTextAreaElement,
    callbacks: EditorCallbacks = {}
): Editor => {
    let onContentChangeCallback = callbacks.onContentChange;
    const switchToInputMode = callbacks.onSwitchToInputMode ?? (() => {});

    const setupEventListeners = (): void => {
        element.addEventListener('focus', switchToInputMode);

        element.addEventListener('input', () => {
            if (onContentChangeCallback) {
                onContentChangeCallback(element.value);
            }
        });
    };

    if (element.value.trim() === '') {
        setElementValue(element, '');
    }
    setupEventListeners();

    const getValue = (): string => element.value.trim();

    const setValue = (value: string): void => {
        setElementValue(element, value);
        switchToInputMode();
    };

    const clear = (switchView = true): void => {
        setElementValue(element, '');
        // モバイルではfocusを避け、仮想キーボードの自動表示を防ぐ。
        if (!isMobile()) {
            focusElement(element);
        }
        if (switchView) {
            switchToInputMode();
        }
    };

    const insertWord = (word: string): void => {
        if (isMobile()) {
            // モバイル: 後置記法に従い、テキストの先頭に挿入する。
            // フォーカスなしではカーソル位置が保持されないため、
            // 常に先頭に挿入することで新しいワードが左側に配置される。
            const space = element.value.length > 0 ? ' ' : '';
            const newText = word + space + element.value;
            setElementValue(element, newText);
        } else {
            const { start, end } = getSelectionRange(element);
            const newText = insertAt(element.value, start, end, word);

            setElementValue(element, newText);

            const newPos = start + word.length;
            setSelectionRange(element, newPos, newPos);
            focusElement(element);
        }
        switchToInputMode();
    };

    const insertText = (text: string): void => {
        const { start, end } = getSelectionRange(element);
        const newText = insertAt(element.value, start, end, text);

        setElementValue(element, newText);

        const cursorPos = calculateCursorPosition(start, text, true);
        setSelectionRange(element, cursorPos, cursorPos);

        // モバイルではfocusを避け、仮想キーボードの自動表示を防ぐ。
        // ユーザーが明示的にテキストエディタをタップしたときのみキーボードを表示する。
        if (!isMobile()) {
            focusElement(element);
        }
        switchToInputMode();
    };

    const focus = (): void => {
        focusElement(element);
        switchToInputMode();
    };

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

export const editorUtils = {
    insertAt,
    findInnerBracketPosition,
    calculateCursorPosition
};
