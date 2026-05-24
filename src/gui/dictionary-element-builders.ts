export const DEFAULT_WORD_INFO_MESSAGE =
    'An example of how to use the specified word will be displayed here.';

export const renderWordInfo = (element: HTMLElement, text: string, isPlaceholder = false): void => {
    element.textContent = text;
    element.classList.toggle('is-placeholder', isPlaceholder);
};

export const resetWordInfoDisplay = (element: HTMLElement): void => {
    renderWordInfo(element, DEFAULT_WORD_INFO_MESSAGE, true);
};

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

const LONG_PRESS_MS = 500;
const LONG_PRESS_MOVE_TOLERANCE_PX = 10;

// Attaches long-press detection that works for both touch and mouse. When the
// press is held past LONG_PRESS_MS without moving, `onLongPress` fires and the
// ensuing `click` is suppressed so a long-press never doubles as a tap.
const attachLongPress = (button: HTMLButtonElement, onLongPress: () => void): void => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    let fired = false;
    let startX = 0;
    let startY = 0;

    const cancelTimer = (): void => {
        if (timer) {
            clearTimeout(timer);
            timer = null;
        }
    };

    button.addEventListener('pointerdown', (e: PointerEvent) => {
        // Only a primary (left) press arms the long-press; right-click stays
        // reserved for the context menu.
        if (e.button !== 0) return;
        fired = false;
        startX = e.clientX;
        startY = e.clientY;
        cancelTimer();
        timer = setTimeout(() => {
            fired = true;
            timer = null;
            onLongPress();
        }, LONG_PRESS_MS);
    });

    button.addEventListener('pointermove', (e: PointerEvent) => {
        if (!timer) return;
        if (Math.abs(e.clientX - startX) > LONG_PRESS_MOVE_TOLERANCE_PX
            || Math.abs(e.clientY - startY) > LONG_PRESS_MOVE_TOLERANCE_PX) {
            cancelTimer();
        }
    });

    button.addEventListener('pointerup', cancelTimer);
    button.addEventListener('pointerleave', cancelTimer);
    button.addEventListener('pointercancel', cancelTimer);

    // Runs before the user-supplied click handler (registered later), so a
    // long-press swallows the click entirely.
    button.addEventListener('click', (e: MouseEvent) => {
        if (fired) {
            fired = false;
            e.preventDefault();
            e.stopImmediatePropagation();
        }
    });
};

export const createWordButtonElement = (
    text: string,
    title: string,
    className: string,
    onClick: () => void,
    onHover?: () => void,
    onLeave?: () => void,
    onContextMenu?: (event: MouseEvent) => void,
    onLongPress?: () => void
): HTMLButtonElement => {
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = text;
    button.className = className;
    button.title = title;

    // Long-press must be wired before the click handler so its capture of a
    // fired long-press (via stopImmediatePropagation) runs first.
    if (onLongPress) attachLongPress(button, onLongPress);

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
