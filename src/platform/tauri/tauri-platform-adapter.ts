import type { PlatformAdapter } from '../platform-adapter';
import { TauriFileIO } from './tauri-file-io';
import { TauriPersistence } from './tauri-persistence';

declare const __AJISAI_CHANGE_NOTE__: string;
declare const __AJISAI_BUILD_TIMESTAMP__: string;

export const TAURI_PLATFORM_ADAPTER: PlatformAdapter = {
    persistence: new TauriPersistence(),
    fileIO: new TauriFileIO(),
    runtime: {
        kind: 'tauri',
        version: __AJISAI_CHANGE_NOTE__,
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
