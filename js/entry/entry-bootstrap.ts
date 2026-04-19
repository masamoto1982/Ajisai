import { detectRuntimeKind } from '../platform/runtime-kind';

const start = async (): Promise<void> => {
    const runtime = detectRuntimeKind();

    if (runtime === 'tauri') {
        const { bootTauriEntry } = await import('./entry-tauri');
        await bootTauriEntry();
        return;
    }

    const { bootWebEntry } = await import('./entry-web');
    await bootWebEntry();
};

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        void start();
    }, { once: true });
} else {
    void start();
}
