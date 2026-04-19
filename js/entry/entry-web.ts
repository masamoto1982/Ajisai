import { getPlatform } from '../platform';
import { initializeApplication, setBuildVersionLabel, setupLogoTouchQR } from './entry-common';

export async function bootWebEntry(): Promise<void> {
    getPlatform().runtime.onReady(() => {
        setBuildVersionLabel();
        setupLogoTouchQR();
        void initializeApplication();
    });
}
