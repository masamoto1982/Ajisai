import type { ClipboardPort } from '../clipboard-port';

export const WEB_CLIPBOARD_PORT: ClipboardPort = {
    async writeText(text: string): Promise<void> {
        await navigator.clipboard.writeText(text);
    }
};
