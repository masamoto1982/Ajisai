
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

// A word is addressed by its bare name. The dictionary has two tiers and User
// is one of them (LANG.DICTIONARY.RESOLUTION), so there is nothing for a
// `DICT@NAME` prefix to select and the interpreter no longer resolves one:
// looking a word up as `USER@FOO` returns null. That null then travelled the
// whole execution path — `restore_user_words` skips a definition-less word, so
// the worker ran without the user's words and reported none back, and the
// post-run sync wiped them from the main interpreter. Every run then looked
// like a dictionary change and dragged the right column to the Words sheet.
export const collectUserWords = (interpreter: AjisaiInterpreter): UserWord[] => {
    const userWordsInfo = interpreter.collect_user_words_info();
    return userWordsInfo.map(wordData => ({
        dictionary: wordData[0],
        name: wordData[1],
        definition: interpreter.lookup_word_definition(wordData[1])
    }));
};

export const createExecutionSnapshot = (interpreter: AjisaiInterpreter): InterpreterSnapshot =>
    createInterpreterSnapshot({
        stack: interpreter.collect_stack(),
        // Carry the lossless snapshot into the worker so exact values on the
        // stack (CodeBlock, ExactScalar) are not flattened by the observation
        // format before this run executes (SPEC §2.3).
        stackSnapshot: interpreter.snapshot_stack(),
        userWords: collectUserWords(interpreter),
        serialInbox: collectSerialInbox(),
        // Host-configured step budget (SPEC §5.3 water level); undefined
        // keeps the interpreter default of 100,000.
        stepLimit: getPlatform().executionConfig.stepLimit
    });

// What a failed run printed, with its dictionary claims corrected.
//
// `syncInterpreterState` below ignores an ERROR result, so the session keeps
// its pre-run dictionary and every `DEF` the failed run reached is discarded —
// but each of those printed `Defined word: X` on its way through, and those
// lines are still in the output the error path shows. A reader who believes
// them finds out only when `LOOKUP` answers `Unknown word` for something the
// log says exists; the tester who hit this lost seven definitions that way.
// The correction goes below them, where it cancels what they claimed.
export const describeFailedRunOutput = (result: ExecuteResult): string => {
    const output = result.output || '';
    const discarded = result.discardedDictionaryChanges ?? [];
    if (discarded.length === 0) return output;
    const plural = discarded.length === 1 ? '' : 's';
    const correction =
        `Rolled back ${discarded.length} dictionary change${plural}: ${discarded.join(', ')}. ` +
        'The run failed, so the dictionary is unchanged and the lines above it do not hold.';
    return output ? `${output.replace(/\n*$/, '')}\n${correction}` : correction;
};

export const syncInterpreterState = (
    interpreter: AjisaiInterpreter,
    result: ExecuteResult
): void => {
    if (!result || result.error) return;
    applyInterpreterSnapshot(interpreter, {
        stack: result.stack,
        // The worker's lossless snapshot is what restores the post-run stack
        // into the main-thread interpreter, so it keeps its exact values
        // (SPEC §2.3).
        stackSnapshot: result.stackSnapshot,
        userWords: result.userWords
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
