

export interface MobileElements {
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly dictionaryArea: HTMLElement;
}

export type ViewMode = 'input' | 'output' | 'stack' | 'dictionary';

export interface MobileHandler {
    readonly isMobile: () => boolean;
    readonly extractCurrentMode: () => ViewMode;
    readonly updateView: (mode: ViewMode) => void;
}

export interface MobileHandlerOptions {
    readonly onModeChange?: (mode: ViewMode) => void;
}

const MOBILE_BREAKPOINT = 768;
const SWIPE_THRESHOLD = 50;

// SPEC §12.3 (Observation surfaces) / Portability Profiles "Presentation Profile".
// On a single-surface device the four observation surfaces are cycled in this
// fixed order; `VIEW_ORDER` and `resolveNextViewMode` are the pure transition
// core of the mobile presentation profile (a model of the Presentation Profile
// LTS). They are exported so the conformance suite
// (layout/presentation-profile.test.ts) can exercise the shipped logic directly.
// The 768px breakpoint and 50px swipe threshold are device tuning, not
// semantics (SPEC §5.3 standing), and are intentionally kept out of that core.
export const VIEW_ORDER: ViewMode[] = ['input', 'output', 'stack', 'dictionary'];

const checkIsMobile = (): boolean => window.innerWidth <= MOBILE_BREAKPOINT;

const detectSwipeDirection = (
    startX: number,
    startY: number,
    endX: number,
    endY: number
): 'left' | 'right' | null => {
    const deltaX = endX - startX;
    const deltaY = endY - startY;

    if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > SWIPE_THRESHOLD) {
        return deltaX > 0 ? 'right' : 'left';
    }

    return null;
};

export const resolveNextViewMode = (currentMode: ViewMode, direction: 'left' | 'right'): ViewMode => {
    const currentIndex = VIEW_ORDER.indexOf(currentMode);
    const nextIndex = direction === 'left'
        ? (currentIndex + 1) % VIEW_ORDER.length
        : (currentIndex - 1 + VIEW_ORDER.length) % VIEW_ORDER.length;
    return VIEW_ORDER[nextIndex]!;
};

const lookupVisibilityForMode = (mode: ViewMode): Record<keyof MobileElements, boolean> => {
    const visibilityByMode: Record<ViewMode, Record<keyof MobileElements, boolean>> = {
        input: { inputArea: false, outputArea: true, stackArea: true, dictionaryArea: true },
        output: { inputArea: true, outputArea: false, stackArea: true, dictionaryArea: true },
        stack: { inputArea: true, outputArea: true, stackArea: false, dictionaryArea: true },
        dictionary: { inputArea: true, outputArea: true, stackArea: true, dictionaryArea: false },
    };
    return visibilityByMode[mode];
};

const applyVisibility = (
    elements: MobileElements,
    visibility: Record<keyof MobileElements, boolean>
): void => {
    (Object.keys(visibility) as Array<keyof MobileElements>).forEach(key => {
        elements[key].hidden = visibility[key];
    });
};

export const createMobileHandler = (
    elements: MobileElements,
    options: MobileHandlerOptions = {}
): MobileHandler => {
    let currentMode: ViewMode = 'input';
    let touchStartX = 0;
    let touchStartY = 0;

    const updateView = (mode: ViewMode): void => {
        currentMode = mode;
        const visibility = lookupVisibilityForMode(mode);
        applyVisibility(elements, visibility);
    };

    const resolveSwipeGesture = (endX: number, endY: number): void => {
        const direction = detectSwipeDirection(touchStartX, touchStartY, endX, endY);

        if (direction === null) return;
        const newMode = resolveNextViewMode(currentMode, direction);
        updateView(newMode);
        options.onModeChange?.(newMode);
    };

    const setupSwipeGestures = (): void => {
        const container = document.body;

        container.addEventListener('touchstart', (e: TouchEvent) => {
            const touch = e.changedTouches[0];
            if (touch) {
                touchStartX = touch.screenX;
                touchStartY = touch.screenY;
            }
        }, { passive: true });

        container.addEventListener('touchend', (e: TouchEvent) => {
            if (!checkIsMobile()) return;
            const touch = e.changedTouches[0];
            if (touch) {
                resolveSwipeGesture(touch.screenX, touch.screenY);
            }
        }, { passive: true });
    };

    setupSwipeGestures();

    return {
        isMobile: () => checkIsMobile(),
        extractCurrentMode: () => currentMode,
        updateView
    };
};
