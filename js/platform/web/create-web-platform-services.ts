import type { PlatformServices } from '../platform-services';
import { WEB_CLIPBOARD_PORT } from './web-clipboard-port';
import { WEB_DIALOG_PORT } from './web-dialog-port';
import { WEB_FILE_PORT } from './web-file-port';
import { WEB_STORAGE_PORT } from './web-storage-port';
import { WEB_WINDOW_PORT } from './web-window-port';

export const createWebPlatformServices = (): PlatformServices => ({
    dialogs: WEB_DIALOG_PORT,
    files: WEB_FILE_PORT,
    storage: WEB_STORAGE_PORT,
    clipboard: WEB_CLIPBOARD_PORT,
    windowEnv: WEB_WINDOW_PORT
});
