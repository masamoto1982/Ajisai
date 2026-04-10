import type DB from '../../indexeddb-user-word-store';
import type { InterpreterState } from '../../gui/interpreter-state-persistence';
import type { StoragePort } from '../storage-port';

declare global {
    interface Window {
        AjisaiDB: typeof DB;
    }
}

export const WEB_STORAGE_PORT: StoragePort = {
    async open(): Promise<void> {
        await window.AjisaiDB.open();
    },
    async saveInterpreterState(state: InterpreterState): Promise<void> {
        await window.AjisaiDB.saveInterpreterState(state);
    },
    async loadInterpreterState(): Promise<InterpreterState | null> {
        return await window.AjisaiDB.loadInterpreterState();
    },
    async clearAll(): Promise<void> {
        await window.AjisaiDB.clearAll();
    }
};
