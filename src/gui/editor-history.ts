// Recall of previously run source.
//
// A successful Run empties the editor, and Reset empties it too. The Stack, by
// contrast, persists across runs. That combination is what makes the Playground
// awkward to learn in: the natural loop — run something, change one token, run
// it again — required retyping the whole program every time, and a Reset after
// a run that did not happen took the text with it.
//
// So every submitted program is remembered for the session and can be walked
// back into the editor. This is editor convenience in the same class as the
// suggestion panel: it stores source text only, never a value, never a
// dictionary entry, and it cannot change what a program observes.

/** Programs kept for recall. Past this the oldest are dropped. */
export const MAX_HISTORY_ENTRIES = 100;

export interface EditorHistory {
    /** Remember a submitted program and rewind the cursor to the newest end. */
    readonly record: (source: string) => void;
    /**
     * Step one entry towards older, returning the source to show, or `null`
     * when there is nothing older (leave the editor as it is).
     *
     * `draft` is the editor's current text; it is stashed on the first step
     * back so stepping forward again returns what the user was typing.
     */
    readonly recallOlder: (draft: string) => string | null;
    /**
     * Step one entry towards newer. Returns the source to show — the stashed
     * draft (possibly empty) once past the newest entry — or `null` when the
     * cursor is already at the newest end.
     */
    readonly recallNewer: () => string | null;
    /** Entries currently held, oldest first. Recall state is not included. */
    readonly entries: () => readonly string[];
}

export const createEditorHistory = (limit: number = MAX_HISTORY_ENTRIES): EditorHistory => {
    const entries: string[] = [];
    // Index into `entries` of the entry currently recalled; `entries.length`
    // means "not recalling — the editor holds the draft".
    let cursor = 0;
    let stashedDraft = '';

    const record = (source: string): void => {
        const trimmed = source.trim();
        // An empty submission is not a program, and re-running the identical
        // program should not push a second copy: recall is for finding what you
        // wrote, and a run of duplicates buries it.
        if (trimmed !== '' && entries[entries.length - 1] !== trimmed) {
            entries.push(trimmed);
            if (entries.length > limit) entries.shift();
        }
        cursor = entries.length;
        stashedDraft = '';
    };

    const recallOlder = (draft: string): string | null => {
        if (cursor === 0) return null;
        if (cursor === entries.length) stashedDraft = draft;
        cursor -= 1;
        return entries[cursor]!;
    };

    const recallNewer = (): string | null => {
        if (cursor >= entries.length) return null;
        cursor += 1;
        return cursor === entries.length ? stashedDraft : entries[cursor]!;
    };

    return {
        record,
        recallOlder,
        recallNewer,
        entries: () => entries
    };
};
