import type { AjisaiInterpreter, UserWord, Value } from '../wasm-interpreter-types';
import type { AjisaiRuntime } from '../core/ajisai-runtime-types';

export interface InterpreterSnapshot {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules: string[];
}

export const createInterpreterSnapshot = (snapshot: {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules?: string[];
}): InterpreterSnapshot => ({
    stack: snapshot.stack,
    userWords: snapshot.userWords,
    importedModules: snapshot.importedModules ?? []
});

export const applyInterpreterSnapshot = (
    interpreter: AjisaiRuntime | AjisaiInterpreter,
    snapshot?: Partial<InterpreterSnapshot> | null
): void => {
    interpreter.reset();
    if (!snapshot) return;

    if (snapshot.importedModules?.length) {
        if ('restoreImportedModules' in interpreter) {
            interpreter.restoreImportedModules(snapshot.importedModules);
        } else {
            interpreter.restore_imported_modules(snapshot.importedModules);
        }
    }
    if (snapshot.stack) {
        if ('restoreStack' in interpreter) {
            interpreter.restoreStack(snapshot.stack);
        } else {
            interpreter.restore_stack(snapshot.stack);
        }
    }
    if (snapshot.userWords) {
        if ('restoreUserWords' in interpreter) {
            void interpreter.restoreUserWords(snapshot.userWords);
        } else {
            interpreter.restore_user_words(snapshot.userWords);
        }
    }
};
