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
        focusElement(element);
        if (switchView) {
            switchToInputMode();
        }
    };

    const insertWord = (word: string): void => {
        const { start, end } = getSelectionRange(element);
        const newText = insertAt(element.value, start, end, word);

        setElementValue(element, newText);

        const newPos = start + word.length;
        setSelectionRange(element, newPos, newPos);

        focusElement(element);
        switchToInputMode();
    };

    const insertText = (text: string): void => {
        const { start, end } = getSelectionRange(element);
        const newText = insertAt(element.value, start, end, text);

        setElementValue(element, newText);

        const cursorPos = calculateCursorPosition(start, text, true);
        setSelectionRange(element, cursorPos, cursorPos);

        focusElement(element);
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
