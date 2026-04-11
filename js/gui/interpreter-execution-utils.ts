
import {
    applyInterpreterSnapshot,
    createInterpreterSnapshot,
    type InterpreterSnapshot
} from '../workers/interpreter-snapshot';
import type { AjisaiInterpreter, ExecuteResult, UserWord } from '../wasm-interpreter-types';

export const collectUserWords = (interpreter: AjisaiInterpreter): UserWord[] => {
    const userWordsInfo = interpreter.collect_user_words_info();
    return userWordsInfo.map(wordData => ({
        dictionary: wordData[0],
        name: wordData[1],
        definition: interpreter.lookup_word_definition(`${wordData[0]}@${wordData[1]}`),
        description: wordData[2]
    }));
};

export const createExecutionSnapshot = (interpreter: AjisaiInterpreter): InterpreterSnapshot =>
    createInterpreterSnapshot({
        stack: interpreter.collect_stack(),
        userWords: collectUserWords(interpreter),
        importedModules: interpreter.collect_imported_modules()
    });

export const syncInterpreterState = (
    interpreter: AjisaiInterpreter,
    result: ExecuteResult
): void => {
    if (!result || result.error) return;
    applyInterpreterSnapshot(interpreter, {
        stack: result.stack,
        userWords: result.userWords,
        importedModules: result.importedModules
    });
};

export const resolveExecutionException = (
    context: string,
    error: unknown,
    showInfo: (text: string, append: boolean) => void,
    showError: (error: Error | string) => void
): void => {
    console.error(`[${context}] Execution failed:`, error);
    if (error instanceof Error && error.message.includes('aborted')) {
        showInfo('Execution aborted', true);
        return;
    }
    showError(error as Error);
};
