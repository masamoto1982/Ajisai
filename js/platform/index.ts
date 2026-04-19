import type { PlatformAdapter } from './platform-adapter';
import { detectRuntimeKind } from './runtime-kind';
import { TAURI_PLATFORM_ADAPTER } from './tauri/tauri-platform-adapter';
import { WEB_PLATFORM_ADAPTER } from './web/web-platform-adapter';

let cachedPlatform: PlatformAdapter | null = null;

export function getPlatform(): PlatformAdapter {
    if (cachedPlatform) {
        return cachedPlatform;
    }

    cachedPlatform = detectRuntimeKind() === 'tauri'
        ? TAURI_PLATFORM_ADAPTER
        : WEB_PLATFORM_ADAPTER;

    return cachedPlatform;
}
