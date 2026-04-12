

import {
    checkWordMatchesFilter,
    compareWordName,
    createEmptyWordsElement,
    createNoResultsElement,
    createWordButtonElement,
    registerBackgroundClickListeners,
} from './dictionary-element-builders';

export interface WordInfo {
    readonly dictionary: string;
    readonly name: string;
    readonly description?: string | null;
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
    readonly updateUserWords: (userWordsInfo: Array<[string, string, string | null, boolean]>) => void;
    readonly updateSearchFilter: (filter: string) => void;
}

const DICTIONARY_DISPLAY_NAMES: Readonly<Record<string, string>> = Object.freeze({
    'DEMO': 'Demonstration',
});

export const formatDictionaryTabName = (pathName: string): string => {
    const displayName = DICTIONARY_DISPLAY_NAMES[pathName]
        ?? pathName.charAt(0).toUpperCase() + pathName.slice(1).toLowerCase();
    return `${displayName} word`;
};

const SYMBOL_MAP: Readonly<Record<string, string>> = Object.freeze({
    'VSTART': '[', 'VEND': ']', 'BSTART': '{', 'BEND': '}',
    'NIL': 'nil', 'ADD': '+', 'SUB': '-', 'MUL': '*', 'DIV': '/',
    'LT': '<', 'LE': '<=', 'GT': '>', 'GE': '>=', 'EQ': '=',
    'AND': 'and', 'OR': 'or', 'NOT': 'not',
});

const deserializeWordName = (name: string): string | null => {
    if (name.match(/^W_[0-9A-F]+$/)) return null;
    if (!name.includes('_')) return null;

    const decoded = name.split('_').map(part => {
        if (part.startsWith('STR_')) {
            return `"${part.substring(4).replace(/_/g, ' ')}"`;
        }
        return SYMBOL_MAP[part] ?? part.toLowerCase();
    }).join(' ');

    return `≈ ${decoded}`;
};

const createWordInfoFromTuple = (wordData: [string, string, string | null, boolean]): WordInfo => ({
    dictionary: wordData[0],
    name: wordData[1],
    description: wordData[2] || deserializeWordName(wordData[1]) || wordData[1],
    protected: wordData[3] || false
});


const clearElement = (element: HTMLElement): void => {
    element.innerHTML = '';
};

const DEFAULT_WORD_INFO_MESSAGE = 'Hover over a word button to view its usage.';

const resolveDisplayDescription = (wordInfo: WordInfo): string | null => {
    const desc = wordInfo.description;
    if (!desc || desc === wordInfo.name) return null;
    return desc;
};

const renderWordInfo = (element: HTMLElement, text: string, isPlaceholder = false): void => {
    element.textContent = text;
    element.classList.toggle('is-placeholder', isPlaceholder);
};

const resetWordInfoDisplay = (element: HTMLElement): void => {
    renderWordInfo(element, DEFAULT_WORD_INFO_MESSAGE, true);
};

const DEPENDENCY_DELETE_ERROR = 'Cannot delete';

const createDeleteContextMenuElement = (
    onDelete: () => void
): HTMLDivElement => {
    const menu = document.createElement('div');
    menu.hidden = true;
    Object.assign(menu.style, {
        position: 'fixed',
        zIndex: '1000',
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
        deleteContextMenu.hidden = true;
        activeContextWordName = null;
    };

    const renderDeleteContextMenu = (event: MouseEvent, wordName: string): void => {
        activeContextWordName = wordName;
        deleteContextMenu.hidden = false;
        deleteContextMenu.style.left = `${event.clientX}px`;
        deleteContextMenu.style.top = `${event.clientY}px`;
    };

    document.addEventListener('click', () => {
        hideDeleteContextMenu();
    });
    document.addEventListener('contextmenu', (event) => {
        if (!(event.target instanceof HTMLElement) || !event.target.closest('.word-button')) {
            hideDeleteContextMenu();
        }
    });
    window.addEventListener('blur', hideDeleteContextMenu);

    [elements.builtInWordsDisplay, elements.userWordsDisplay].forEach(container => {
        registerBackgroundClickListeners(container, onBackgroundClick, onBackgroundDoubleClick);
    });

    [elements.builtInWordInfo, elements.userWordInfo].forEach(resetWordInfoDisplay);


    let searchFilter = '';
    let cachedUserWords: Array<[string, string, string | null, boolean]> = [];
    let selectedDictionary = 'DEMO';

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
        container: HTMLElement,
        coreWords: unknown[][]
    ): void => {
        clearElement(container);


        const filtered = coreWords.filter(
            (wd): wd is unknown[] =>
                Array.isArray(wd) && wd[0] !== ';' && wd[0] !== '"'
        );


        const sorted = [...filtered].sort((a, b) =>
            compareWordName(a[0] as string, b[0] as string)
        );


        const matched = sorted.filter(wd =>
            checkWordMatchesFilter(wd[0] as string, searchFilter)
        );


        matched.forEach(wordData => {
            const name = wordData[0] as string;
            const description = (wordData[1] as string) || name;
            const syntaxExample = (wordData[2] as string) || '';
            const signatureType = (wordData[3] as string) || 'none';
            const sigClass = signatureType !== 'none' ? ` signature-${signatureType}` : '';

            const button = createWordButtonElement(
                name,
                description,
                `word-button core${sigClass}`,
                () => onWordClick(name),
                () => { renderWordInfo(elements.builtInWordInfo, syntaxExample || DEFAULT_WORD_INFO_MESSAGE, !syntaxExample); },
                () => { resetWordInfoDisplay(elements.builtInWordInfo); }
            );

            container.appendChild(button);
        });

        if (searchFilter && matched.length === 0) {
            container.appendChild(createNoResultsElement());
        }
    };

    const renderUserWordButtons = (
        container: HTMLElement,
        words: WordInfo[]
    ): void => {
        clearElement(container);


        const filteredWords = words.filter(wordInfo =>
            checkWordMatchesFilter(wordInfo.name, searchFilter)
        );


        const sortedFiltered = [...filteredWords].sort((a, b) =>
            compareWordName(a.name, b.name)
        );

        sortedFiltered.forEach(wordInfo => {
            const className = wordInfo.protected
                ? 'word-button dependency'
                : 'word-button non-dependency';

            const button = createWordButtonElement(
                wordInfo.name,
                wordInfo.description || '',
                className,
                () => onWordClick(wordInfo.dictionary === 'DEMO' ? wordInfo.name : `${wordInfo.dictionary}@${wordInfo.name}`),
                () => {
                    const lookupName = `${wordInfo.dictionary}@${wordInfo.name}`;
                    const definition = window.ajisaiInterpreter?.lookup_word_definition(lookupName);
                    const desc = resolveDisplayDescription(wordInfo);
                    const displayText = desc
                        ? `${desc}\n\n${definition ?? ''}`.trim()
                        : (definition ?? '');
                    renderWordInfo(
                        elements.userWordInfo,
                        displayText || DEFAULT_WORD_INFO_MESSAGE,
                        !displayText
                    );
                },
                () => { resetWordInfoDisplay(elements.userWordInfo); },
                (event) => renderDeleteContextMenu(event, `${wordInfo.dictionary}@${wordInfo.name}`)
            );

            container.appendChild(button);
        });



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
            const coreWords = window.ajisaiInterpreter.collect_core_words_info();
            renderBuiltInWordsSorted(elements.builtInWordsDisplay, coreWords);
        } catch (error) {
            console.error('Failed to render core words:', error);
        }
    };

    const updateUserWords = (
        userWordsInfo: Array<[string, string, string | null, boolean]>
    ): void => {

        cachedUserWords = userWordsInfo || [];
        const dictionaries = Array.from(new Set(cachedUserWords.map(([dictionary]) => dictionary))).sort();
        elements.userDictionarySelect.innerHTML = '';
        for (const dictionary of dictionaries.length > 0 ? dictionaries : ['DEMO']) {
            const option = document.createElement('option');
            option.value = dictionary;
            option.textContent = formatDictionaryTabName(dictionary);
            elements.userDictionarySelect.appendChild(option);
        }
        if (!dictionaries.includes(selectedDictionary)) {
            selectedDictionary = dictionaries.includes('DEMO') ? 'DEMO' : (dictionaries[0] || 'DEMO');
        }
        elements.userDictionarySelect.value = selectedDictionary;
        const words = cachedUserWords.map(createWordInfoFromTuple).filter(word => word.dictionary === selectedDictionary);
        renderUserWordButtons(elements.userWordsDisplay, words);
    };

    const updateSearchFilter = (filter: string): void => {
        searchFilter = filter.trim();

        renderBuiltInWords();
        const words = cachedUserWords.map(createWordInfoFromTuple).filter(word => word.dictionary === selectedDictionary);
        renderUserWordButtons(elements.userWordsDisplay, words);
    };

    elements.userDictionarySelect.addEventListener('change', () => {
        selectedDictionary = elements.userDictionarySelect.value;
        const words = cachedUserWords.map(createWordInfoFromTuple).filter(word => word.dictionary === selectedDictionary);
        renderUserWordButtons(elements.userWordsDisplay, words);
    });

    return {
        renderBuiltInWords,
        updateUserWords,
        updateSearchFilter
    };
};
