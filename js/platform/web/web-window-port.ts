import type { WindowPort } from '../window-port';

export const WEB_WINDOW_PORT: WindowPort = {
    getInnerWidth(): number {
        return window.innerWidth;
    },
    getHardwareConcurrency(): number {
        return navigator.hardwareConcurrency || 4;
    },
    addWindowEventListener(type, listener, options): void {
        window.addEventListener(type, listener as EventListener, options);
    },
    addDocumentEventListener(type, listener, options): void {
        document.addEventListener(type, listener as EventListener, options);
    },
    getBody(): HTMLElement {
        return document.body;
    }
};
