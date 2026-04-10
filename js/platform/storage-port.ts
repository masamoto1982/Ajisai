import type { InterpreterState } from '../gui/interpreter-state-persistence';

export interface StoragePort {
    open(): Promise<void>;
    saveInterpreterState(state: InterpreterState): Promise<void>;
    loadInterpreterState(): Promise<InterpreterState | null>;
    clearAll(): Promise<void>;
}
