// Playground view model (spreadsheet redesign plan Phase 3). The app —
// the Playground — encompasses two views: the Sheet view (the default
// home) and the Script view (the former 4-panel Playground UI, now
// positioned like Apps Script relative to the sheet). This module holds
// the DOM-free part so it stays vitest-testable.

export type PlaygroundView = 'sheet' | 'script';

/**
 * Which view a fresh page load should show. The Sheet is the home view;
 * `?view=script` (and the plan's original `?view=editor` spelling) opts
 * into the Script view, and a `#code=` payload — the Reference's "open in
 * Playground" hand-off — always lands in the Script view, where the code
 * editor lives. An explicit `?view=sheet` wins over a stray hash.
 */
export function resolveInitialPlaygroundView(search: string, hash: string): PlaygroundView {
    const requested = (new URLSearchParams(search).get('view') ?? '').toLowerCase();
    if (requested === 'script' || requested === 'editor') return 'script';
    if (requested === 'sheet') return 'sheet';
    if (hash.startsWith('#code=')) return 'script';
    return 'sheet';
}
