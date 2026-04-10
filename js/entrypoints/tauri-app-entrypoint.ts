import '../indexeddb-user-word-store';
import { createAjisaiRuntimeFromWasm } from '../core/ajisai-runtime-factory';
import { createGUI } from '../gui/gui-application';
import { createTauriPlatformServices } from '../platform/tauri/create-tauri-platform-services';
import { getAjisaiAppVersion } from '../ui/shared/app-version';
import { renderAjisaiHeader } from '../ui/shared/header-view';


const renderStartupError = (error: unknown): void => {
    const outputDisplay = document.getElementById('output-display');
    if (!outputDisplay) return;

    outputDisplay.innerHTML = `
        <span style="color: #dc3545; font-weight: bold;">
            Application startup failed: ${(error as Error).message}
        </span>
    `;
};

export async function startTauriApp(): Promise<void> {
    console.log('[Main] Starting Ajisai Tauri application...');

    try {
        const headerEl = document.getElementById('js-header');
        if (headerEl instanceof HTMLElement) {
            renderAjisaiHeader(headerEl, {
                mode: 'web',
                version: getAjisaiAppVersion(),
                assetsPath: './images',
                referenceHref: 'docs/index.html'
            });
        }

        const runtime = await createAjisaiRuntimeFromWasm();
        const platform = createTauriPlatformServices();
        const gui = createGUI({ runtime, root: document, platform });

        await gui.init();
        gui.updateAllDisplays();

        console.log('[Main] Tauri application initialization completed successfully');
    } catch (error) {
        console.error('[Main] Tauri application startup failed:', error);
        renderStartupError(error);
    }
}
