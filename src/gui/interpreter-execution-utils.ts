
import {
    applyInterpreterSnapshot,
    createInterpreterSnapshot,
    type InterpreterSnapshot,
    type SerialInboxEntry
} from '../workers/interpreter-snapshot';
import { getPlatform } from '../platform';
import type { AjisaiInterpreter, ExecuteResult, UserWord } from '../wasm-interpreter-types';

// Drain any host-received serial bytes so this run's SERIAL@READ sees the data
// that arrived since the previous run. Returns undefined when nothing is open.
const collectSerialInbox = (): SerialInboxEntry[] | undefined => {
    const entries = getPlatform().serial.drainAllInboxes();
    return entries.length > 0 ? entries : undefined;
};

export const collectUserWords = (interpreter: AjisaiInterpreter): UserWord[] => {
    const userWordsInfo = interpreter.collect_user_words_info();
    return userWordsInfo.map(wordData => ({
        dictionary: wordData[0],
        name: wordData[1],
        definition: interpreter.lookup_word_definition(`${wordData[0]}@${wordData[1]}`)
    }));
};

export const createExecutionSnapshot = (interpreter: AjisaiInterpreter): InterpreterSnapshot =>
    createInterpreterSnapshot({
        stack: interpreter.collect_stack(),
        // Carry the lossless snapshot into the worker so exact values on the
        // stack (CodeBlock, ExactScalar) are not flattened by the observation
        // format before this run executes. Undefined against a wasm bundle that
        // predates the API — the worker then falls back to `stack` (SPEC §2.3).
        stackSnapshot: interpreter.snapshot_stack?.(),
        userWords: collectUserWords(interpreter),
        importedModules: interpreter.collect_imported_modules(),
        executionMode: interpreter.get_execution_mode(),
        serialInbox: collectSerialInbox(),
        // Host-configured step budget (SPEC §5.3 water level); undefined
        // keeps the interpreter default of 100,000.
        stepLimit: getPlatform().executionConfig.stepLimit
    });

export const syncInterpreterState = (
    interpreter: AjisaiInterpreter,
    result: ExecuteResult
): void => {
    if (!result || result.error) return;
    applyInterpreterSnapshot(interpreter, {
        stack: result.stack,
        // Prefer the worker's lossless snapshot so the post-run stack restored
        // into the main-thread interpreter keeps its exact values (SPEC §2.3).
        stackSnapshot: result.stackSnapshot,
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
