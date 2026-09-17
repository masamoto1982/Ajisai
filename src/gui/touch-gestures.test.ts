import { describe, it, expect } from 'vitest';
import {
    checkIsStationary,
    createMultiTapRecognizer,
    detectSwipeDirection,
    measureDistance
} from './touch-gestures';

const TAP = { intervalMs: 500, movementTolerancePx: 24 };
const SWIPE = { thresholdPx: 50 };

describe('checkIsStationary', () => {
    it('accepts a touch that came up where it went down', () => {
        expect(checkIsStationary({ x: 100, y: 100 }, { x: 100, y: 100 }, 24)).toBe(true);
    });

    it('accepts the wobble of a thumb inside the tolerance', () => {
        expect(checkIsStationary({ x: 100, y: 100 }, { x: 110, y: 108 }, 24)).toBe(true);
    });

    it('rejects a drag past the tolerance — a text selection, not a tap', () => {
        expect(checkIsStationary({ x: 100, y: 100 }, { x: 180, y: 100 }, 24)).toBe(false);
    });

    it('measures diagonally, not per axis', () => {
        // 20px on each axis is inside the tolerance on either axis alone and
        // outside it as a distance (~28px).
        expect(measureDistance({ x: 0, y: 0 }, { x: 20, y: 20 })).toBeCloseTo(28.28, 1);
        expect(checkIsStationary({ x: 0, y: 0 }, { x: 20, y: 20 }, 24)).toBe(false);
    });
});

describe('createMultiTapRecognizer', () => {
    it('counts a run of taps in the same place', () => {
        const recognizer = createMultiTapRecognizer(TAP);
        const here = { x: 50, y: 50 };

        expect(recognizer.registerTap(here, 1000)).toBe(1);
        expect(recognizer.registerTap(here, 1200)).toBe(2);
        expect(recognizer.registerTap(here, 1400)).toBe(3);
    });

    it('starts a new run once the interval lapses', () => {
        const recognizer = createMultiTapRecognizer(TAP);
        const here = { x: 50, y: 50 };

        expect(recognizer.registerTap(here, 1000)).toBe(1);
        expect(recognizer.registerTap(here, 1501)).toBe(1);
    });

    it('treats the interval boundary as still the same run', () => {
        const recognizer = createMultiTapRecognizer(TAP);
        const here = { x: 50, y: 50 };

        recognizer.registerTap(here, 1000);
        expect(recognizer.registerTap(here, 1500)).toBe(2);
    });

    it('starts a new run when a tap lands away from the run', () => {
        const recognizer = createMultiTapRecognizer(TAP);

        expect(recognizer.registerTap({ x: 50, y: 50 }, 1000)).toBe(1);
        expect(recognizer.registerTap({ x: 50, y: 50 }, 1100)).toBe(2);
        expect(recognizer.registerTap({ x: 300, y: 50 }, 1200)).toBe(1);
    });

    it('measures against the first tap, so a run cannot drift across the screen', () => {
        const recognizer = createMultiTapRecognizer(TAP);

        // Each tap is within tolerance of the one before it, and the third is
        // 40px from where the run started: a slow drag with pauses, not a run
        // of taps on one spot.
        expect(recognizer.registerTap({ x: 0, y: 0 }, 1000)).toBe(1);
        expect(recognizer.registerTap({ x: 20, y: 0 }, 1100)).toBe(2);
        expect(recognizer.registerTap({ x: 40, y: 0 }, 1200)).toBe(1);
    });

    it('abandons the run on reset', () => {
        const recognizer = createMultiTapRecognizer(TAP);
        const here = { x: 50, y: 50 };

        recognizer.registerTap(here, 1000);
        recognizer.registerTap(here, 1100);
        recognizer.reset();
        expect(recognizer.registerTap(here, 1200)).toBe(1);
    });

    it('keeps counting past the trigger until the caller resets', () => {
        // The bindings reset on the tap that fires, so this only pins that the
        // recognizer itself does not silently roll over.
        const recognizer = createMultiTapRecognizer(TAP);
        const here = { x: 50, y: 50 };

        recognizer.registerTap(here, 1000);
        recognizer.registerTap(here, 1100);
        recognizer.registerTap(here, 1200);
        expect(recognizer.registerTap(here, 1300)).toBe(4);
    });
});

describe('detectSwipeDirection', () => {
    it('reads a long rightward drag as a right swipe', () => {
        expect(detectSwipeDirection({ x: 10, y: 10 }, { x: 120, y: 20 }, SWIPE)).toBe('right');
    });

    it('reads a long leftward drag as a left swipe', () => {
        expect(detectSwipeDirection({ x: 200, y: 10 }, { x: 40, y: 20 }, SWIPE)).toBe('left');
    });

    it('ignores a drag that did not travel far enough', () => {
        expect(detectSwipeDirection({ x: 10, y: 10 }, { x: 55, y: 10 }, SWIPE)).toBeNull();
    });

    it('ignores the threshold exactly, so the two readings never overlap', () => {
        expect(detectSwipeDirection({ x: 0, y: 0 }, { x: 50, y: 0 }, SWIPE)).toBeNull();
        expect(detectSwipeDirection({ x: 0, y: 0 }, { x: 51, y: 0 }, SWIPE)).toBe('right');
    });

    it('ignores a mostly vertical drag — that is a scroll', () => {
        expect(detectSwipeDirection({ x: 10, y: 10 }, { x: 120, y: 200 }, SWIPE)).toBeNull();
    });

    it('ignores a tap that never moved', () => {
        expect(detectSwipeDirection({ x: 10, y: 10 }, { x: 10, y: 10 }, SWIPE)).toBeNull();
    });
});
