// js/gui/mobile.ts

export interface MobileElements {
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly dictionaryArea: HTMLElement;
}

export type ViewMode = 'input' | 'execution';

export interface MobileHandler {
    readonly isMobile: () => boolean;
    readonly getCurrentMode: () => ViewMode;
    readonly updateView: (mode: ViewMode) => void;
}

const MOBILE_BREAKPOINT = 768;
const SWIPE_THRESHOLD = 50;

const checkIsMobile = (): boolean => window.innerWidth <= MOBILE_BREAKPOINT;

const detectSwipeDirection = (
    startX: number,
    startY: number,
    endX: number,
    endY: number
): 'left' | 'right' | 'up' | 'down' | null => {
    const deltaX = endX - startX;
    const deltaY = endY - startY;

    if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > SWIPE_THRESHOLD) {
        return deltaX > 0 ? 'right' : 'left';
    }

    if (Math.abs(deltaY) > Math.abs(deltaX) && Math.abs(deltaY) > SWIPE_THRESHOLD) {
        return deltaY > 0 ? 'down' : 'up';
    }

    return null;
};

const toggleMode = (currentMode: ViewMode): ViewMode =>
    currentMode === 'input' ? 'execution' : 'input';

const getInputModeStyles = (): Record<keyof MobileElements, string> => ({
    inputArea: 'block',
    outputArea: 'none',
    stackArea: 'none',
    dictionaryArea: 'block'
});

const getExecutionModeStyles = (): Record<keyof MobileElements, string> => ({
    inputArea: 'none',
    outputArea: 'block',
    stackArea: 'block',
    dictionaryArea: 'none'
});

const getStylesForMode = (mode: ViewMode): Record<keyof MobileElements, string> =>
    mode === 'input' ? getInputModeStyles() : getExecutionModeStyles();

const setElementDisplay = (element: HTMLElement, display: string): void => {
    element.style.display = display;
};

const resetElementDisplay = (element: HTMLElement): void => {
    element.style.display = '';
};

const applyMobileStyles = (
    elements: MobileElements,
    styles: Record<keyof MobileElements, string>
): void => {
    (Object.keys(styles) as Array<keyof MobileElements>).forEach(key => {
        setElementDisplay(elements[key], styles[key]);
    });
};

const resetAllStyles = (elements: MobileElements): void => {
    Object.values(elements).forEach(el => {
        if (el?.style) {
            resetElementDisplay(el);
        }
    });
};

export const createMobileHandler = (elements: MobileElements): MobileHandler => {
    let currentMode: ViewMode = 'input';
    let touchStartX = 0;
    let touchStartY = 0;

    const handleSwipeGesture = (endX: number, endY: number): void => {
        const direction = detectSwipeDirection(touchStartX, touchStartY, endX, endY);

        if (direction === 'left' || direction === 'right') {
            const newMode = toggleMode(currentMode);
            updateView(newMode);
        }
    };

    const setupSwipeGestures = (): void => {
        if (!checkIsMobile()) return;

        const container = document.body;

        container.addEventListener('touchstart', (e: TouchEvent) => {
            const touch = e.changedTouches[0];
            if (touch) {
                touchStartX = touch.screenX;
                touchStartY = touch.screenY;
            }
        }, { passive: true });

        container.addEventListener('touchend', (e: TouchEvent) => {
            const touch = e.changedTouches[0];
            if (touch) {
                handleSwipeGesture(touch.screenX, touch.screenY);
            }
        }, { passive: true });
    };

    setupSwipeGestures();

    const isMobile = (): boolean => checkIsMobile();
    const getCurrentMode = (): ViewMode => currentMode;

    const updateView = (mode: ViewMode): void => {
        if (!checkIsMobile()) {
            resetAllStyles(elements);
            return;
        }

        currentMode = mode;
        const styles = getStylesForMode(mode);
        applyMobileStyles(elements, styles);
    };

    return {
        isMobile,
        getCurrentMode,
        updateView
    };
};

export const mobileUtils = {
    checkIsMobile,
    detectSwipeDirection,
    toggleMode,
    getStylesForMode,
    MOBILE_BREAKPOINT,
    SWIPE_THRESHOLD
};
