import { confirm, message } from '@tauri-apps/plugin-dialog';
import type { DialogPort } from '../dialog-port';

export const TAURI_DIALOG_PORT: DialogPort = {
    async confirm(messageText: string, options): Promise<boolean> {
        return await confirm(messageText, {
            title: options?.title,
            okLabel: options?.okLabel,
            cancelLabel: options?.cancelLabel
        });
    },
    async alert(messageText: string, options): Promise<void> {
        await message(messageText, {
            title: options?.title,
            kind: options?.kind,
            okLabel: options?.okLabel
        });
    }
};
