import { isMobileViewport } from '../platform/viewport';
import { formatAjisaiSource } from './code-formatter';
import { countLeadingSourceWhitespace, isSourceWhitespace, trimSource } from './source-atoms';

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

// A Dictionary word written at [start, end) of `text`, with a space on either
// side where it would otherwise run into a neighbouring name: clicking ADD and
// then SQRT wrote `ADDSQRT`, one unknown Word, where the reader had asked for
// two. Whitespace (the space written by a click between the word buttons) is
// written as it is. Exported for `code-input-editor.test.ts`.
export const separateWord = (text: string, start: number, end: number, word: string): {
    readonly insertion: string;
    readonly caretOffset: number;
} => {
    if (trimSource(word) === '') return { insertion: word, caretOffset: word.length };
    const before = text.substring(0, start);
    const after = text.substring(end);
    const lead = before !== '' && !isSourceWhitespace(before[before.length - 1]!) ? ' ' : '';
    const trail = after !== '' && !isSourceWhitespace(after[0]!) ? ' ' : '';
    return { insertion: lead + word + trail, caretOffset: lead.length + word.length };
};

// Replace [start, end) of the textarea with `text` the way typing would, so
// the change joins the browser's own undo history. Assigning `value` — which
// every programmatic edit here used to do — empties that history, so one
// dictionary click or Format made every earlier Ctrl+Z impossible. The
// browser's editing command acts on the focused field only; an unfocused one
// (a phone, where focus would raise the keyboard) takes `setRangeText`, which
// keeps the rest of the text and the caret as the same edit would.
const replaceRange = (
    element: HTMLTextAreaElement,
    start: number,
    end: number,
    text: string
): void => {
    if (document.activeElement === element && typeof document.execCommand === 'function') {
        element.setSelectionRange(start, end);
        if (start === end && text === '') return;
        if (document.execCommand(text === '' ? 'delete' : 'insertText', false, text)) return;
    }
    element.setRangeText(text, start, end, 'end');
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
// Two characters. The mobile cheat sheet advertises autocomplete while
// typing, and a longer floor silently withholds it for exactly the prefixes a
// phone typist most wants it for: `MA` for `MAP`, `SQ` for `SQRT`. Ten
// results are the ceiling either way (`MAX_SUGGESTIONS`), so a shorter prefix
// costs a longer list, not an unbounded one.
const MIN_SUGGESTION_TRIGGER_LENGTH = 2;

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

// The word the cursor touches, as Lookup and the suggestion panel read it.
// A name is any run of non-whitespace (spec/grammar.json,
// characterClasses.nameCharacter), so a User Word spelled `X.Y`, `A|B` or
// `合計` is one word here, and Lookup identifies the entry execution would —
// an ASCII letter class used to hand Lookup `X` for a cursor on `X.Y`, and
// nothing at all for a Japanese name. The one split made is the formatter's:
// `[` and `]` must stand alone whatever they are glued to, so the word in
// `[SQRT]` is `SQRT`, and a cursor on a bracket, like one on whitespace,
// touches no word. Exported for `code-input-editor.test.ts`.
export const extractToken = (
    text: string,
    cursorPosition: number
): { token: string; start: number; end: number } => {
    const safeCursor = Math.max(0, Math.min(cursorPosition, text.length));
    let start = safeCursor;
    while (start > 0 && !isSourceWhitespace(text[start - 1]!)) start -= 1;
    let end = safeCursor;
    while (end < text.length && !isSourceWhitespace(text[end]!)) end += 1;

    let pieceStart = start;
    for (const piece of text.slice(start, end).match(/[[\]]|[^[\]]+/g) ?? []) {
        const pieceEnd = pieceStart + piece.length;
        if (piece !== '[' && piece !== ']' && pieceStart <= safeCursor && safeCursor <= pieceEnd) {
            return { token: piece, start: pieceStart, end: pieceEnd };
        }
        pieceStart = pieceEnd;
    }
    return { token: '', start: safeCursor, end: safeCursor };
};

export const createEditor = (
    element: HTMLTextAreaElement,
    callbacks: EditorCallbacks = {}
): Editor => {
    const switchToInputMode = callbacks.onSwitchToInputMode ?? (() => {});
    const requestSuggestions = callbacks.onRequestSuggestions ?? (() => []);

    let currentSuggestions: string[] = [];
    let selectedSuggestionIndex = 0;
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
        currentSuggestions = [];
        selectedSuggestionIndex = 0;
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

        // Completions belong to the text being typed, so they follow the caret.
        const { top, left } = computeCursorCoords(element);
        suggestionPanel.style.top = `${top}px`;
        suggestionPanel.style.left = `${left + 8}px`;

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
            // The press is swallowed so focus, and with it the phone keyboard,
            // stays on the editor; the click that follows still arrives.
            button.addEventListener('pointerdown', (e) => e.preventDefault());
            button.addEventListener('click', () => applySuggestion(suggestion));
            suggestionPanel.appendChild(button);
        });

        suggestionPanel.style.display = 'block';
        suggestionPanel.querySelector('.is-selected')?.scrollIntoView({ block: 'nearest' });
    };

    const refreshSuggestions = (): void => {
        // Symbols come from the device's own keyboard; this panel completes
        // Words, from their second character.
        const { token } = extractToken(element.value, element.selectionStart);
        if (token.length < MIN_SUGGESTION_TRIGGER_LENGTH) {
            hideSuggestions();
            return;
        }

        const suggestions = requestSuggestions(token)
            .filter(word => word.toLowerCase().startsWith(token.toLowerCase()))
            .slice(0, MAX_SUGGESTIONS);

        currentSuggestions = suggestions;
        selectedSuggestionIndex = 0;
        renderSuggestions();
    };

    const applySuggestion = (suggestion: string): void => {
        const { start, end } = extractToken(element.value, element.selectionStart);
        replaceRange(element, start, end, suggestion);
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
                // be able to eat one (an Enter that accepted the completion for
                // `PRINT` would glue the next line's first token to it:
                // `PRINT3`). Dismissing the panel instead keeps the following
                // Enter, whether the panel was wanted or not, a newline.
                e.preventDefault();
                applySuggestion(currentSuggestions[selectedSuggestionIndex]!);
            } else if (e.key === 'Enter') {
                hideSuggestions();
            } else if (e.key === 'Escape') {
                hideSuggestions();
            }
        });
    };

    if (trimSource(element.value) === '') {
        element.value = '';
    }
    registerEventListeners();

    const extractValue = (): string => trimSource(element.value);

    const updateValue = (value: string): void => {
        replaceRange(element, 0, element.value.length, value);
        const cursor = value.length;
        updateSelectionRange(element, cursor, cursor);
        syncLastKnownSelection();
        hideSuggestions();
        switchToInputMode();
    };

    // On a phone, taking focus raises the keyboard over the surface the user
    // is looking at, so focus is only kept where it already was. A field that
    // is not on screen cannot take focus at all: on desktop the left column
    // can show Output while the right shows Dictionary (a run that changed
    // both leaves it so), and a Dictionary word clicked then was written into
    // the hidden editor, unseen and outside the undo history. The Input
    // surface is shown first where focus was refused, so the edit lands
    // where the spec puts it — on the Input surface, as an ordinary edit.
    const refocus = (wasFocused: boolean): void => {
        if (!wasFocused && isMobileViewport()) return;
        element.focus();
        if (document.activeElement !== element) {
            switchToInputMode();
            element.focus();
        }
    };

    const clear = (switchView = true): void => {
        const wasFocused = document.activeElement === element;
        refocus(wasFocused);
        replaceRange(element, 0, element.value.length, '');
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
        const { insertion, caretOffset } = separateWord(element.value, start, end, word);
        // Focus first where it will be kept anyway, so the edit is the
        // browser's own and Ctrl+Z takes it back.
        refocus(wasFocused);
        replaceRange(element, start, end, insertion);
        const newPos = start + caretOffset;
        updateSelectionRange(element, newPos, newPos);
        syncLastKnownSelection();
        hideSuggestions();
    };

    const removeLastWord = (): void => {
        const wasFocused = document.activeElement === element;
        const { start } = lookupEditableSelectionRange();
        const before = element.value.substring(0, start);

        // The last word before the caret and the whitespace after it, by the
        // grammar's whitespace class; nothing is taken when no word precedes.
        let wordEnd = before.length;
        while (wordEnd > 0 && isSourceWhitespace(before[wordEnd - 1]!)) wordEnd -= 1;
        let wordStart = wordEnd;
        while (wordStart > 0 && !isSourceWhitespace(before[wordStart - 1]!)) wordStart -= 1;
        const cut = wordStart === wordEnd ? start : wordStart;
        refocus(wasFocused);
        replaceRange(element, cut, start, '');
        updateSelectionRange(element, cut, cut);
        syncLastKnownSelection();
        hideSuggestions();
    };

    const format = (): void => {
        const wasFocused = document.activeElement === element;
        const formatted = formatAjisaiSource(element.value);

        refocus(wasFocused);

        if (formatted !== element.value) {
            replaceRange(element, 0, element.value.length, formatted);
            const cursor = formatted.length;
            updateSelectionRange(element, cursor, cursor);
            syncLastKnownSelection();
        }


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
        const offset = countLeadingSourceWhitespace(raw);
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
