// The Stack area's drawing bound. A legal program can put a half-million
// element Vector on the stack; drawing it unbounded froze the browser tab for
// tens of seconds, so the render is capped and the undrawn tail is counted.

import { describe, expect, test } from 'vitest';
import {
    MAX_RENDERED_ELEMENTS_PER_COLLECTION,
    MAX_RENDERED_ELEMENTS_PER_STACK,
    createRenderBudget,
    formatElision,
    planCollectionRender
} from './stack-render-budget';

describe('planCollectionRender', () => {
    test('a small collection is drawn whole and elides nothing', () => {
        const budget = createRenderBudget();
        expect(planCollectionRender(3, budget)).toEqual({ shown: 3, elided: 0 });
        expect(budget.remaining).toBe(MAX_RENDERED_ELEMENTS_PER_STACK - 3);
    });

    test('an empty collection costs nothing', () => {
        const budget = createRenderBudget();
        expect(planCollectionRender(0, budget)).toEqual({ shown: 0, elided: 0 });
        expect(budget.remaining).toBe(MAX_RENDERED_ELEMENTS_PER_STACK);
    });

    test('a collection past the per-collection cap is cut there and the rest counted', () => {
        const budget = createRenderBudget();
        const plan = planCollectionRender(500_000, budget);
        expect(plan.shown).toBe(MAX_RENDERED_ELEMENTS_PER_COLLECTION);
        expect(plan.elided).toBe(500_000 - MAX_RENDERED_ELEMENTS_PER_COLLECTION);
    });

    test('many collections cannot together exceed the per-render budget', () => {
        const budget = createRenderBudget();
        let drawn = 0;
        for (let i = 0; i < 10_000; i++) {
            drawn += planCollectionRender(MAX_RENDERED_ELEMENTS_PER_COLLECTION, budget).shown;
        }
        expect(drawn).toBe(MAX_RENDERED_ELEMENTS_PER_STACK);
        expect(budget.remaining).toBe(0);
    });

    test('an exhausted budget draws nothing and elides everything', () => {
        const budget = createRenderBudget(0);
        expect(planCollectionRender(7, budget)).toEqual({ shown: 0, elided: 7 });
    });

    test('shown plus elided always accounts for the whole collection', () => {
        for (const length of [1, 99, 100, 101, 2_500, 1_000_000]) {
            const budget = createRenderBudget();
            const plan = planCollectionRender(length, budget);
            expect(plan.shown + plan.elided).toBe(length);
        }
    });

    test('a nonsense length is treated as empty rather than drawn', () => {
        const budget = createRenderBudget();
        expect(planCollectionRender(Number.NaN, budget)).toEqual({ shown: 0, elided: 0 });
        expect(planCollectionRender(-5, budget)).toEqual({ shown: 0, elided: 0 });
        expect(budget.remaining).toBe(MAX_RENDERED_ELEMENTS_PER_STACK);
    });
});

describe('formatElision', () => {
    test('states the exact undrawn count', () => {
        expect(formatElision(499_900)).toBe('… 499900 more');
    });

    test('is not Ajisai-shaped, so it cannot be misread as part of the value', () => {
        expect(formatElision(3)).not.toMatch(/^[[\]A-Z0-9'/-]+$/);
    });
});
