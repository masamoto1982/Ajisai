export const compareWordName = (a: string, b: string): number => {
    const aIsAlpha = /^[A-Za-z]/.test(a);
    const bIsAlpha = /^[A-Za-z]/.test(b);

    if (!aIsAlpha && bIsAlpha) return -1;
    if (aIsAlpha && !bIsAlpha) return 1;

    return a.localeCompare(b);
};

export const checkWordMatchesFilter = (wordName: string, filter: string): boolean => {
    if (!filter) return true;
    return wordName.toLowerCase().includes(filter.toLowerCase());
};

export const createNoResultsElement = (): HTMLElement => {
    const message = document.createElement('div');
    message.className = 'no-results-message';
    message.textContent = 'No matching words found';
    return message;
};

export const createEmptyWordsElement = (text: string): HTMLElement => {
    const message = document.createElement('div');
    message.className = 'empty-words-message';
    message.textContent = text;
    return message;
};

export const registerBackgroundClickListeners = (
    container: HTMLElement,
    onBackgroundClick?: () => void,
    onBackgroundDoubleClick?: () => void
): void => {
    const shouldIgnoreBackgroundInteraction = (): boolean =>
        container.classList.contains('is-empty');

    const isBackgroundClick = (e: MouseEvent): boolean => {
        if (shouldIgnoreBackgroundInteraction()) return false;
        const target = e.target as HTMLElement;
        return !target.closest('.word-button');
    };

    let clickTimer: ReturnType<typeof setTimeout> | null = null;

    if (onBackgroundClick) {
        container.addEventListener('click', (e) => {
            if (!isBackgroundClick(e as MouseEvent)) return;
            if (clickTimer) clearTimeout(clickTimer);
            clickTimer = setTimeout(() => {
                onBackgroundClick();
                clickTimer = null;
            }, 200);
        });
    }

    if (onBackgroundDoubleClick) {
        container.addEventListener('dblclick', (e) => {
            if (!isBackgroundClick(e as MouseEvent)) return;
            if (clickTimer) {
                clearTimeout(clickTimer);
                clickTimer = null;
            }
            onBackgroundDoubleClick();
        });
    }
};

export const createWordButtonElement = (
    text: string,
    title: string,
    className: string,
    onClick: () => void,
    onHover?: () => void,
    onLeave?: () => void,
    onContextMenu?: (event: MouseEvent) => void
): HTMLButtonElement => {
    const button = document.createElement('button');
    button.textContent = text;
    button.className = className;
    button.title = title;
    button.addEventListener('click', onClick);

    if (onHover) button.addEventListener('mouseenter', onHover);
    if (onLeave) button.addEventListener('mouseleave', onLeave);
    if (onContextMenu) {
        button.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            onContextMenu(e);
        });
    }

    return button;
};
