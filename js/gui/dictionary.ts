// js/gui/dictionary.ts - 辞書管理（関数型スタイル）

// ============================================================
// 型定義
// ============================================================

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
    readonly showInfo?: (text: string, append: boolean, en?: string) => void;
}

export interface Dictionary {
    readonly renderBuiltinWords: () => void;
    readonly updateCustomWords: (customWordsInfo: Array<[string, string | null, boolean]>) => void;
}

// ============================================================
// 定数: シンボルマップ
// ============================================================

const SYMBOL_MAP: Readonly<Record<string, string>> = Object.freeze({
    'VSTART': '[', 'VEND': ']', 'BSTART': '{', 'BEND': '}',
    'NIL': 'nil', 'ADD': '+', 'SUB': '-', 'MUL': '*', 'DIV': '/',
    'LT': '<', 'LE': '<=', 'GT': '>', 'GE': '>=', 'EQ': '=',
    'AND': 'and', 'OR': 'or', 'NOT': 'not',
});

// ============================================================
// 純粋関数: ワード名のデコード
// ============================================================

/**
 * エンコードされたワード名を人間が読みやすい形式にデコード
 */
const decodeWordName = (name: string): string | null => {
    // W_XXXX 形式は無視
    if (name.match(/^W_[0-9A-F]+$/)) return null;
    // アンダースコアを含まない場合はデコード不要
    if (!name.includes('_')) return null;

    const decoded = name.split('_').map(part => {
        if (part.startsWith('STR_')) {
            return `"${part.substring(4).replace(/_/g, ' ')}"`;
        }
        return SYMBOL_MAP[part] ?? part.toLowerCase();
    }).join(' ');

    return `≈ ${decoded}`;
};

/**
 * ワード情報の配列を WordInfo 型に変換
 */
const toWordInfo = (wordData: [string, string | null, boolean]): WordInfo => ({
    name: wordData[0],
    description: wordData[1] || decodeWordName(wordData[0]) || wordData[0],
    protected: wordData[2] || false
});

/**
 * 組み込みワードをカテゴリ別にグループ化
 */
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

// ============================================================
// 副作用関数: DOM操作
// ============================================================

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

// ============================================================
// ファクトリ関数: Dictionary作成
// ============================================================

export const createDictionary = (
    elements: DictionaryElements,
    callbacks: DictionaryCallbacks
): Dictionary => {
    const { onWordClick, onUpdateDisplays, onSaveState, showInfo } = callbacks;

    // ワード削除の確認と実行
    const confirmAndDeleteWord = async (wordName: string): Promise<void> => {
        if (!confirm(`Delete word '${wordName}'?`)) return;

        try {
            const result = await window.ajisaiInterpreter.execute(`'${wordName}' DEL`);
            if (result.status === 'ERROR') {
                alert(`Failed to delete word: ${result.message}`);
            } else {
                onUpdateDisplays?.();
                await onSaveState?.();
                showInfo?.(`Word '${wordName}' deleted.`, true);
            }
        } catch (error) {
            alert(`Error deleting word: ${error}`);
        }
    };

    // 組み込みワードのグループ別レンダリング
    const renderBuiltinWordsWithGroups = (
        container: HTMLElement,
        builtinWords: unknown[][]
    ): void => {
        clearElement(container);

        const groups = groupByCategory(builtinWords);

        Object.keys(groups).sort().forEach(category => {
            const groupContainer = document.createElement('span');
            groupContainer.className = 'builtin-category-group';

            const categoryWords = groups[category];
            if (categoryWords) {
                categoryWords.forEach(wordData => {
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
            }

            container.appendChild(groupContainer);
        });
    };

    // カスタムワードボタンのレンダリング
    const renderCustomWordButtons = (
        container: HTMLElement,
        words: WordInfo[]
    ): void => {
        clearElement(container);

        words.forEach(wordInfo => {
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
    };

    // 組み込みワードのレンダリング
    const renderBuiltinWords = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            const builtinWords = window.ajisaiInterpreter.get_builtin_words_info();
            renderBuiltinWordsWithGroups(elements.builtinWordsDisplay, builtinWords);
        } catch (error) {
            console.error('Failed to render builtin words:', error);
        }
    };

    // カスタムワードの更新
    const updateCustomWords = (
        customWordsInfo: Array<[string, string | null, boolean]>
    ): void => {
        const words = (customWordsInfo || []).map(toWordInfo);
        renderCustomWordButtons(elements.customWordsDisplay, words);
    };

    return {
        renderBuiltinWords,
        updateCustomWords
    };
};

// 純粋関数をエクスポート（テスト用）
export const dictionaryUtils = {
    decodeWordName,
    toWordInfo,
    groupByCategory,
    SYMBOL_MAP
};
