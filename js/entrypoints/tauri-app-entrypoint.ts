import { startApp } from '../application/start-app';
import { createTauriPlatformServices } from '../platform/tauri/create-tauri-platform-services';

export async function startTauriApp(): Promise<void> {
    console.log('[Main] Starting Ajisai Tauri application...');

    await startApp({
        mode: 'tauri',
        platform: createTauriPlatformServices()
    });
}
