import type { PlatformAdapter } from '../platform-adapter';
import webPersistence from './web-persistence';
import { WebFileIO } from './web-file-io';
import { WebSerialAdapter } from './web-serial';

declare const __AJISAI_BUILD_TIMESTAMP__: string;


export const WEB_PLATFORM_ADAPTER: PlatformAdapter = {
    persistence: webPersistence,
    fileIO: new WebFileIO(),
    serial: new WebSerialAdapter(),
    // Host execution settings seam (SPEC §5.3 water levels). Empty = all
    // interpreter defaults; an embedding host fills in e.g. stepLimit here.
    executionConfig: {},
    runtime: {
        kind: 'web',
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
