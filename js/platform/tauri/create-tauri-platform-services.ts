import type { PlatformServices } from '../platform-services';
import { TAURI_CLIPBOARD_PORT } from './tauri-clipboard-port';
import { TAURI_DIALOG_PORT } from './tauri-dialog-port';
import { TAURI_FILE_PORT } from './tauri-file-port';
import { TAURI_STORAGE_PORT } from './tauri-storage-port';
import { TAURI_WINDOW_PORT } from './tauri-window-port';

export const createTauriPlatformServices = (): PlatformServices => ({
    dialogs: TAURI_DIALOG_PORT,
    files: TAURI_FILE_PORT,
    storage: TAURI_STORAGE_PORT,
    clipboard: TAURI_CLIPBOARD_PORT,
    windowEnv: TAURI_WINDOW_PORT
});
