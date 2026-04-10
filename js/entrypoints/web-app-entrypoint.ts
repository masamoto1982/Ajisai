import { startApp } from '../application/start-app';
import { monitorWebOnlineStatus } from '../infrastructure/web/web-online-status';
import { registerWebServiceWorker } from '../infrastructure/web/web-service-worker';
import { createWebPlatformServices } from '../platform/web/create-web-platform-services';

export async function startWebApp(): Promise<void> {
    console.log('[Main] Starting Ajisai web application...');

    await startApp({
        mode: 'web',
        platform: createWebPlatformServices(),
        enableWebHooks: (gui) => {
            registerWebServiceWorker({
                onUpdateReady: () => gui.extractDisplay().renderInfo('New version available. Please reload.', true)
            });

            monitorWebOnlineStatus({
                onOnline: () => gui.extractDisplay().renderInfo('Online mode', true),
                onOffline: () => gui.extractDisplay().renderInfo('Offline mode', true)
            }, document.getElementById('offline-indicator'));
        }
    });
}
