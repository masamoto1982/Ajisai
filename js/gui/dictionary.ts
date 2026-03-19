// js/gui/dictionary.ts

import {
    createEmptyWordsMessage,
    createNoResultsMessage,
    createWordButton,
    matchesFilter,
    setupBackgroundClickHandlers,
    sortWordName,
} from './dictionary-ui';

export interface WordInfo {
    readonly name: string;
    readonly description?: string | null;
    readonly protected?: boolean;
}

export interface VocabularyElements {
    readonly builtInWordsDisplay: HTMLElement;
    readonly customWordsDisplay: HTMLElement;
    readonly builtInWordInfo: HTMLElement;
    readonly customWordInfo: HTMLElement;
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
    readonly updateCustomWords: (customWordsInfo: Array<[string, string | null, boolean]>) => void;
    readonly setSearchFilter: (filter: string) => void;
}

const SYMBOL_MAP: Readonly<Record<string, string>> = Object.freeze({
    'VSTART': '[', 'VEND': ']', 'BSTART': '{', 'BEND': '}',
    'NIL': 'nil', 'ADD': '+', 'SUB': '-', 'MUL': '*', 'DIV': '/',
    'LT': '<', 'LE': '<=', 'GT': '>', 'GE': '>=', 'EQ': '=',
    'AND': 'and', 'OR': 'or', 'NOT': 'not',
});

const decodeWordName = (name: string): string | null => {
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

const toWordInfo = (wordData: [string, string | null, boolean]): WordInfo => ({
    name: wordData[0],
    description: wordData[1] || decodeWordName(wordData[0]) || wordData[0],
    protected: wordData[2] || false
});


const clearElement = (element: HTMLElement): void => {
    element.innerHTML = '';
};

const DEFAULT_WORD_INFO_MESSAGE = 'Hover over a word button to view its usage.';

const setWordInfo = (element: HTMLElement, text: string, isPlaceholder = false): void => {
    element.textContent = text;
    element.classList.toggle('is-placeholder', isPlaceholder);
};

const resetWordInfo = (element: HTMLElement): void => {
    setWordInfo(element, DEFAULT_WORD_INFO_MESSAGE, true);
};

const DEPENDENCY_DELETE_ERROR = 'Cannot delete';

const createDeleteContextMenu = (
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
    const deleteContextMenu = createDeleteContextMenu(() => {
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

    const showDeleteContextMenu = (event: MouseEvent, wordName: string): void => {
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

    [elements.builtInWordsDisplay, elements.customWordsDisplay].forEach(container => {
        setupBackgroundClickHandlers(container, onBackgroundClick, onBackgroundDoubleClick);
    });

    [elements.builtInWordInfo, elements.customWordInfo].forEach(resetWordInfo);

    // 検索フィルターとカスタムワードのキャッシュ
    let searchFilter = '';
    let cachedCustomWords: Array<[string, string | null, boolean]> = [];

    const deleteWord = async (wordName: string, forceDelete: boolean): Promise<boolean> => {
        const deleteCode = forceDelete
            ? `! '${wordName}' DEL`
            : `'${wordName}' DEL`;

        try {
            const result = await window.ajisaiInterpreter.execute(deleteCode);
            if (result.status === 'ERROR') {
                if (!forceDelete && result.message?.includes(DEPENDENCY_DELETE_ERROR)) {
                    const confirmed = confirm(
                        `Word '${wordName}' is referenced by other custom words. Force delete with ! ?`
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

        // Filter out ';' and '"'
        const filtered = coreWords.filter(
            (wd): wd is unknown[] =>
                Array.isArray(wd) && wd[0] !== ';' && wd[0] !== '"'
        );

        // Sort: symbols first, then alphabetic
        const sorted = [...filtered].sort((a, b) =>
            sortWordName(a[0] as string, b[0] as string)
        );

        // Apply search filter
        const matched = sorted.filter(wd =>
            matchesFilter(wd[0] as string, searchFilter)
        );

        // Create buttons
        matched.forEach(wordData => {
            const name = wordData[0] as string;
            const description = (wordData[1] as string) || name;
            const syntaxExample = (wordData[2] as string) || '';
            const signatureType = (wordData[3] as string) || 'none';
            const sigClass = signatureType !== 'none' ? ` signature-${signatureType}` : '';

            const button = createWordButton(
                name,
                description,
                `word-button core${sigClass}`,
                () => onWordClick(name),
                () => { setWordInfo(elements.builtInWordInfo, syntaxExample || DEFAULT_WORD_INFO_MESSAGE, !syntaxExample); },
                () => { resetWordInfo(elements.builtInWordInfo); }
            );

            container.appendChild(button);
        });

        if (searchFilter && matched.length === 0) {
            container.appendChild(createNoResultsMessage());
        }
    };

    const renderCustomWordButtons = (
        container: HTMLElement,
        words: WordInfo[]
    ): void => {
        clearElement(container);

        // フィルタリング: マッチするワードのみ抽出
        const filteredWords = words.filter(wordInfo =>
            matchesFilter(wordInfo.name, searchFilter)
        );

        // Sort: symbols first, then alphabetic
        const sortedFiltered = [...filteredWords].sort((a, b) =>
            sortWordName(a.name, b.name)
        );

        sortedFiltered.forEach(wordInfo => {
            const className = wordInfo.protected
                ? 'word-button dependency'
                : 'word-button non-dependency';

            const button = createWordButton(
                wordInfo.name,
                wordInfo.description || '',
                className,
                () => onWordClick(wordInfo.name),
                () => {
                    const definition = window.ajisaiInterpreter?.get_word_definition(wordInfo.name);
                    setWordInfo(elements.customWordInfo, definition || DEFAULT_WORD_INFO_MESSAGE, !definition);
                },
                () => { resetWordInfo(elements.customWordInfo); },
                (event) => showDeleteContextMenu(event, wordInfo.name)
            );

            container.appendChild(button);
        });

        // フィルターが設定されているが結果がない場合
        // (ただし、元のワードリストが空の場合は表示しない)
        if (searchFilter && words.length > 0 && filteredWords.length === 0) {
            container.classList.add('is-empty');
            container.appendChild(createNoResultsMessage());
            return;
        }

        if (!searchFilter && words.length === 0) {
            container.classList.add('is-empty');
            container.appendChild(createEmptyWordsMessage('No custom words defined yet.'));
            return;
        }

        container.classList.toggle('is-empty', sortedFiltered.length === 0);
    };

    const renderBuiltInWords = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            const coreWords = window.ajisaiInterpreter.get_core_words_info();
            renderBuiltInWordsSorted(elements.builtInWordsDisplay, coreWords);
        } catch (error) {
            console.error('Failed to render core words:', error);
        }
    };

    const updateCustomWords = (
        customWordsInfo: Array<[string, string | null, boolean]>
    ): void => {
        // キャッシュを更新
        cachedCustomWords = customWordsInfo || [];
        const words = cachedCustomWords.map(toWordInfo);
        renderCustomWordButtons(elements.customWordsDisplay, words);
    };

    const setSearchFilter = (filter: string): void => {
        searchFilter = filter.trim();
        // 両方のワードリストを再レンダリング
        renderBuiltInWords();
        const words = cachedCustomWords.map(toWordInfo);
        renderCustomWordButtons(elements.customWordsDisplay, words);
    };

    return {
        renderBuiltInWords,
        updateCustomWords,
        setSearchFilter
    };
};

export const dictionaryUtils = {
    decodeWordName,
    toWordInfo,
    SYMBOL_MAP
};
