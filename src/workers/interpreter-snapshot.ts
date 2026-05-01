import type { AjisaiInterpreter, ExecutionMode, UserWord, Value } from '../wasm-interpreter-types';

export interface InterpreterSnapshot {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules: string[];
    readonly executionMode: ExecutionMode;
}

export const createInterpreterSnapshot = (snapshot: {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules?: string[];
    readonly executionMode?: ExecutionMode;
}): InterpreterSnapshot => ({
    stack: snapshot.stack,
    userWords: snapshot.userWords,
    importedModules: snapshot.importedModules ?? [],
    executionMode: snapshot.executionMode ?? "greedy"
});

export const applyInterpreterSnapshot = (
    interpreter: AjisaiInterpreter,
    snapshot?: Partial<InterpreterSnapshot> | null
): void => {
    interpreter.reset();
    if (!snapshot) return;

    if (snapshot.importedModules?.length) {
        interpreter.restore_imported_modules(snapshot.importedModules);
    }
    if (snapshot.stack) {
        interpreter.restore_stack(snapshot.stack);
    }
    if (snapshot.userWords) {
        interpreter.restore_user_words(snapshot.userWords);
    }
    if (snapshot.executionMode) {
        interpreter.set_execution_mode(snapshot.executionMode);
    }
};
