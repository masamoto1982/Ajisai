declare global {
    interface Window {
        AjisaiConfig?: {
            appVersion?: string;
        };
    }
}

const DEFAULT_APP_VERSION = '202604102001';

export const AJISAI_APP_VERSION =
    (typeof window !== 'undefined' && window.AjisaiConfig?.appVersion) || DEFAULT_APP_VERSION;
