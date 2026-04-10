import '../indexeddb-user-word-store';
import { createAjisaiRuntimeFromWasm } from '../core/ajisai-runtime-factory';
import { createGUI } from '../gui/gui-application';
import { monitorWebOnlineStatus } from '../infrastructure/web/web-online-status';
import { registerWebServiceWorker } from '../infrastructure/web/web-service-worker';
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

export async function startWebApp(): Promise<void> {
    console.log('[Main] Starting Ajisai application...');

    try {
        const headerEl = document.getElementById('js-header');
        if (headerEl instanceof HTMLElement) {
            renderAjisaiHeader(headerEl, {
                mode: 'web',
                version: '202604102001',
                assetsPath: 'public/images',
                referenceHref: 'docs/index.html'
            });
        }

        const runtime = await createAjisaiRuntimeFromWasm();
        const gui = createGUI({ runtime, root: document });

        await gui.init();
        gui.updateAllDisplays();

        registerWebServiceWorker({
            onUpdateReady: () => gui.extractDisplay().renderInfo('New version available. Please reload.', true)
        });

        monitorWebOnlineStatus({
            onOnline: () => gui.extractDisplay().renderInfo('Online mode', true),
            onOffline: () => gui.extractDisplay().renderInfo('Offline mode', true)
        }, document.getElementById('offline-indicator'));

        console.log('[Main] Application initialization completed successfully');
    } catch (error) {
        console.error('[Main] Application startup failed:', error);
        renderStartupError(error);
    }
}
