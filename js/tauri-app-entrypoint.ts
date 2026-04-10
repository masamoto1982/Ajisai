import { startTauriApp } from './entrypoints/tauri-app-entrypoint';

document.addEventListener('DOMContentLoaded', () => {
    void startTauriApp();
});
