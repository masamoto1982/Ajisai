import type { ClipboardPort } from './clipboard-port';
import type { DialogPort } from './dialog-port';
import type { FilePort } from './file-port';
import type { StoragePort } from './storage-port';
import type { WindowPort } from './window-port';

export interface PlatformServices {
    readonly dialogs: DialogPort;
    readonly files: FilePort;
    readonly storage: StoragePort;
    readonly clipboard: ClipboardPort;
    readonly windowEnv: WindowPort;
}
