/** What was thrown, as an Error: a thrown non-Error is wrapped, never cast. */
export const toError = (thrown: unknown): Error =>
    thrown instanceof Error ? thrown : new Error(String(thrown));
