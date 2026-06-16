import type { PlatformAdapter } from '../platform-adapter';
import { TauriFileIO } from './tauri-file-io';
import { TauriPersistence } from './tauri-persistence';
import { TauriSerialAdapter } from './tauri-serial';

declare const __AJISAI_BUILD_TIMESTAMP__: string;

export const TAURI_PLATFORM_ADAPTER: PlatformAdapter = {
    persistence: new TauriPersistence(),
    fileIO: new TauriFileIO(),
    serial: new TauriSerialAdapter(),
    runtime: {
        kind: 'tauri',
        buildTimestamp: __AJISAI_BUILD_TIMESTAMP__,
        onReady(callback: () => void): void {
            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', callback, { once: true });
                return;
            }
            callback();
        }
    }
};
