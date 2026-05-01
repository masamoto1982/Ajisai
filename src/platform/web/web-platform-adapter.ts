import type { PlatformAdapter } from '../platform-adapter';
import webPersistence from './web-persistence';
import { WebFileIO } from './web-file-io';

declare const __AJISAI_CHANGE_NOTE__: string;
declare const __AJISAI_BUILD_TIMESTAMP__: string;


export const WEB_PLATFORM_ADAPTER: PlatformAdapter = {
    persistence: webPersistence,
    fileIO: new WebFileIO(),
    runtime: {
        kind: 'web',
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
