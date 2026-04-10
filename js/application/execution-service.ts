import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import {
    applyInterpreterSnapshot,
    createInterpreterSnapshot,
    type InterpreterSnapshot
} from '../workers/interpreter-snapshot';
import type { ExecuteResult, UserWord } from '../wasm-interpreter-types';
import type { AjisaiRuntime } from '../core/ajisai-runtime-types';

export interface ExecutionService {
    readonly executeCode: (code: string) => Promise<ExecuteResult>;
    readonly executeReset: () => Promise<ExecuteResult>;
    readonly abortExecution: () => void;
}

const mapWordDataToUserWord = (
    interpreter: AjisaiRuntime,
    wordData: [string, string, string | null, boolean]
): UserWord => ({
    dictionary: wordData[0],
    name: wordData[1],
    definition: interpreter.lookupWordDefinition(`${wordData[0]}@${wordData[1]}`),
    description: wordData[2]
});

const collectUserWords = (interpreter: AjisaiRuntime): UserWord[] => {
    const userWordsInfo = interpreter.collectUserWordsInfo();
    return userWordsInfo.map(wordData => mapWordDataToUserWord(interpreter, wordData));
};

const createExecutionSnapshot = (interpreter: AjisaiRuntime): InterpreterSnapshot =>
    createInterpreterSnapshot({
        stack: interpreter.collectStack(),
        userWords: collectUserWords(interpreter),
        importedModules: interpreter.collectImportedModules()
    });

const restoreInterpreterState = (interpreter: AjisaiRuntime, result: ExecuteResult): void => {
    if (!result || result.error) return;

    applyInterpreterSnapshot(interpreter, {
        stack: result.stack,
        userWords: result.userWords,
        importedModules: result.importedModules
    });
};

export const createExecutionService = (interpreter: AjisaiRuntime): ExecutionService => {
    const executeCode = async (code: string): Promise<ExecuteResult> => {
        const currentState = createExecutionSnapshot(interpreter);
        const result = await WORKER_MANAGER.execute(code, currentState);
        restoreInterpreterState(interpreter, result);
        return result;
    };

    const executeReset = async (): Promise<ExecuteResult> => {
        await WORKER_MANAGER.resetAllWorkers();
        return interpreter.reset();
    };

    const abortExecution = (): void => {
        WORKER_MANAGER.abortAll();
    };

    return {
        executeCode,
        executeReset,
        abortExecution
    };
};

export const executionServiceUtils = {
    mapWordDataToUserWord,
    collectUserWords,
    createExecutionSnapshot,
    restoreInterpreterState
};
