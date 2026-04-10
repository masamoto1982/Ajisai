export interface OnlineStatusCallbacks {
    readonly onOnline: () => void;
    readonly onOffline: () => void;
}

export const monitorWebOnlineStatus = (
    callbacks: OnlineStatusCallbacks,
    offlineIndicator?: HTMLElement | null
): void => {
    let isInitialCheck = true;

    const updateOnlineStatus = (): void => {
        if (navigator.onLine) {
            offlineIndicator?.style.setProperty('display', 'none');
            if (!isInitialCheck) callbacks.onOnline();
        } else {
            offlineIndicator?.style.setProperty('display', 'inline');
            callbacks.onOffline();
        }
        isInitialCheck = false;
    };

    window.addEventListener('online', updateOnlineStatus);
    window.addEventListener('offline', updateOnlineStatus);
    updateOnlineStatus();
};
