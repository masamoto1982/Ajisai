import type { DialogPort } from '../dialog-port';

export const WEB_DIALOG_PORT: DialogPort = {
    async confirm(message: string): Promise<boolean> {
        return window.confirm(message);
    },
    async alert(message: string): Promise<void> {
        window.alert(message);
    }
};
