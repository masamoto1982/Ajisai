// Presentation Profile conformance suite (SPEC §12.3 "Observation surfaces" +
// Portability Profiles "Presentation Profile").
//
// SPEC formalizes the device-facing programming experience in two layers:
//   * §12.3 — the four observation surfaces are total, pure projections of the
//     runtime state: π_Input, π_Stack, π_Output, π_Dict over the surface set A.
//   * Presentation Profile — how those surfaces are made visible on a device is
//     a labeled transition system M = (C, Σ, →, c0) over visibility
//     configurations c ⊆ A, constrained by six normative invariants.
//
// This suite checks that the *shipped* desktop and single-surface (mobile)
// layouts are two models of that one abstract LTS, by exercising the real
// transition cores (`updateDesktopModes`, `resolveNextViewMode`) rather than a
// re-encoding. Device tuning (breakpoints, swipe thresholds, tap counts, column
// geometry) is implementation freedom (SPEC §5.3 standing) and is not asserted.

import { describe, it, expect } from 'vitest';
import { updateDesktopModes, type LayoutState } from '../gui-layout-state';
import { createGuiLayoutState } from './layout-model';
import { resolveNextViewMode, VIEW_ORDER, type ViewMode } from '../mobile-view-switcher';

// Surface set A (SPEC §12.3). Order is irrelevant here; configurations are sets.
const SURFACES: readonly ViewMode[] = ['input', 'output', 'stack', 'dictionary'];

// A presentation profile as a labeled transition system M = (C, Σ, →, c0).
// `step` is the partial function →; `visible` reads the configuration c ⊆ A;
// `key` gives a configuration/state a canonical id for reachability dedup.
interface PresentationLTS<S> {
    readonly name: string;
    readonly initial: S;
    readonly key: (state: S) => string;
    readonly visible: (state: S) => ReadonlySet<ViewMode>;
    /** Alphabet Σ as concrete operation labels understood by `step`. */
    readonly operations: readonly string[];
    /** The `show(a)` operation label for surface `a` (used by Invariants 2, 5, 6). */
    readonly show: (surface: ViewMode) => string;
    /** The `run` operation label (used by Invariant 6.ii). */
    readonly run: string;
    readonly step: (state: S, op: string) => S;
}

/** Breadth-first enumeration of the reachable configurations C from c0. */
const reachableStates = <S>(lts: PresentationLTS<S>): S[] => {
    const seen = new Map<string, S>();
    const queue: S[] = [lts.initial];
    seen.set(lts.key(lts.initial), lts.initial);
    while (queue.length > 0) {
        const state = queue.shift() as S;
        for (const op of lts.operations) {
            const next = lts.step(state, op);
            const id = lts.key(next);
            if (!seen.has(id)) {
                seen.set(id, next);
                queue.push(next);
            }
        }
    }
    return [...seen.values()];
};

/** Is surface `a` exposable from `state` via some finite operation sequence? */
const canExpose = <S>(lts: PresentationLTS<S>, state: S, surface: ViewMode): boolean => {
    const seen = new Set<string>([lts.key(state)]);
    const queue: S[] = [state];
    while (queue.length > 0) {
        const current = queue.shift() as S;
        if (lts.visible(current).has(surface)) return true;
        for (const op of lts.operations) {
            const next = lts.step(current, op);
            const id = lts.key(next);
            if (!seen.has(id)) {
                seen.add(id);
                queue.push(next);
            }
        }
    }
    return false;
};

const sortedConfig = <S>(lts: PresentationLTS<S>, state: S): string =>
    [...lts.visible(state)].sort().join(',');

// --- Model 1: desktop presentation profile (two columns) --------------------
// State = the full LayoutState; configuration = { left, right }. Selecting a
// surface runs the shipped `updateDesktopModes` coupling core.
const desktopProfile: PresentationLTS<LayoutState> = {
    name: 'desktop',
    initial: createGuiLayoutState(),
    key: (s) => `${s.currentLeftMode}|${s.currentRightMode}`,
    visible: (s) => new Set<ViewMode>([s.currentLeftMode, s.currentRightMode]),
    operations: ['show:input', 'show:output', 'show:stack', 'show:dictionary', 'run'],
    show: (surface) => `show:${surface}`,
    run: 'run',
    step: (s, op) => {
        const next: LayoutState = { ...s };
        // Running surfaces Output (which the coupling core pins next to Stack).
        const mode = op === 'run' ? 'output' : (op.slice('show:'.length) as ViewMode);
        updateDesktopModes(next, mode);
        return next;
    },
};

// --- Model 2: single-surface presentation profile (mobile) ------------------
// State = the one visible surface; configuration = { surface }. Swipes advance/
// retreat through VIEW_ORDER; running moves to Stack (triple-tap, per the mobile
// editor affordances); direct selection shows a surface.
type MobileState = ViewMode;
const mobileProfile: PresentationLTS<MobileState> = {
    name: 'mobile',
    initial: 'input',
    key: (s) => s,
    visible: (s) => new Set<ViewMode>([s]),
    operations: ['advance', 'retreat', 'show:input', 'show:output', 'show:stack', 'show:dictionary', 'run'],
    show: (surface) => `show:${surface}`,
    run: 'run',
    step: (s, op) => {
        if (op === 'advance') return resolveNextViewMode(s, 'left');
        if (op === 'retreat') return resolveNextViewMode(s, 'right');
        if (op === 'run') return 'stack';
        return op.slice('show:'.length) as ViewMode;
    },
};

const PROFILES: ReadonlyArray<PresentationLTS<unknown>> = [
    desktopProfile as PresentationLTS<unknown>,
    mobileProfile as PresentationLTS<unknown>,
];

describe('Presentation Profile LTS — observation surfaces are device-independent', () => {
    it('both profiles range over exactly the four SPEC §12.3 surfaces', () => {
        expect([...SURFACES].sort()).toEqual(['dictionary', 'input', 'output', 'stack']);
        expect([...VIEW_ORDER].sort()).toEqual([...SURFACES].sort());
    });
});

describe.each(PROFILES)('Presentation Profile invariants — $name model', (lts) => {
    const states = reachableStates(lts);

    it('has a non-empty reachable configuration space C', () => {
        expect(states.length).toBeGreaterThan(0);
    });

    // Invariant 1 — Partition: each reachable c partitions A into visible/hidden.
    it('Invariant 1 (Partition): visible ⊆ A and visible ⊎ hidden = A', () => {
        for (const state of states) {
            const visible = lts.visible(state);
            for (const surface of visible) expect(SURFACES).toContain(surface);
            const hidden = SURFACES.filter((s) => !visible.has(s));
            expect(visible.size + hidden.length).toBe(SURFACES.length);
            for (const surface of hidden) expect(visible.has(surface)).toBe(false);
        }
    });

    // Invariant 2 — Reachability: every surface is exposable from everywhere.
    it('Invariant 2 (Reachability): every surface is exposable from every c', () => {
        for (const state of states) {
            for (const surface of SURFACES) {
                expect(canExpose(lts, state, surface)).toBe(true);
            }
        }
    });

    // Invariant 3 — Non-emptiness: the user is never shown nothing.
    it('Invariant 3 (Non-emptiness): every reachable c shows ≥ 1 surface', () => {
        for (const state of states) {
            expect(lts.visible(state).size).toBeGreaterThanOrEqual(1);
        }
    });

    // Invariant 4 — Determinism: → is a (partial) function.
    it('Invariant 4 (Determinism): step is a function of (c, σ)', () => {
        for (const state of states) {
            for (const op of lts.operations) {
                expect(lts.key(lts.step(state, op))).toBe(lts.key(lts.step(state, op)));
            }
        }
    });

    // Invariant 5 — Idempotent selection: selecting a visible surface is a no-op.
    // Holds only over the *reachable* space: for desktop, the coupling rules
    // (Invariant 6) keep the conflicting { output, dictionary } config out of C,
    // which is precisely what makes show(visible surface) a no-op everywhere in C.
    it('Invariant 5 (Idempotent selection): show(a) with a ∈ c fixes c', () => {
        for (const state of states) {
            for (const surface of lts.visible(state)) {
                const after = lts.step(state, lts.show(surface));
                expect(sortedConfig(lts, after)).toBe(sortedConfig(lts, state));
            }
        }
    });

    // Invariant 6 — Semantic coupling: visibility tracks intent, not geometry.
    it('Invariant 6.i (Editing is observable): the edit entry shows Input', () => {
        // Selecting Input must make the edit buffer observable.
        const afterEdit = lts.step(lts.initial, lts.show('input'));
        expect(lts.visible(afterEdit).has('input')).toBe(true);
    });

    it('Invariant 6.ii (Execution is observable): run surfaces Stack from every c', () => {
        for (const state of states) {
            const afterRun = lts.step(state, lts.run);
            expect(lts.visible(afterRun).has('stack')).toBe(true);
        }
    });

    it('Invariant 6.iii (Selection feeds editing): π_Input stays reachable from every c', () => {
        for (const state of states) {
            expect(canExpose(lts, state, 'input')).toBe(true);
        }
    });
});

describe('Desktop coupling carves out the conflicting configuration', () => {
    it('{ output, dictionary } is unreachable (Output and Dictionary never coexist)', () => {
        const configs = reachableStates(desktopProfile).map((s) => sortedConfig(desktopProfile, s));
        expect(configs).not.toContain('dictionary,output');
        // The three reachable desktop configurations, for the record.
        expect([...new Set(configs)].sort()).toEqual([
            'dictionary,input',
            'input,stack',
            'output,stack',
        ]);
    });
});
