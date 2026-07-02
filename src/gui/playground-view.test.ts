// Initial-view resolution tests (plan Phase 3: Sheet is the home view;
// Reference hand-offs land in the Script view).

import { describe, expect, test } from 'vitest';
import { resolveInitialPlaygroundView } from './playground-view';

describe('resolveInitialPlaygroundView', () => {
    test('the Sheet view is the default home', () => {
        expect(resolveInitialPlaygroundView('', '')).toBe('sheet');
        expect(resolveInitialPlaygroundView('?foo=bar', '#other')).toBe('sheet');
    });

    test('?view=script and the legacy ?view=editor open the Script view', () => {
        expect(resolveInitialPlaygroundView('?view=script', '')).toBe('script');
        expect(resolveInitialPlaygroundView('?view=editor', '')).toBe('script');
        expect(resolveInitialPlaygroundView('?view=SCRIPT', '')).toBe('script');
    });

    test('a #code= hand-off from the Reference lands in the Script view', () => {
        expect(resolveInitialPlaygroundView('', '#code=%5B%201%20%5D')).toBe('script');
    });

    test('an explicit ?view=sheet wins over a stray hash', () => {
        expect(resolveInitialPlaygroundView('?view=sheet', '#code=x')).toBe('sheet');
    });

    test('unknown view values fall back to the Sheet home', () => {
        expect(resolveInitialPlaygroundView('?view=classic', '')).toBe('sheet');
    });
});
