import { isMobileViewport } from '../platform/viewport';
import { formatAjisaiSource } from './code-formatter';

export interface EditorCallbacks {
    readonly onSwitchToInputMode?: () => void;
    readonly onRequestSuggestions?: (prefix: string) => string[];
}

export interface Editor {
    readonly extractValue: () => string;
    readonly updateValue: (value: string) => void;
    readonly clear: (switchView?: boolean) => void;
    readonly insertWord: (word: string) => void;
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
    /**
     * The word-shaped token touching the cursor (or the current selection's
     * start), the same extraction autocomplete uses. Empty when the cursor
     * sits on whitespace or punctuation with nothing to look up.
     */
    readonly getWordAtCursor: () => string;
}

const insertAt = (
    text: string,
    start: number,
    end: number,
    insertion: string
): string => text.substring(0, start) + insertion + text.substring(end);

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
// Two characters, not three. The mobile cheat sheet advertises autocomplete
// while typing, and a three-character floor silently withholds it for exactly
// the prefixes a phone typist most wants it for: `MA` for `MAP`, `SQ` for
// `SQRT`. Ten results are the ceiling either way (`MAX_SUGGESTIONS`), so a
// shorter prefix costs a longer list, not an unbounded one.
const MIN_SUGGESTION_TRIGGER_LENGTH = 2;
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
    const switchToInputMode = callbacks.onSwitchToInputMode ?? (() => {});
    const requestSuggestions = callbacks.onRequestSuggestions ?? (() => []);

    let currentSuggestions: string[] = [];
    let selectedSuggestionIndex = 0;
    let isSymbolMode = false;
    let lastKnownSelection = lookupSelectionRange(element);

    const textareaContainer = element.closest('.input-area');
    const suggestionPanel = document.createElement('div');
    suggestionPanel.className = 'editor-suggestions';
    suggestionPanel.setAttribute('role', 'listbox');
    suggestionPanel.style.display = 'none';
    textareaContainer?.appendChild(suggestionPanel);

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

        // Two panels, two anchors, and the symbol palette switches between them
        // on whether there is anything written yet.
        //
        // Word completions belong to the text being typed, so they always
        // follow the caret. The symbol palette opens at any token boundary,
        // which includes an empty editor — and there the caret is on line one,
        // so a caret anchor puts the palette square over the first lines of
        // the placeholder cheat sheet. On a phone that sheet is the only place
        // the touch gestures are written down, so tapping in to read how to
        // run something hid how to run something. Pinned to the bottom edge
        // the sheet reads from the top down into the palette instead, and the
        // corner buttons it would otherwise cover are themselves hidden while
        // the placeholder shows (`:placeholder-shown` in components.css).
        //
        // Once something *is* written those corner buttons are live, and the
        // bottom edge is where Format sits — so from the first character on,
        // the palette goes back to the caret.
        const anchorToBottomEdge = isSymbolMode && element.value.length === 0;

        if (anchorToBottomEdge) {
            suggestionPanel.style.top = 'auto';
            suggestionPanel.style.bottom = '0';
            suggestionPanel.style.left = '0';
            suggestionPanel.style.right = '0';
        } else {
            const { top, left } = computeCursorCoords(element);
            suggestionPanel.style.top = `${top}px`;
            suggestionPanel.style.left = `${left + 8}px`;
            suggestionPanel.style.right = 'auto';
            suggestionPanel.style.bottom = 'auto';
        }

        suggestionPanel.classList.toggle('editor-suggestions--symbols', isSymbolMode);

        suggestionPanel.innerHTML = '';
        currentSuggestions.forEach((suggestion, index) => {
            const button = document.createElement('button');
            button.type = 'button';
            button.className = 'editor-suggestion-item';
            button.setAttribute('role', 'option');
            // The item Tab will accept, moved by ArrowUp/ArrowDown.
            const selected = index === selectedSuggestionIndex;
            button.classList.toggle('is-selected', selected);
            button.setAttribute('aria-selected', String(selected));
            button.textContent = suggestion;
            button.addEventListener('mousedown', (e) => {
                e.preventDefault();
                applySuggestion(suggestion);
            });
            suggestionPanel.appendChild(button);
        });

        suggestionPanel.style.display = isSymbolMode ? 'grid' : 'block';
        suggestionPanel.querySelector('.is-selected')?.scrollIntoView({ block: 'nearest' });
    };

    const refreshSuggestions = (): void => {
        const cursorPos = element.selectionStart;
        const prevChar = cursorPos > 0 ? element.value[cursorPos - 1] ?? '' : '';
        const isTokenStart = cursorPos === 0 || /\s/.test(prevChar);
        const { token } = extractToken(element.value, element.selectionStart);
        if (isTokenStart && token.length === 0) {
            if (!isMobileViewport()) {
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
        element.value = insertAt(element.value, start, end, suggestion);
        const newPos = start + suggestion.length;
        updateSelectionRange(element, newPos, newPos);
        syncLastKnownSelection();
        hideSuggestions();
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
        element.value = '';
    }
    registerEventListeners();

    const extractValue = (): string => element.value.trim();

    const updateValue = (value: string): void => {
        element.value = value;
        const cursor = value.length;
        updateSelectionRange(element, cursor, cursor);
        syncLastKnownSelection();
        hideSuggestions();
        switchToInputMode();
    };

    // On a phone, taking focus raises the keyboard over the surface the user
    // is looking at, so focus is only kept where it already was.
    const refocus = (wasFocused: boolean): void => {
        if (wasFocused || !isMobileViewport()) element.focus();
    };

    const clear = (switchView = true): void => {
        const wasFocused = document.activeElement === element;
        element.value = '';
        refocus(wasFocused);
        updateSelectionRange(element, 0, 0);
        syncLastKnownSelection();
        hideSuggestions();
        if (switchView) {
            switchToInputMode();
        }
    };

    const insertWord = (word: string): void => {
        const wasFocused = document.activeElement === element;
        const { start, end } = lookupEditableSelectionRange();
        element.value = insertAt(element.value, start, end, word);
        const newPos = start + word.length;
        updateSelectionRange(element, newPos, newPos);
        syncLastKnownSelection();
        refocus(wasFocused);
        hideSuggestions();
    };

    const removeLastWord = (): void => {
        const wasFocused = document.activeElement === element;
        const { start } = lookupEditableSelectionRange();
        const before = element.value.substring(0, start);
        const after = element.value.substring(start);

        const trimmed = before.replace(/\S+\s*$/, '');
        element.value = trimmed + after;
        updateSelectionRange(element, trimmed.length, trimmed.length);
        syncLastKnownSelection();
        refocus(wasFocused);
        hideSuggestions();
    };

    const format = (): void => {
        const wasFocused = document.activeElement === element;
        const formatted = formatAjisaiSource(element.value);

        if (formatted !== element.value) {
            element.value = formatted;
            const cursor = formatted.length;
            updateSelectionRange(element, cursor, cursor);
            syncLastKnownSelection();
        }

        refocus(wasFocused);

        // Focusing the textarea re-runs the focus handler, which would reopen the
        // suggestion panel. Formatting is an explicit, whole-buffer action, so
        // close any suggestions afterwards rather than competing with input assist.
        hideSuggestions();
    };

    const focus = (): void => {
        element.focus();
        switchToInputMode();
        refreshSuggestions();
    };

    // Escape is bound window-wide to Abort on a capturing listener, so the
    // window handler asks here first and only aborts when there was no panel
    // to close.
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
        element.focus();
        updateSelectionRange(element, from, to);
        syncLastKnownSelection();
    };

    // Reads the last known caret position rather than the live one: anything
    // that takes focus off the textarea before Lookup runs leaves a blurred
    // textarea, and its own `selectionStart` is not something to rely on.
    // `lookupEditableSelectionRange` is the same caret every other
    // cursor-addressed operation here uses.
    const getWordAtCursor = (): string =>
        extractToken(element.value, lookupEditableSelectionRange().start).token;

    return {
        extractValue,
        updateValue,
        clear,
        insertWord,
        removeLastWord,
        format,
        focus,
        dismissSuggestions,
        revealRange,
        getWordAtCursor
    };
};
