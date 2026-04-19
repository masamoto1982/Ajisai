export type RuntimeKind = 'web' | 'tauri';

declare const __AJISAI_TARGET__: RuntimeKind;

export function detectRuntimeKind(): RuntimeKind {
    if (typeof __AJISAI_TARGET__ !== 'undefined' && __AJISAI_TARGET__ === 'tauri') {
        return 'tauri';
    }

    if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
        return 'tauri';
    }

    return 'web';
}
