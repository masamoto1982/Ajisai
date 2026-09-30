import type { AjisaiInterpreter, CoreWordInfo, UserWordInfo } from '../wasm-interpreter-types';
import {
    checkWordMatchesFilter,
    compareWordName,
    createEmptyWordsElement,
    createNoResultsElement,
    createWordButtonElement,
    registerBackgroundClickListeners,
} from './dictionary-element-builders';
import { isFailure } from './interpreter-execution-utils';
import { toError } from './to-error';

export interface VocabularyElements {
    readonly coreWordsDisplay: HTMLElement;
    readonly userWordsDisplay: HTMLElement;
}

export interface VocabularyCallbacks {
    readonly onWordClick: (word: string) => void;
    readonly onBackgroundClick?: () => void;
    readonly onBackgroundDoubleClick?: () => void;
    readonly onUpdateDisplays?: () => void;
    readonly onSaveState?: () => Promise<void>;
    readonly showInfo?: (text: string, append: boolean) => void;
    readonly showError?: (error: Error) => void;
}

export interface VocabularyManager {
    readonly renderCoreWords: () => void;
    readonly updateUserWords: (userWordsInfo: UserWordInfo[]) => void;
    readonly updateSearchFilter: (filter: string) => void;
}

// The tooltip text for a User Word: what its author wrote for a reader
// (`#:contract`), or its source when nothing was written. Empty when the
// interpreter has neither.
const lookupUserWordTooltip = (interpreter: AjisaiInterpreter, name: string): string =>
    interpreter.lookup_word_description(name)
    ?? interpreter.lookup_word_definition(name)
    ?? '';

// DEL's refusal of a Word other Words still reference (spec/outcomes.json).
// Matched by category, never by the message, which is display text.
const DEPENDENCY_DELETE_CATEGORY = 'definitionConflict';

// A native popover (top-layer placement, light-dismiss on outside click or
// Escape), positioned at the cursor by `renderDeleteContextMenu`.
const createDeleteContextMenuElement = (onDelete: () => void): HTMLDivElement => {
    const menu = document.createElement('div');
    menu.className = 'context-menu';
    menu.popover = 'auto';

    const deleteButton = document.createElement('button');
    deleteButton.type = 'button';
    deleteButton.textContent = 'Delete';
    deleteButton.addEventListener('click', (event) => {
        event.stopPropagation();
        onDelete();
    });

    menu.appendChild(deleteButton);
    document.body.appendChild(menu);
    return menu;
};

export const createVocabularyManager = (
    interpreter: AjisaiInterpreter,
    elements: VocabularyElements,
    callbacks: VocabularyCallbacks
): VocabularyManager => {
    const { onWordClick, onBackgroundClick, onBackgroundDoubleClick, onUpdateDisplays, onSaveState, showInfo, showError } = callbacks;
    let activeContextWordName: string | null = null;

    const deleteContextMenu = createDeleteContextMenuElement(() => {
        if (!activeContextWordName) return;
        const selectedWordName = activeContextWordName;
        hideDeleteContextMenu();
        void deleteWord(selectedWordName);
    });

    const hideDeleteContextMenu = (): void => {
        if (deleteContextMenu.matches(':popover-open')) deleteContextMenu.hidePopover();
        activeContextWordName = null;
    };

    const renderDeleteContextMenu = (event: MouseEvent, wordName: string): void => {
        activeContextWordName = wordName;
        deleteContextMenu.style.left = `${event.clientX}px`;
        deleteContextMenu.style.top = `${event.clientY}px`;
        if (deleteContextMenu.matches(':popover-open')) deleteContextMenu.hidePopover();
        deleteContextMenu.showPopover();
    };

    // Reset the tracked word when the popover is light-dismissed (outside click /
    // Escape) so a later Delete can't act on a stale selection.
    deleteContextMenu.addEventListener('toggle', (event) => {
        if ((event as ToggleEvent).newState === 'closed') {
            activeContextWordName = null;
        }
    });

    for (const container of [elements.coreWordsDisplay, elements.userWordsDisplay]) {
        registerBackgroundClickListeners(container, onBackgroundClick, onBackgroundDoubleClick);
    }

    let searchFilter = '';
    let cachedUserWords: UserWordInfo[] = [];
    // Core words are fixed once WASM is loaded, so they are sorted once.
    let sortedCoreWordsCache: CoreWordInfo[] | null = null;

    const getSortedCoreWords = (): CoreWordInfo[] => {
        sortedCoreWordsCache ??= [...interpreter.collect_core_words_info()]
            .sort((a, b) => compareWordName(a[0], b[0]));
        return sortedCoreWordsCache;
    };

    // A referenced word is not deletable, and there is no way to override that:
    // the only route is to delete the dependents first. The interpreter names
    // the referencing words in its message, so it is surfaced as-is.
    const deleteWord = async (wordName: string): Promise<boolean> => {
        try {
            const result = await interpreter.execute(`'${wordName}' DEL`);
            if (isFailure(result)) {
                const message = result.message || 'Unknown error';
                if (result.aiDiagnostic?.category === DEPENDENCY_DELETE_CATEGORY) {
                    showInfo?.(message, true);
                } else {
                    showError?.(new Error(`Failed to delete word: ${message}`));
                }
                return false;
            }

            onUpdateDisplays?.();
            await onSaveState?.();
            showInfo?.(`Word '${wordName}' deleted`, true);
            return true;
        } catch (error) {
            showError?.(toError(error));
            return false;
        }
    };

    const renderCoreWordsSorted = (container: HTMLElement): void => {
        container.replaceChildren();
        container.classList.remove('is-empty');

        const matched = getSortedCoreWords().filter(([name]) =>
            checkWordMatchesFilter(name, searchFilter)
        );

        const fragment = document.createDocumentFragment();
        for (const [name, summary, syntaxExample] of matched) {
            // One authored line on what the Word is, then how it is called.
            const hoverText = [summary, syntaxExample].filter(Boolean).join('\n');
            fragment.appendChild(createWordButtonElement(
                name,
                'word-button core',
                () => onWordClick(name),
                hoverText
            ));
        }
        container.appendChild(fragment);

        if (searchFilter && matched.length === 0) {
            container.classList.add('is-empty');
            container.appendChild(createNoResultsElement());
        }
    };

    const renderUserWordButtons = (container: HTMLElement, words: UserWordInfo[]): void => {
        container.replaceChildren();

        const filteredWords = words.filter(([name]) =>
            checkWordMatchesFilter(name, searchFilter)
        );
        const sortedFiltered = [...filteredWords].sort(([a], [b]) =>
            compareWordName(a, b)
        );

        const fragment = document.createDocumentFragment();
        for (const [name, hasDependents] of sortedFiltered) {
            // Another User Word calls it, so DEL refuses it until that caller
            // is gone; it is coloured apart.
            const className = hasDependents
                ? 'word-button dependency'
                : 'word-button non-dependency';
            fragment.appendChild(createWordButtonElement(
                name,
                className,
                () => onWordClick(name),
                // Read at render rather than on hover: a tooltip has to carry
                // its text before the pointer arrives.
                lookupUserWordTooltip(interpreter, name),
                (event) => renderDeleteContextMenu(event, name)
            ));
        }
        container.appendChild(fragment);

        // A filter that matches nothing says so, whether or not there are
        // words to match: an empty User sheet under a filter used to show
        // neither message.
        if (searchFilter && filteredWords.length === 0) {
            container.classList.add('is-empty');
            container.appendChild(createNoResultsElement());
            return;
        }

        if (!searchFilter && words.length === 0) {
            container.classList.add('is-empty');
            container.appendChild(createEmptyWordsElement('No user words defined yet.'));
            return;
        }

        container.classList.remove('is-empty');
    };

    const renderCoreWords = (): void => {
        try {
            renderCoreWordsSorted(elements.coreWordsDisplay);
        } catch (error) {
            console.error('Failed to render core words:', error);
        }
    };

    const renderUserWords = (): void => {
        renderUserWordButtons(elements.userWordsDisplay, cachedUserWords);
    };

    const updateUserWords = (userWordsInfo: UserWordInfo[]): void => {
        cachedUserWords = userWordsInfo;
        renderUserWords();
    };

    const updateSearchFilter = (filter: string): void => {
        searchFilter = filter.trim();
        renderCoreWords();
        renderUserWords();
    };

    return {
        renderCoreWords,
        updateUserWords,
        updateSearchFilter
    };
};
