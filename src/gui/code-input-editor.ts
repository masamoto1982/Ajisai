import { formatAjisaiSource } from './code-formatter';

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
    readonly format: () => void;
    readonly focus: () => void;
    /**
     * Close the suggestion panel if it is open. Returns whether there was
     * anything to close, so a shared Escape handler can spend the key on the
     * panel and leave Abort alone.
     */
    readonly dismissSuggestions: () => boolean;
    /**
     * Select the character range `[start, end)` of the *trimmed* source — the
     * text `extractValue` returns — and scroll it into view. Step mode uses
     * this to point at the token it is about to run; an empty range collapses
     * the selection, which is how step mode says it has finished.
     */
    readonly revealRange: (start: number, end: number) => void;
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

const MAX_SUGGESTIONS = 10;
const MIN_SUGGESTION_TRIGGER_LENGTH = 3;
const MOBILE_BREAKPOINT = 768;
const checkIsMobile = (): boolean => window.innerWidth <= MOBILE_BREAKPOINT;
const QUICK_SYMBOL_SUGGESTIONS: readonly string[] = Object.freeze([
    '(', ')', '[', ']', '{', '}',
    '<', '>', '+', '-', '*', '/',
    '%', '=', '!', '?', '&', '|',
    '~', '@', '#', '$', '_', '\\',
    ':', ';', '.', ',', "'", '"',
]);

const CARET_MIRROR_STYLE_PROPERTIES = [
    'borderBottomWidth',
    'borderLeftWidth',
    'borderRightWidth',
    'borderTopWidth',
    'boxSizing',
    'fontFamily',
    'fontSize',
    'fontStyle',
    'fontVariant',
    'fontWeight',
    'letterSpacing',
    'lineHeight',
    'paddingBottom',
    'paddingLeft',
    'paddingRight',
    'paddingTop',
    'tabSize',
    'textIndent',
    'textTransform',
    'wordSpacing',
] as const;

const copyCaretMirrorStyle = (
    mirror: HTMLDivElement,
    style: CSSStyleDeclaration
): void => {
    CARET_MIRROR_STYLE_PROPERTIES.forEach(property => {
        mirror.style[property] = style[property];
    });
};

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
    let isSymbolMode = false;
    let lastKnownSelection = lookupSelectionRange(element);

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

    const syncLastKnownSelection = (): void => {
        lastKnownSelection = lookupSelectionRange(element);
    };

    const lookupEditableSelectionRange = (): { start: number; end: number } => {
        if (document.activeElement === element) {
            syncLastKnownSelection();
        }
        return lastKnownSelection;
    };

    const hideSuggestions = (): void => {
        suggestionPanel.style.display = 'none';
        suggestionPanel.classList.remove('editor-suggestions--symbols');
        currentSuggestions = [];
        selectedSuggestionIndex = 0;
        isSymbolMode = false;
    };

    const computeCursorCoords = (el: HTMLTextAreaElement): { top: number; left: number } => {
        const style = getComputedStyle(el);
        const lineHeight = parseFloat(style.lineHeight) || 20;
        const mirror = document.createElement('div');
        const marker = document.createElement('span');

        copyCaretMirrorStyle(mirror, style);
        mirror.style.position = 'absolute';
        mirror.style.visibility = 'hidden';
        mirror.style.whiteSpace = 'pre-wrap';
        mirror.style.wordBreak = 'break-word';
        mirror.style.overflowWrap = 'break-word';
        mirror.style.overflow = 'hidden';
        mirror.style.left = '-9999px';
        mirror.style.top = '0';
        mirror.style.width = `${el.offsetWidth}px`;
        mirror.style.minHeight = '0';

        mirror.textContent = el.value.substring(0, el.selectionStart);
        marker.textContent = '\u200b';
        mirror.appendChild(marker);
        document.body.appendChild(mirror);

        const top = el.offsetTop + marker.offsetTop - el.scrollTop + lineHeight;
        const left = el.offsetLeft;

        mirror.remove();

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

        suggestionPanel.classList.toggle('editor-suggestions--symbols', isSymbolMode);

        suggestionPanel.innerHTML = '';
        currentSuggestions.forEach((suggestion) => {
            const button = document.createElement('button');
            button.type = 'button';
            button.className = 'editor-suggestion-item';
            button.textContent = suggestion;
            button.addEventListener('mousedown', (e) => {
                e.preventDefault();
                applySuggestion(suggestion);
            });
            suggestionPanel.appendChild(button);
        });

        suggestionPanel.style.display = isSymbolMode ? 'grid' : 'block';
    };

    const refreshSuggestions = (): void => {
        const cursorPos = element.selectionStart;
        const prevChar = cursorPos > 0 ? element.value[cursorPos - 1] ?? '' : '';
        const isTokenStart = cursorPos === 0 || /\s/.test(prevChar);
        const { token } = extractToken(element.value, element.selectionStart);
        if (isTokenStart && token.length === 0) {
            if (!checkIsMobile()) {
                hideSuggestions();
                return;
            }
            currentSuggestions = QUICK_SYMBOL_SUGGESTIONS.slice();
            isSymbolMode = true;
            selectedSuggestionIndex = 0;
            renderSuggestions();
            return;
        }

        if (token.length < MIN_SUGGESTION_TRIGGER_LENGTH) {
            hideSuggestions();
            return;
        }

        const suggestions = requestSuggestions(token)
            .filter(word => token.length === 0 || word.toLowerCase().startsWith(token.toLowerCase()))
            .slice(0, MAX_SUGGESTIONS);

        currentSuggestions = suggestions;
        isSymbolMode = false;
        selectedSuggestionIndex = 0;
        renderSuggestions();
    };

    const applySuggestion = (suggestion: string): void => {
        const { start, end } = extractToken(element.value, element.selectionStart);
        const newText = insertAt(element.value, start, end, suggestion);
        updateElementValue(element, newText);
        const newPos = start + suggestion.length;
        updateSelectionRange(element, newPos, newPos);
        syncLastKnownSelection();
        hideSuggestions();
        emitContentChange();
    };

    const registerEventListeners = (): void => {
        element.addEventListener('focus', () => {
            syncLastKnownSelection();
            switchToInputMode();
            refreshSuggestions();
        });

        element.addEventListener('blur', () => {
            syncLastKnownSelection();
            setTimeout(hideSuggestions, 100);
        });

        element.addEventListener('input', () => {
            syncLastKnownSelection();
            emitContentChange();
            refreshSuggestions();
        });

        element.addEventListener('select', syncLastKnownSelection);
        element.addEventListener('click', syncLastKnownSelection);
        element.addEventListener('keyup', syncLastKnownSelection);
        element.addEventListener('touchend', syncLastKnownSelection, { passive: true });

        element.addEventListener('keydown', (e) => {
            if (currentSuggestions.length === 0) return;

            if (e.key === 'ArrowDown') {
                e.preventDefault();
                selectedSuggestionIndex = (selectedSuggestionIndex + 1) % currentSuggestions.length;
                renderSuggestions();
            } else if (e.key === 'ArrowUp') {
                e.preventDefault();
                selectedSuggestionIndex = (selectedSuggestionIndex - 1 + currentSuggestions.length) % currentSuggestions.length;
                renderSuggestions();
            } else if (e.key === 'Tab') {
                // Tab accepts; Enter never does. A newline separates
                // statements in a definition body, so it is load-bearing
                // syntax in this language — an open suggestion panel must not
                // be able to eat one. It used to: typing `PRINT` opened the panel, and the
                // Enter meant to end the line accepted the completion instead,
                // so the next line's first token was appended to it (`PRINT3`).
                // Dismissing the panel instead keeps the following Enter,
                // whether the panel was wanted or not, a newline.
                e.preventDefault();
                applySuggestion(currentSuggestions[selectedSuggestionIndex]!);
            } else if (e.key === 'Enter') {
                hideSuggestions();
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
        const cursor = value.length;
        updateSelectionRange(element, cursor, cursor);
        syncLastKnownSelection();
        hideSuggestions();
        emitContentChange();
        switchToInputMode();
    };

    const clear = (switchView = true): void => {
        const wasFocused = document.activeElement === element;
        updateElementValue(element, '');
        if (wasFocused || !checkIsMobile()) {
            focusElement(element);
        }
        updateSelectionRange(element, 0, 0);
        syncLastKnownSelection();
        hideSuggestions();
        emitContentChange();
        if (switchView) {
            switchToInputMode();
        }
    };

    const insertWord = (word: string): void => {
        const wasFocused = document.activeElement === element;
        const { start, end } = lookupEditableSelectionRange();
        const newText = insertAt(element.value, start, end, word);

        updateElementValue(element, newText);

        const newPos = start + word.length;
        updateSelectionRange(element, newPos, newPos);
        syncLastKnownSelection();

        if (wasFocused || !checkIsMobile()) {
            focusElement(element);
        }
        hideSuggestions();
        emitContentChange();
    };

    const insertText = (text: string): void => {
        const wasFocused = document.activeElement === element;
        const { start, end } = lookupEditableSelectionRange();
        const newText = insertAt(element.value, start, end, text);

        updateElementValue(element, newText);

        const cursorPos = computeCursorPosition(start, text, true);
        updateSelectionRange(element, cursorPos, cursorPos);
        syncLastKnownSelection();

        if (wasFocused || !checkIsMobile()) {
            focusElement(element);
        }
        hideSuggestions();
        emitContentChange();
    };

    const removeLastWord = (): void => {
        const wasFocused = document.activeElement === element;
        const { start } = lookupEditableSelectionRange();
        const before = element.value.substring(0, start);
        const after = element.value.substring(start);

        const trimmed = before.replace(/\S+\s*$/, '');
        const newText = trimmed + after;

        updateElementValue(element, newText);
        updateSelectionRange(element, trimmed.length, trimmed.length);
        syncLastKnownSelection();

        if (wasFocused || !checkIsMobile()) {
            focusElement(element);
        }
        hideSuggestions();
        emitContentChange();
    };

    const format = (): void => {
        const wasFocused = document.activeElement === element;
        const formatted = formatAjisaiSource(element.value);

        if (formatted !== element.value) {
            updateElementValue(element, formatted);
            const cursor = formatted.length;
            updateSelectionRange(element, cursor, cursor);
            syncLastKnownSelection();
            emitContentChange();
        }

        if (wasFocused || !checkIsMobile()) {
            focusElement(element);
        }

        // Focusing the textarea re-runs the focus handler, which would reopen the
        // suggestion panel. Formatting is an explicit, whole-buffer action, so
        // close any suggestions afterwards rather than competing with input assist.
        hideSuggestions();
    };

    const focus = (): void => {
        focusElement(element);
        switchToInputMode();
        refreshSuggestions();
    };

    const registerContentChangeCallback = (callback: (content: string) => void): void => {
        onContentChangeCallback = callback;
    };

    // Escape is bound window-wide to Abort, on a capturing listener that stops
    // propagation, so the panel's own Escape branch never ran and the panel
    // could not be dismissed with the key every other editor dismisses it with.
    // The window handler now asks here first and only aborts when there was no
    // panel to close.
    const dismissSuggestions = (): boolean => {
        if (suggestionPanel.style.display === 'none') return false;
        hideSuggestions();
        return true;
    };

    // Offsets arrive measured against the trimmed source (what `extractValue`
    // hands out), so they are shifted by whatever leading whitespace the raw
    // value carries before being applied to the textarea. Selecting the range
    // is the whole mechanism: a textarea scrolls its selection into view, so
    // the token being stepped stays visible without an overlay to keep in sync
    // with the text.
    const revealRange = (start: number, end: number): void => {
        const raw = element.value;
        const offset = raw.length - raw.trimStart().length;
        const from = Math.min(offset + start, raw.length);
        const to = Math.min(offset + end, raw.length);
        focusElement(element);
        updateSelectionRange(element, from, to);
        syncLastKnownSelection();
    };

    return {
        extractValue,
        updateValue,
        clear,
        insertWord,
        insertText,
        removeLastWord,
        format,
        focus,
        dismissSuggestions,
        revealRange,
        registerContentChangeCallback
    };
};
