

import {
    checkIsSwipeExempt,
    detectSwipeDirection,
    type GesturePoint
} from './touch-gestures';

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

// LANG.OBSERVATION.PROJECTIONS (Observation surfaces) / Portability Profiles "Presentation Profile".
// On a single-surface device the four observation surfaces are cycled in this
// fixed order; `VIEW_ORDER` and `resolveNextViewMode` are the pure transition
// core of the mobile presentation profile (a model of the Presentation Profile
// LTS). They are exported so the conformance suite
// (layout/presentation-profile.test.ts) can exercise the shipped logic directly.
// The 768px breakpoint and 50px swipe threshold are device tuning, not
// semantics (LANG.AUTHORITY.FREEDOM standing), and are intentionally kept out of that core.
export const VIEW_ORDER: ViewMode[] = ['input', 'output', 'stack', 'dictionary'];

const checkIsMobile = (): boolean => window.innerWidth <= MOBILE_BREAKPOINT;

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
    // `null` means the gesture in progress is not a candidate swipe: it started
    // on an element that owns its own horizontal drag, or it grew a second
    // finger (a pinch ends two touches, each with a delta of its own).
    let swipeOrigin: GesturePoint | null = null;

    const updateView = (mode: ViewMode): void => {
        currentMode = mode;
        const visibility = lookupVisibilityForMode(mode);
        applyVisibility(elements, visibility);
    };

    const resolveSwipeGesture = (origin: GesturePoint, end: GesturePoint): void => {
        const direction = detectSwipeDirection(origin, end, { thresholdPx: SWIPE_THRESHOLD });

        if (direction === null) return;
        const newMode = resolveNextViewMode(currentMode, direction);
        updateView(newMode);
        options.onModeChange?.(newMode);
    };

    const setupSwipeGestures = (): void => {
        const container = document.body;

        container.addEventListener('touchstart', (e: TouchEvent) => {
            if (e.touches.length > 1) {
                swipeOrigin = null;
                return;
            }
            const touch = e.changedTouches[0];
            if (!touch || checkIsSwipeExempt(e.target)) {
                swipeOrigin = null;
                return;
            }
            swipeOrigin = { x: touch.screenX, y: touch.screenY };
        }, { passive: true });

        container.addEventListener('touchend', (e: TouchEvent) => {
            const origin = swipeOrigin;
            swipeOrigin = null;
            if (!checkIsMobile()) return;
            // Fingers still down: this is one release out of a multi-touch
            // gesture, and its travel is that gesture's, not a swipe's.
            if (origin === null || e.touches.length > 0) return;
            const touch = e.changedTouches[0];
            if (touch) {
                resolveSwipeGesture(origin, { x: touch.screenX, y: touch.screenY });
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
