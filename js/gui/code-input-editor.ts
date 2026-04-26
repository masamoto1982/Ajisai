

export interface EditorCallbacks {
    readonly onContentChange?: (content: string) => void;
    readonly onSwitchToInputMode?: () => void;
    readonly onRequestSuggestions?: (prefix: string) => string[];
}

export interface Editor {
    readonly extractValue: () => string;
    readonly updateValue: (value: string) => void;
    readonly clear: (switchView?: boolean) => void;
    readonly insertWord: (word: string) => void;
    readonly insertText: (text: string) => void;
    readonly removeLastWord: () => void;
    readonly focus: () => void;
    readonly registerContentChangeCallback: (callback: (content: string) => void) => void;
}

const insertAt = (
    text: string,
    start: number,
    end: number,
    insertion: string
): string => text.substring(0, start) + insertion + text.substring(end);

const locateInnerBracketPosition = (text: string): number | null => {
    const pos = text.lastIndexOf('[ ]');
    return pos !== -1 ? pos + 2 : null;
};

const computeCursorPosition = (
    basePosition: number,
    insertedText: string,
    preferInnerBracket: boolean
): number => {
    if (preferInnerBracket) {
        const innerPos = locateInnerBracketPosition(insertedText);
        if (innerPos !== null) {
            return basePosition + innerPos;
        }
    }
    return basePosition + insertedText.length;
};

const updateElementValue = (element: HTMLTextAreaElement, value: string): void => {
    element.value = value;
};

const focusElement = (element: HTMLTextAreaElement): void => {
    element.focus();
};

const updateSelectionRange = (
    element: HTMLTextAreaElement,
    start: number,
    end: number
): void => {
    element.selectionStart = start;
    element.selectionEnd = end;
};

const lookupSelectionRange = (element: HTMLTextAreaElement): { start: number; end: number } => ({
    start: element.selectionStart,
    end: element.selectionEnd
});

const MOBILE_BREAKPOINT = 768;
const MAX_SUGGESTIONS = 10;
const MIN_SUGGESTION_TRIGGER_LENGTH = 3;
const checkIsMobile = (): boolean => window.innerWidth <= MOBILE_BREAKPOINT;

const extractToken = (
    text: string,
    cursorPosition: number
): { token: string; start: number; end: number } => {
    const safeCursor = Math.max(0, Math.min(cursorPosition, text.length));
    const left = text.slice(0, safeCursor);
    const right = text.slice(safeCursor);
    const leftMatch = left.match(/[A-Za-z0-9_?!+\-*/<>=]+$/);
    const rightMatch = right.match(/^[A-Za-z0-9_?!+\-*/<>=]*/);

    const tokenLeft = leftMatch?.[0] ?? '';
    const tokenRight = rightMatch?.[0] ?? '';

    return {
        token: `${tokenLeft}${tokenRight}`,
        start: safeCursor - tokenLeft.length,
        end: safeCursor + tokenRight.length
    };
};

export const createEditor = (
    element: HTMLTextAreaElement,
    callbacks: EditorCallbacks = {}
): Editor => {
    let onContentChangeCallback = callbacks.onContentChange;
    const switchToInputMode = callbacks.onSwitchToInputMode ?? (() => {});
    const requestSuggestions = callbacks.onRequestSuggestions ?? (() => []);

    let currentSuggestions: string[] = [];
    let selectedSuggestionIndex = 0;

    const textareaContainer = element.closest('.input-area');
    const suggestionPanel = document.createElement('div');
    suggestionPanel.className = 'editor-suggestions';
    suggestionPanel.style.display = 'none';
    textareaContainer?.appendChild(suggestionPanel);

    const emitContentChange = (): void => {
        if (onContentChangeCallback) {
            onContentChangeCallback(element.value);
        }
    };

    const hideSuggestions = (): void => {
        suggestionPanel.style.display = 'none';
        currentSuggestions = [];
        selectedSuggestionIndex = 0;
    };

    const computeCursorCoords = (el: HTMLTextAreaElement): { top: number; left: number } => {
        const lineHeight = parseFloat(getComputedStyle(el).lineHeight) || 20;
        const paddingTop = parseFloat(getComputedStyle(el).paddingTop) || 0;
        const text = el.value.substring(0, el.selectionStart);
        const lines = text.split('\n');
        const lineIndex = lines.length - 1;
        const top = paddingTop + lineIndex * lineHeight + lineHeight;
        const left = 0;
        return { top, left };
    };

    const renderSuggestions = (): void => {
        if (currentSuggestions.length === 0) {
            hideSuggestions();
            return;
        }

        const { top, left } = computeCursorCoords(element);
        suggestionPanel.style.top = `${top}px`;
        suggestionPanel.style.left = `${left + 8}px`;
        suggestionPanel.style.bottom = 'auto';

        suggestionPanel.innerHTML = '';
        currentSuggestions.forEach((suggestion, index) => {
            const button = document.createElement('button');
            button.type = 'button';
            button.className = `editor-suggestion-item${index === selectedSuggestionIndex ? ' active' : ''}`;
            button.textContent = suggestion;
            button.addEventListener('mousedown', (e) => {
                e.preventDefault();
                applySuggestion(suggestion);
            });
            suggestionPanel.appendChild(button);
        });

        suggestionPanel.style.display = 'block';
    };

    const refreshSuggestions = (): void => {
        const { token } = extractToken(element.value, element.selectionStart);
        if (token.length < MIN_SUGGESTION_TRIGGER_LENGTH) {
            hideSuggestions();
            return;
        }

        const suggestions = requestSuggestions(token)
            .filter(word => token.length === 0 || word.toLowerCase().startsWith(token.toLowerCase()))
            .slice(0, MAX_SUGGESTIONS);

        currentSuggestions = suggestions;
        selectedSuggestionIndex = 0;
        renderSuggestions();
    };

    const applySuggestion = (suggestion: string): void => {
        const { start, end } = extractToken(element.value, element.selectionStart);
        const newText = insertAt(element.value, start, end, suggestion);
        updateElementValue(element, newText);
        const newPos = start + suggestion.length;
        updateSelectionRange(element, newPos, newPos);
        hideSuggestions();
        emitContentChange();
    };

    const registerEventListeners = (): void => {
        element.addEventListener('focus', () => {
            switchToInputMode();
            refreshSuggestions();
        });

        element.addEventListener('blur', () => {
            setTimeout(hideSuggestions, 100);
        });

        element.addEventListener('input', () => {
            emitContentChange();
            refreshSuggestions();
        });

        element.addEventListener('keydown', (e) => {
            if (e.key === ' ' && e.ctrlKey) {
                e.preventDefault();
                refreshSuggestions();
                return;
            }

            if (currentSuggestions.length === 0) return;

            if (e.key === 'ArrowDown') {
                e.preventDefault();
                selectedSuggestionIndex = (selectedSuggestionIndex + 1) % currentSuggestions.length;
                renderSuggestions();
            } else if (e.key === 'ArrowUp') {
                e.preventDefault();
                selectedSuggestionIndex = (selectedSuggestionIndex - 1 + currentSuggestions.length) % currentSuggestions.length;
                renderSuggestions();
            } else if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !e.altKey)) {
                e.preventDefault();
                applySuggestion(currentSuggestions[selectedSuggestionIndex]!);
            } else if (e.key === 'Escape') {
                hideSuggestions();
            }
        });
    };

    if (element.value.trim() === '') {
        updateElementValue(element, '');
    }
    registerEventListeners();

    const extractValue = (): string => element.value.trim();

    const updateValue = (value: string): void => {
        updateElementValue(element, value);
        hideSuggestions();
        emitContentChange();
        switchToInputMode();
    };

    const clear = (switchView = true): void => {
        updateElementValue(element, '');
        if (!checkIsMobile()) {
            focusElement(element);
        }
        hideSuggestions();
        emitContentChange();
        if (switchView) {
            switchToInputMode();
        }
    };

    const insertWord = (word: string): void => {
        const { start, end } = lookupSelectionRange(element);
        const newText = insertAt(element.value, start, end, word);

        updateElementValue(element, newText);

        const newPos = start + word.length;
        updateSelectionRange(element, newPos, newPos);

        if (!checkIsMobile()) {
            focusElement(element);
        }
        hideSuggestions();
        emitContentChange();
    };

    const insertText = (text: string): void => {
        const { start, end } = lookupSelectionRange(element);
        const newText = insertAt(element.value, start, end, text);

        updateElementValue(element, newText);

        const cursorPos = computeCursorPosition(start, text, true);
        updateSelectionRange(element, cursorPos, cursorPos);

        if (!checkIsMobile()) {
            focusElement(element);
        }
        hideSuggestions();
        emitContentChange();
    };

    const removeLastWord = (): void => {
        const { start } = lookupSelectionRange(element);
        const before = element.value.substring(0, start);
        const after = element.value.substring(start);

        const trimmed = before.replace(/\S+\s*$/, '');
        const newText = trimmed + after;

        updateElementValue(element, newText);
        updateSelectionRange(element, trimmed.length, trimmed.length);

        if (!checkIsMobile()) {
            focusElement(element);
        }
        hideSuggestions();
        emitContentChange();
    };

    const focus = (): void => {
        focusElement(element);
        switchToInputMode();
        refreshSuggestions();
    };

    const registerContentChangeCallback = (callback: (content: string) => void): void => {
        onContentChangeCallback = callback;
    };

    return {
        extractValue,
        updateValue,
        clear,
        insertWord,
        insertText,
        removeLastWord,
        focus,
        registerContentChangeCallback
    };
};
