// js/gui/dictionary.ts

export interface WordInfo {
    readonly name: string;
    readonly description?: string | null;
    readonly protected?: boolean;
}

export interface DictionaryElements {
    readonly builtinWordsDisplay: HTMLElement;
    readonly customWordsDisplay: HTMLElement;
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

const groupByCategory = (
    builtinWords: unknown[][]
): Record<string, unknown[][]> => {
    const groups: Record<string, unknown[][]> = {};

    builtinWords
        .filter((wordData): wordData is unknown[] =>
            Array.isArray(wordData) && wordData[0] !== ';' && wordData[0] !== '"'
        )
        .forEach(wordData => {
            const category = (wordData[2] as string) || 'Other';
            if (!groups[category]) {
                groups[category] = [];
            }
            groups[category].push(wordData);
        });

    return groups;
};

const clearElement = (element: HTMLElement): void => {
    element.innerHTML = '';
};

const createButton = (
    text: string,
    title: string,
    className: string,
    onClick: () => void
): HTMLButtonElement => {
    const button = document.createElement('button');
    button.textContent = text;
    button.className = className;
    button.title = title;
    button.addEventListener('click', onClick);
    return button;
};

const createButtonWithContextMenu = (
    text: string,
    title: string,
    className: string,
    onClick: () => void,
    onContextMenu: () => void
): HTMLButtonElement => {
    const button = createButton(text, title, className, onClick);
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

    const renderBuiltinWordsWithGroups = (
        container: HTMLElement,
        builtinWords: unknown[][]
    ): void => {
        clearElement(container);

        const groups = groupByCategory(builtinWords);
        let totalMatches = 0;

        Object.keys(groups).sort().forEach(category => {
            const categoryWords = groups[category];
            if (!categoryWords) return;

            // フィルタリング: マッチするワードのみ抽出
            const filteredWords = categoryWords.filter(wordData => {
                const name = wordData[0] as string;
                return matchesFilter(name, searchFilter);
            });

            // カテゴリ内にマッチするワードがなければスキップ
            if (filteredWords.length === 0) return;

            totalMatches += filteredWords.length;

            const groupContainer = document.createElement('span');
            groupContainer.className = 'builtin-category-group';

            filteredWords.forEach(wordData => {
                const name = wordData[0] as string;
                const description = (wordData[1] as string) || name;

                const button = createButton(
                    name,
                    description,
                    'word-button builtin',
                    () => onWordClick(name)
                );

                groupContainer.appendChild(button);
            });

            container.appendChild(groupContainer);
        });

        // フィルターが設定されているが結果がない場合
        if (searchFilter && totalMatches === 0) {
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

        filteredWords.forEach(wordInfo => {
            const className = wordInfo.protected
                ? 'word-button dependency'
                : 'word-button non-dependency';

            const button = createButtonWithContextMenu(
                wordInfo.name,
                wordInfo.description || '',
                className,
                () => onWordClick(wordInfo.name),
                () => confirmAndDeleteWord(wordInfo.name)
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
            renderBuiltinWordsWithGroups(elements.builtinWordsDisplay, builtinWords);
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
    groupByCategory,
    SYMBOL_MAP
};
