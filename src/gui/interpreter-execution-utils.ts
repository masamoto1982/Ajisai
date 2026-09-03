
import {
    applyInterpreterSnapshot,
    createInterpreterSnapshot,
    type InterpreterSnapshot
} from '../workers/interpreter-snapshot';
import { getPlatform } from '../platform';
import { ExecutionTimeoutError } from '../workers/execution-timeout';
import type { AjisaiInterpreter, ExecuteResult, UserWord } from '../wasm-interpreter-types';

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

// What a run that succeeded but cannot be carried into the session has to say,
// or null when there is nothing to explain.
//
// The snapshot is taken after the program has already run, so its refusal is
// not the program's failure — reporting it as one is how `PI` came to look like
// a Word that does not work. The session keeps its pre-run stack (see
// `syncInterpreterState`), which is the same answer a failed run gets, so this
// says both halves: the run worked, and its result stops here.
export const describeSnapshotRefusal = (result: ExecuteResult): string | null => {
    if (!result?.stackSnapshotError) return null;
    return [
        'The program ran and produced its result, but the session cannot keep it: ',
        result.stackSnapshotError,
        '. The stack is unchanged from before the run.'
    ].join('');
};

// The diagnosis a wall-clock stop can answer with.
//
// Every other refusal is built by the interpreter, which knows the Word, the
// position and the ceiling. This one is not: the playground terminates the
// worker where it stands, so nothing of the run survives to be diagnosed and
// the bare sentence was all the reader got. What is knowable here is knowable
// without the run — which guard fired, that it is the host's and not the
// language's, and what makes a program fit inside it.
export const describeTimeoutDiagnosis = (limitMs: number): string =>
    [
        '[DIAGNOSIS] hostGuard / playground (hostEnvironment) / resourceLimit (executionTimeout)',
        'Q1 when: hostGuard',
        'Q2 where: playground (hostEnvironment)',
        'Q3 why: resourceLimit',
        `limit executionTimeoutMs: ${limitMs} (wall clock, this host only)`,
        'next: Check which guard stopped it - The playground stops a run on wall-clock time. '
            + "The interpreter's own budgets (execution steps, materialized elements, numeric work) "
            + 'did not refuse this program; it was still running when the time ran out.',
        'next: Rewrite the loop as a bulk operation - A whole-vector Word does in one step what a '
            + 'per-element loop does in as many, and only the loop is charged per step.',
        'next: Trim what the run carries - Exact values grow as they are combined; QUANTIZE bounds a '
            + 'denominator that is otherwise free to grow every iteration.',
        'next: Check the host profile - This guard is not part of the language. Another host '
            + '(the MCP server) applies different limits; the profile badge beside the build version '
            + 'lists the ones in force here.'
    ].join('\n');

export const syncInterpreterState = (
    interpreter: AjisaiInterpreter,
    result: ExecuteResult
): void => {
    if (!result || result.error) return;
    // A result that could not be snapshotted is not applied at all: the
    // observation format is never restored from (SPEC §2.3), so applying a
    // snapshot-less state would replace the session's stack with an empty one
    // — losing what the user had, on top of the value the run just made.
    if (result.stackSnapshotError) return;
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
    // The one refusal the interpreter never gets to explain: it is stopped from
    // outside, so the diagnosis is written here instead of arriving with the
    // result. Without it the wall-clock stop was the only error in the
    // playground that answered with a bare sentence.
    if (error instanceof ExecutionTimeoutError) {
        showInfo(describeTimeoutDiagnosis(error.limitMs), true);
    }
};
