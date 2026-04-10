import '../indexeddb-user-word-store';
import { createAjisaiRuntimeFromWasm } from '../core/ajisai-runtime-factory';
import { createGUI } from '../gui/gui-application';
import type { PlatformServices } from '../platform/platform-services';
import { getAjisaiAppVersion } from '../ui/shared/app-version';
import { renderAjisaiHeader } from '../ui/shared/header-view';

interface StartAppOptions {
    readonly platform: PlatformServices;
    readonly mode: 'web' | 'tauri';
    readonly enableWebHooks?: (gui: ReturnType<typeof createGUI>) => void;
}

const renderStartupError = (error: unknown): void => {
    const outputDisplay = document.getElementById('output-display');
    if (!outputDisplay) return;

    outputDisplay.innerHTML = `
        <span style="color: #dc3545; font-weight: bold;">
            Application startup failed: ${(error as Error).message}
        </span>
    `;
};

export async function startApp({ platform, mode, enableWebHooks }: StartAppOptions): Promise<void> {
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
        const gui = createGUI({ runtime, root: document, platform });

        await gui.init();
        gui.updateAllDisplays();

        enableWebHooks?.(gui);
        console.log(`[Main] ${mode} application initialization completed successfully`);
    } catch (error) {
        console.error(`[Main] ${mode} application startup failed:`, error);
        renderStartupError(error);
    }
}
