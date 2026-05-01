import { bootstrapApplication } from './entry-common';

const start = (): void => {
    bootstrapApplication();
};

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        start();
    }, { once: true });
} else {
    start();
}
