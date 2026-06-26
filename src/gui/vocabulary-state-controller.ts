

import {
    DEFAULT_WORD_INFO_MESSAGE,
    checkWordMatchesFilter,
    compareWordName,
    createEmptyWordsElement,
    createNoResultsElement,
    createWordButtonElement,
    registerBackgroundClickListeners,
    renderWordInfo,
    resetWordInfoDisplay,
} from './dictionary-element-builders';

export interface WordInfo {
    readonly dictionary: string;
    readonly name: string;
    readonly protected?: boolean;
}

export interface VocabularyElements {
    readonly builtInWordsDisplay: HTMLElement;
    readonly userWordsDisplay: HTMLElement;
    readonly builtInWordInfo: HTMLElement;
    readonly userWordInfo: HTMLElement;
    readonly userDictionarySelect: HTMLSelectElement;
}

export interface VocabularyCallbacks {
    readonly onWordClick: (word: string) => void;
    readonly onBackgroundClick?: () => void;
    readonly onBackgroundDoubleClick?: () => void;
    readonly onUpdateDisplays?: () => void;
    readonly onSaveState?: () => Promise<void>;
    readonly showInfo?: (text: string, append: boolean) => void;
}

export interface VocabularyManager {
    readonly renderBuiltInWords: () => void;
    readonly updateUserWords: (userWordsInfo: Array<[string, string, boolean]>) => void;
    readonly updateSearchFilter: (filter: string) => void;
    readonly setSelectedDictionary: (dictionary: string) => void;
}

const DICTIONARY_DISPLAY_NAMES: Readonly<Record<string, string>> = Object.freeze({
    'EXAMPLE': 'Example Words',
});
const REMOVED_USER_WORD_DICTIONARIES = new Set(['DEMO']);

export const formatDictionaryTabName = (pathName: string): string => {
    const displayName = DICTIONARY_DISPLAY_NAMES[pathName]
        ?? pathName
            .toLowerCase()
            .split(/[-_\s]+/)
            .filter(Boolean)
            .map(part => part.charAt(0).toUpperCase() + part.slice(1))
            .join(' ');
    return displayName.endsWith(' Words') ? displayName : `${displayName} Words`;
};

const createWordInfoFromTuple = (wordData: [string, string, boolean]): WordInfo => ({
    dictionary: wordData[0],
    name: wordData[1],
    protected: wordData[2] || false
});


const clearElement = (element: HTMLElement): void => {
    element.innerHTML = '';
};

const isCanonicalCoreWordName = (name: string): boolean => /^[A-Z][A-Z0-9-]*$/.test(name);

const DEPENDENCY_DELETE_ERROR = 'Cannot delete';

const createDeleteContextMenuElement = (
    onDelete: () => void
): HTMLDivElement => {
    const menu = document.createElement('div');
    // Native popover: top-layer placement and light-dismiss (outside click /
    // Escape) are handled by the browser, so no document-level listeners or
    // z-index management are needed. `inset: auto; margin: 0` lets the explicit
    // left/top below position it at the cursor (overriding the popover UA
    // centering).
    menu.popover = 'auto';
    Object.assign(menu.style, {
        position: 'fixed',
        inset: 'auto',
        margin: '0',
        minWidth: '7rem',
        padding: '0.125rem',
        backgroundColor: '#ffffff',
        border: '1px solid #c0c0c0',
        boxShadow: '0 2px 6px rgba(0, 0, 0, 0.15)'
    } satisfies Partial<CSSStyleDeclaration>);

    const deleteButton = document.createElement('button');
    deleteButton.type = 'button';
    deleteButton.textContent = 'Delete';
    Object.assign(deleteButton.style, {
        display: 'block',
        width: '100%',
        padding: '0.375rem 0.75rem',
        backgroundColor: 'transparent',
        color: '#000000',
        border: 'none',
        textAlign: 'left',
        cursor: 'pointer'
    } satisfies Partial<CSSStyleDeclaration>);
    deleteButton.addEventListener('mouseenter', () => {
        deleteButton.style.backgroundColor = '#e8e8e8';
    });
    deleteButton.addEventListener('mouseleave', () => {
        deleteButton.style.backgroundColor = 'transparent';
    });
    deleteButton.addEventListener('click', (event) => {
        event.stopPropagation();
        onDelete();
    });

    menu.appendChild(deleteButton);
    document.body.appendChild(menu);

    return menu;
};

export const createVocabularyManager = (
    elements: VocabularyElements,
    callbacks: VocabularyCallbacks
): VocabularyManager => {
    const { onWordClick, onBackgroundClick, onBackgroundDoubleClick, onUpdateDisplays, onSaveState, showInfo } = callbacks;
    const deleteContextMenu = createDeleteContextMenuElement(() => {
        if (!activeContextWordName) {
            return;
        }

        const selectedWordName = activeContextWordName;
        hideDeleteContextMenu();
        void confirmAndDeleteWord(selectedWordName);
    });
    let activeContextWordName: string | null = null;

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

    [elements.builtInWordsDisplay, elements.userWordsDisplay].forEach(container => {
        registerBackgroundClickListeners(container, onBackgroundClick, onBackgroundDoubleClick);
    });

    [elements.builtInWordInfo, elements.userWordInfo].forEach(resetWordInfoDisplay);


    let searchFilter = '';
    let cachedUserWords: Array<[string, string, boolean]> = [];
    let selectedDictionary = 'EXAMPLE';
    // Core words are fixed once WASM is loaded; fetching + canonical-filtering +
    // sorting them on every search keystroke was pure waste.
    let sortedCoreWordsCache: unknown[][] | null = null;

    const getSortedCoreWords = (): unknown[][] => {
        if (sortedCoreWordsCache) return sortedCoreWordsCache;

        const coreWords = window.ajisaiInterpreter.collect_core_listed_words_info();
        const filtered = coreWords.filter(
            wd =>
                Array.isArray(wd)
                && typeof wd[0] === 'string'
                && isCanonicalCoreWordName(wd[0])
        );

        const droppedCount = coreWords.length - filtered.length;
        if (droppedCount > 0) {
            console.info(`[Vocabulary] Filtered out ${droppedCount} non-canonical core word entries from WASM payload.`);
        }

        sortedCoreWordsCache = [...filtered].sort((a, b) =>
            compareWordName(a[0] as string, b[0] as string)
        );
        return sortedCoreWordsCache;
    };

    const selectDictionaryWords = (): WordInfo[] =>
        cachedUserWords
            .map(createWordInfoFromTuple)
            .filter(word => word.dictionary === selectedDictionary);

    const deleteWord = async (wordName: string, forceDelete: boolean): Promise<boolean> => {
        const deleteCode = forceDelete
            ? `! '${wordName}' DEL`
            : `'${wordName}' DEL`;

        try {
            const result = await window.ajisaiInterpreter.execute(deleteCode);
            if (result.status === 'ERROR') {
                if (!forceDelete && result.message?.includes(DEPENDENCY_DELETE_ERROR)) {
                    const confirmed = confirm(
                        `Word '${wordName}' is referenced by other user words. Force delete with ! ?`
                    );

                    if (confirmed) {
                        return deleteWord(wordName, true);
                    }

                    return false;
                }

                alert(`Failed to delete word: ${result.message}`);
                return false;
            }

            onUpdateDisplays?.();
            await onSaveState?.();
            showInfo?.(`Word '${wordName}' deleted`, true);
            return true;
        } catch (error) {
            alert(`Error deleting word: ${error}`);
            return false;
        }
    };

    const confirmAndDeleteWord = async (wordName: string): Promise<void> => {
        await deleteWord(wordName, false);
    };

    const renderBuiltInWordsSorted = (
        container: HTMLElement
    ): void => {
        clearElement(container);
        container.classList.remove('is-empty');

        const matched = getSortedCoreWords().filter(wd =>
            checkWordMatchesFilter(wd[0] as string, searchFilter)
        );

        const fragment = document.createDocumentFragment();
        matched.forEach(wordData => {
            const name = wordData[0] as string;
            const syntaxExample = (wordData[2] as string) || '';
            const button = createWordButtonElement(
                name,
                `word-button core`,
                () => onWordClick(name),
                () => { renderWordInfo(elements.builtInWordInfo, syntaxExample || DEFAULT_WORD_INFO_MESSAGE, !syntaxExample); },
                () => { resetWordInfoDisplay(elements.builtInWordInfo); }
            );

            fragment.appendChild(button);
        });
        container.appendChild(fragment);

        if (searchFilter && matched.length === 0) {
            container.classList.add('is-empty');
            container.appendChild(createNoResultsElement());
        }
    };

    const renderUserWordButtons = (
        container: HTMLElement,
        words: WordInfo[]
    ): void => {
        clearElement(container);
        resetWordInfoDisplay(elements.userWordInfo);


        const filteredWords = words.filter(wordInfo =>
            checkWordMatchesFilter(wordInfo.name, searchFilter)
        );


        const sortedFiltered = [...filteredWords].sort((a, b) =>
            compareWordName(a.name, b.name)
        );

        const fragment = document.createDocumentFragment();
        sortedFiltered.forEach(wordInfo => {
            const className = wordInfo.protected
                ? 'word-button dependency'
                : 'word-button non-dependency';

            const button = createWordButtonElement(
                wordInfo.name,
                className,
                () => onWordClick(wordInfo.dictionary === 'EXAMPLE' ? wordInfo.name : `${wordInfo.dictionary}@${wordInfo.name}`),
                () => {
                    const lookupName = `${wordInfo.dictionary}@${wordInfo.name}`;
                    const definition = window.ajisaiInterpreter?.lookup_word_definition(lookupName) ?? '';
                    renderWordInfo(
                        elements.userWordInfo,
                        definition || DEFAULT_WORD_INFO_MESSAGE,
                        !definition
                    );
                },
                () => { resetWordInfoDisplay(elements.userWordInfo); },
                (event) => renderDeleteContextMenu(event, `${wordInfo.dictionary}@${wordInfo.name}`)
            );

            fragment.appendChild(button);
        });
        container.appendChild(fragment);


        if (searchFilter && words.length > 0 && filteredWords.length === 0) {
            container.classList.add('is-empty');
            container.appendChild(createNoResultsElement());
            return;
        }

        if (!searchFilter && words.length === 0) {
            container.classList.add('is-empty');
            container.appendChild(createEmptyWordsElement('No user words defined yet.'));
            return;
        }

        container.classList.toggle('is-empty', sortedFiltered.length === 0);
    };

    const renderBuiltInWords = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            renderBuiltInWordsSorted(elements.builtInWordsDisplay);
        } catch (error) {
            console.error('Failed to render core words:', error);
        }
    };

    const updateUserWords = (
        userWordsInfo: Array<[string, string, boolean]>
    ): void => {

        cachedUserWords = userWordsInfo || [];
        const dictionaries = Array.from(new Set(cachedUserWords.map(([dictionary]) => dictionary)))
            .filter(dictionary => !REMOVED_USER_WORD_DICTIONARIES.has(dictionary.toUpperCase()))
            .sort();
        elements.userDictionarySelect.innerHTML = '';
        for (const dictionary of dictionaries.length > 0 ? dictionaries : ['EXAMPLE']) {
            const option = document.createElement('option');
            option.value = dictionary;
            option.textContent = formatDictionaryTabName(dictionary);
            elements.userDictionarySelect.appendChild(option);
        }
        if (!dictionaries.includes(selectedDictionary)) {
            selectedDictionary = dictionaries.includes('EXAMPLE') ? 'EXAMPLE' : (dictionaries[0] || 'EXAMPLE');
        }
        elements.userDictionarySelect.value = selectedDictionary;
        renderUserWordButtons(elements.userWordsDisplay, selectDictionaryWords());
    };

    const updateSearchFilter = (filter: string): void => {
        searchFilter = filter.trim();

        renderBuiltInWords();
        renderUserWordButtons(elements.userWordsDisplay, selectDictionaryWords());
    };

    const setSelectedDictionary = (dictionary: string): void => {
        if (!dictionary) return;
        const optionExists = Array.from(elements.userDictionarySelect.options).some(opt => opt.value === dictionary);
        if (!optionExists) return;
        selectedDictionary = dictionary;
        elements.userDictionarySelect.value = dictionary;
        renderUserWordButtons(elements.userWordsDisplay, selectDictionaryWords());
    };

    elements.userDictionarySelect.addEventListener('change', () => {
        selectedDictionary = elements.userDictionarySelect.value;
        renderUserWordButtons(elements.userWordsDisplay, selectDictionaryWords());
        void onSaveState?.();
    });

    return {
        renderBuiltInWords,
        updateUserWords,
        updateSearchFilter,
        setSelectedDictionary
    };
};
