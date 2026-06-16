import type { ExecuteResult, UserWord, Value } from '../wasm-interpreter-types';
import type { ExecutionSurfaceChanges } from './gui-layout-state';

const stableStringify = (value: unknown): string => JSON.stringify(value ?? null);

// Order-insensitive identity of the user dictionary. The pre-execution
// snapshot and the post-execution read-back can enumerate words in different
// orders (a synced interpreter rebuilds its dictionaries from scratch), so the
// set is sorted by fully-qualified name before comparison — otherwise a pure
// stack op like `2 3 +` would look like a dictionary change whenever any user
// word exists, and wrongly pull the right column to the Words sheet.
const normalizeUserWords = (words: readonly UserWord[]): string =>
    stableStringify(
        [...words]
            .map(word => ({
                dictionary: word.dictionary ?? null,
                name: word.name,
                definition: word.definition ?? null
            }))
            .sort((a, b) =>
                `${a.dictionary ?? ''}@${a.name}`.localeCompare(`${b.dictionary ?? ''}@${b.name}`))
    );

const normalizeModules = (modules: readonly string[]): string =>
    stableStringify([...modules].sort());

// A view of the surfaces an execution can touch, read from one interpreter
// instance so before/after are directly comparable.
export interface ExecutionStateView {
    readonly stack: Value[];
    readonly userWords: UserWord[];
    readonly importedModules: string[];
}

export const detectExecutionSurfaceChanges = (
    before: ExecutionStateView,
    after: ExecutionStateView,
    result: ExecuteResult
): ExecutionSurfaceChanges => {
    const userWordsChanged = normalizeUserWords(before.userWords) !== normalizeUserWords(after.userWords);

    const beforeModules = before.importedModules;
    const afterModules = after.importedModules;
    const importedModulesChanged = normalizeModules(beforeModules) !== normalizeModules(afterModules);
    // The module whose import state just flipped: prefer a newly imported
    // module, otherwise the one that was just unimported, so the dictionary
    // lands on the sheet the user actually changed.
    const changedModule = afterModules.find(name => !beforeModules.includes(name))
        ?? beforeModules.find(name => !afterModules.includes(name));

    // Errors and diagnostics render into the Output surface, so a failed run
    // changes Output even when the program emitted no text of its own.
    const hasError = result.status !== 'OK' || Boolean(result.error);

    return {
        outputChanged: hasError || Boolean((result.output ?? '').trim()),
        stackChanged: stableStringify(before.stack) !== stableStringify(after.stack),
        dictionaryChanged: userWordsChanged || importedModulesChanged,
        // Defining your own word lands on the 'user' sheet; a module import or
        // unimport lands on that module's sheet. User words win when both move.
        dictionarySheetId: userWordsChanged
            ? 'user'
            : (changedModule ? `module-${changedModule}` : undefined)
    };
};
