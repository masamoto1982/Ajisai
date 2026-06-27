// Coverage for cross-origin-isolation capability detection (implicit-
// parallelism roadmap Phase 5).
//
// DUT: src/platform/cross-origin-isolation.ts — detectParallelCapability /
// describeParallelCapability.
//
// `threadsAvailable` is the decision the worker pool and the future
// wasm-bindgen-rayon initializer branch on:
//   threadsAvailable = crossOriginIsolated && (SharedArrayBuffer defined)
// Both conditions are exercised independently (MC/DC), plus the
// hardwareConcurrency clamping and the recommendedThreads / maxThreads policy.

import { describe, expect, it } from 'vitest';
import {
    detectParallelCapability,
    describeParallelCapability,
    type IsolationScope,
} from './cross-origin-isolation';

// A stand-in for `SharedArrayBuffer` so `typeof scope.SharedArrayBuffer` is
// 'function' without depending on the host actually exposing it.
const SAB = function SharedArrayBufferStub() {} as unknown;

function scope(
    isolated: boolean,
    sab: boolean,
    cores: number | undefined = 8,
): IsolationScope {
    return {
        crossOriginIsolated: isolated,
        ...(sab ? { SharedArrayBuffer: SAB } : {}),
        navigator: cores === undefined ? {} : { hardwareConcurrency: cores },
    };
}

describe('detectParallelCapability', () => {
    it('reports threading available only when isolated AND SharedArrayBuffer present', () => {
        const cap = detectParallelCapability(scope(true, true, 8));
        expect(cap.threadsAvailable).toBe(true);
        expect(cap.recommendedThreads).toBe(8);
    });

    it('isolated but no SharedArrayBuffer → no threading (single condition flips)', () => {
        const cap = detectParallelCapability(scope(true, false, 8));
        expect(cap.crossOriginIsolated).toBe(true);
        expect(cap.sharedArrayBuffer).toBe(false);
        expect(cap.threadsAvailable).toBe(false);
        expect(cap.recommendedThreads).toBe(1);
    });

    it('SharedArrayBuffer present but not isolated → no threading (other condition flips)', () => {
        const cap = detectParallelCapability(scope(false, true, 8));
        expect(cap.crossOriginIsolated).toBe(false);
        expect(cap.sharedArrayBuffer).toBe(true);
        expect(cap.threadsAvailable).toBe(false);
        expect(cap.recommendedThreads).toBe(1);
    });

    it('neither → no threading', () => {
        const cap = detectParallelCapability(scope(false, false, 8));
        expect(cap.threadsAvailable).toBe(false);
        expect(cap.recommendedThreads).toBe(1);
    });

    it('clamps a missing/invalid hardwareConcurrency to 1', () => {
        // Absent hardwareConcurrency (navigator present but no field).
        expect(
            detectParallelCapability({ crossOriginIsolated: true, SharedArrayBuffer: SAB, navigator: {} }).hardwareConcurrency,
        ).toBe(1);
        // Zero / non-positive report clamps up to 1.
        expect(
            detectParallelCapability({ crossOriginIsolated: true, SharedArrayBuffer: SAB, navigator: { hardwareConcurrency: 0 } }).hardwareConcurrency,
        ).toBe(1);
    });

    it('caps recommendedThreads at maxThreads when threading is available', () => {
        const cap = detectParallelCapability(scope(true, true, 16), { maxThreads: 4 });
        expect(cap.hardwareConcurrency).toBe(16);
        expect(cap.recommendedThreads).toBe(4);
    });

    it('maxThreads does not raise the single-thread fallback', () => {
        const cap = detectParallelCapability(scope(false, true, 16), { maxThreads: 4 });
        expect(cap.recommendedThreads).toBe(1);
    });
});

describe('describeParallelCapability', () => {
    it('names the available case with the thread count', () => {
        const text = describeParallelCapability(detectParallelCapability(scope(true, true, 4)));
        expect(text).toContain('threading available');
        expect(text).toContain('4');
    });

    it('attributes the missing-isolation case to COOP/COEP', () => {
        const text = describeParallelCapability(detectParallelCapability(scope(false, true, 4)));
        expect(text).toContain('COOP/COEP');
    });

    it('attributes the missing-SharedArrayBuffer case', () => {
        const text = describeParallelCapability(detectParallelCapability(scope(true, false, 4)));
        expect(text).toContain('SharedArrayBuffer');
    });
});
