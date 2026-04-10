export interface WebServiceWorkerCallbacks {
    readonly onUpdateReady?: () => void;
}

export const registerWebServiceWorker = ({ onUpdateReady }: WebServiceWorkerCallbacks = {}): void => {
    if (!('serviceWorker' in navigator)) return;

    window.addEventListener('load', () => {
        navigator.serviceWorker.register('./service-worker.js')
            .then((registration) => {
                registration.addEventListener('updatefound', () => {
                    const newWorker = registration.installing;
                    newWorker?.addEventListener('statechange', () => {
                        if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
                            onUpdateReady?.();
                        }
                    });
                });
            })
            .catch((error) => {
                console.error('[Main] Service Worker registration failed:', error);
            });
    });
};
