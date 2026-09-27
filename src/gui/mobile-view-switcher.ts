import { isMobileViewport } from '../platform/viewport';
import { detectSwipeDirection, type GesturePoint } from './touch-gestures';

export interface MobileElements {
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly dictionaryArea: HTMLElement;
}

export type ViewMode = 'input' | 'output' | 'stack' | 'dictionary';

export interface MobileHandler {
    readonly isMobile: () => boolean;
    readonly updateView: (mode: ViewMode) => void;
}

export interface MobileHandlerOptions {
    /** The surface currently showing (LayoutState.currentMode); a swipe moves from it. */
    readonly currentMode: () => ViewMode;
    readonly onModeChange?: (mode: ViewMode) => void;
}

// Device tuning, not semantics (LANG.AUTHORITY.FREEDOM standing), kept out of
// the transition core below.
const SWIPE_THRESHOLD = 50;

// LANG.OBSERVATION.PROJECTIONS (Observation surfaces) / Portability Profiles "Presentation Profile".
// On a single-surface device the four observation surfaces are cycled in this
// fixed order; `VIEW_ORDER` and `resolveNextViewMode` are the pure transition
// core of the mobile presentation profile (a model of the Presentation Profile
// LTS). They are exported so the conformance suite
// (layout/presentation-profile.test.ts) can exercise the shipped logic directly.
export const VIEW_ORDER: ViewMode[] = ['input', 'output', 'stack', 'dictionary'];

export const resolveNextViewMode = (currentMode: ViewMode, direction: 'left' | 'right'): ViewMode => {
    const currentIndex = VIEW_ORDER.indexOf(currentMode);
    const nextIndex = direction === 'left'
        ? (currentIndex + 1) % VIEW_ORDER.length
        : (currentIndex - 1 + VIEW_ORDER.length) % VIEW_ORDER.length;
    return VIEW_ORDER[nextIndex]!;
};

const AREA_FOR_MODE: Record<ViewMode, keyof MobileElements> = {
    input: 'inputArea',
    output: 'outputArea',
    stack: 'stackArea',
    dictionary: 'dictionaryArea'
};

export const createMobileHandler = (
    elements: MobileElements,
    options: MobileHandlerOptions
): MobileHandler => {
    // `null` means the gesture in progress is not a candidate swipe: it grew a
    // second finger (a pinch ends two touches, each with a delta of its own).
    // Where it started is not a disqualifier — see touch-gestures.ts.
    let swipeOrigin: GesturePoint | null = null;

    const updateView = (mode: ViewMode): void => {
        const visible = AREA_FOR_MODE[mode];
        for (const area of Object.values(AREA_FOR_MODE)) {
            elements[area].hidden = area !== visible;
        }
    };

    const resolveSwipeGesture = (origin: GesturePoint, end: GesturePoint): void => {
        const direction = detectSwipeDirection(origin, end, { thresholdPx: SWIPE_THRESHOLD });
        if (direction === null) return;
        options.onModeChange?.(resolveNextViewMode(options.currentMode(), direction));
    };

    document.body.addEventListener('touchstart', (e: TouchEvent) => {
        const touch = e.changedTouches[0];
        swipeOrigin = e.touches.length > 1 || !touch
            ? null
            : { x: touch.screenX, y: touch.screenY };
    }, { passive: true });

    document.body.addEventListener('touchend', (e: TouchEvent) => {
        const origin = swipeOrigin;
        swipeOrigin = null;
        if (!isMobileViewport()) return;
        // Fingers still down: this is one release out of a multi-touch
        // gesture, and its travel is that gesture's, not a swipe's.
        if (origin === null || e.touches.length > 0) return;
        const touch = e.changedTouches[0];
        if (touch) resolveSwipeGesture(origin, { x: touch.screenX, y: touch.screenY });
    }, { passive: true });

    return {
        isMobile: isMobileViewport,
        updateView
    };
};
