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

const BACKGROUND_CLICK_HINT = 'Click the blank area to insert a space';

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
        // The background is a control too, so the screen says so: the
        // browser's own tooltip carries the hint, the way a word's description
        // rides on its button. It is set as the pointer arrives rather than
        // once: an empty list ignores background clicks, so it must not
        // advertise one.
        container.addEventListener('mouseover', () => {
            container.title = shouldIgnoreBackgroundInteraction()
                ? ''
                : BACKGROUND_CLICK_HINT;
        });
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
    className: string,
    onClick: () => void,
    /**
     * What the word does and an example of using it, shown as the browser's
     * own tooltip: the browser owns the timing, the placement and the
     * dismissal, and the surface reserves no row of its own for a hover
     * display.
     */
    title?: string,
    onContextMenu?: (event: MouseEvent) => void
): HTMLButtonElement => {
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = text;
    button.className = className;
    // Always set, even when empty: an absent `title` would inherit the list
    // background's hint, which describes the gap between buttons, not a word.
    button.title = title ?? '';

    button.addEventListener('click', onClick);

    if (onContextMenu) {
        button.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            onContextMenu(e);
        });
    }

    return button;
};
