// js/gui/dictionary.ts

export interface WordInfo {
    readonly name: string;
    readonly description?: string | null;
    readonly protected?: boolean;
}

export interface DictionaryElements {
    readonly builtinWordsDisplay: HTMLElement;
    readonly customWordsDisplay: HTMLElement;
    readonly builtinWordInfo: HTMLElement;
    readonly customWordInfo: HTMLElement;
}

export interface DictionaryCallbacks {
    readonly onWordClick: (word: string) => void;
    readonly onUpdateDisplays?: () => void;
    readonly onSaveState?: () => Promise<void>;
    readonly showInfo?: (text: string, append: boolean) => void;
}

export interface Dictionary {
    readonly renderBuiltinWords: () => void;
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

const sortWordName = (a: string, b: string): number => {
    const aIsAlpha = /^[A-Za-z]/.test(a);
    const bIsAlpha = /^[A-Za-z]/.test(b);

    // Symbols first, alphabetic after
    if (!aIsAlpha && bIsAlpha) return -1;
    if (aIsAlpha && !bIsAlpha) return 1;

    // Within same group, sort by localeCompare
    return a.localeCompare(b);
};

const clearElement = (element: HTMLElement): void => {
    element.innerHTML = '';
};

const createButton = (
    text: string,
    title: string,
    className: string,
    onClick: () => void,
    onHover?: () => void,
    onLeave?: () => void
): HTMLButtonElement => {
    const button = document.createElement('button');
    button.textContent = text;
    button.className = className;
    button.title = title;
    button.addEventListener('click', onClick);
    if (onHover) {
        button.addEventListener('mouseenter', onHover);
    }
    if (onLeave) {
        button.addEventListener('mouseleave', onLeave);
    }
    return button;
};

const createButtonWithContextMenu = (
    text: string,
    title: string,
    className: string,
    onClick: () => void,
    onContextMenu: () => void,
    onHover?: () => void,
    onLeave?: () => void
): HTMLButtonElement => {
    const button = createButton(text, title, className, onClick, onHover, onLeave);
    button.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        onContextMenu();
    });
    return button;
};

const matchesFilter = (wordName: string, filter: string): boolean => {
    if (!filter) return true;
    return wordName.toLowerCase().includes(filter.toLowerCase());
};

const createNoResultsMessage = (): HTMLElement => {
    const message = document.createElement('div');
    message.className = 'no-results-message';
    message.textContent = 'No matching words found';
    return message;
};

export const createDictionary = (
    elements: DictionaryElements,
    callbacks: DictionaryCallbacks
): Dictionary => {
    const { onWordClick, onUpdateDisplays, onSaveState, showInfo } = callbacks;

    // 検索フィルターとカスタムワードのキャッシュ
    let searchFilter = '';
    let cachedCustomWords: Array<[string, string | null, boolean]> = [];

    const confirmAndDeleteWord = async (wordName: string): Promise<void> => {
        if (!confirm(`Delete word '${wordName}'?`)) return;

        try {
            const result = await window.ajisaiInterpreter.execute(`'${wordName}' DEL`);
            if (result.status === 'ERROR') {
                alert(`Failed to delete word: ${result.message}`);
            } else {
                onUpdateDisplays?.();
                await onSaveState?.();
                showInfo?.(`Word '${wordName}' deleted`, true);
            }
        } catch (error) {
            alert(`Error deleting word: ${error}`);
        }
    };

    const renderBuiltinWordsSorted = (
        container: HTMLElement,
        builtinWords: unknown[][]
    ): void => {
        clearElement(container);

        // Filter out ';' and '"'
        const filtered = builtinWords.filter(
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

            const button = createButton(
                name,
                description,
                `word-button builtin${sigClass}`,
                () => onWordClick(name),
                () => { elements.builtinWordInfo.textContent = syntaxExample; },
                () => { elements.builtinWordInfo.textContent = ''; }
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

            const button = createButtonWithContextMenu(
                wordInfo.name,
                wordInfo.description || '',
                className,
                () => onWordClick(wordInfo.name),
                () => confirmAndDeleteWord(wordInfo.name),
                () => {
                    // Show word definition on hover
                    const definition = window.ajisaiInterpreter?.get_word_definition(wordInfo.name);
                    elements.customWordInfo.textContent = definition || '';
                },
                () => { elements.customWordInfo.textContent = ''; }
            );

            container.appendChild(button);
        });

        // フィルターが設定されているが結果がない場合
        // (ただし、元のワードリストが空の場合は表示しない)
        if (searchFilter && words.length > 0 && filteredWords.length === 0) {
            container.appendChild(createNoResultsMessage());
        }
    };

    const renderBuiltinWords = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            const builtinWords = window.ajisaiInterpreter.get_builtin_words_info();
            renderBuiltinWordsSorted(elements.builtinWordsDisplay, builtinWords);
        } catch (error) {
            console.error('Failed to render builtin words:', error);
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
        renderBuiltinWords();
        const words = cachedCustomWords.map(toWordInfo);
        renderCustomWordButtons(elements.customWordsDisplay, words);
    };

    return {
        renderBuiltinWords,
        updateCustomWords,
        setSearchFilter
    };
};

export const dictionaryUtils = {
    decodeWordName,
    toWordInfo,
    SYMBOL_MAP
};
