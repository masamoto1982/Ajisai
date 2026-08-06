// The names the *host* answers to, rather than the language.
//
// Kept apart from `execution-controller` so it is a pure function with no
// worker, no interpreter and no DOM behind it — the shape the test suite
// exercises directly.

/// A **host command**: a bare name, typed on its own into the editor, that the
/// host acts on instead of handing to the interpreter.
///
/// These are not Words, and the distinction is worth keeping sharp. `RESET`
/// discards the session and `CLEAR` discards the stack, and neither belongs in
/// the vocabulary: a program must not be able to throw away the values it was
/// handed, or the dictionary it was defined in. They are the typed spelling of
/// controls that also exist as buttons and shortcuts — `Ctrl+Alt+Enter`, and
/// the Stack area's `×` / `Shift+Alt+C` — for people who would rather not leave
/// the keyboard.
export type HostCommand = 'RESET' | 'CLEAR';

export const HOST_COMMANDS: readonly HostCommand[] = ['RESET', 'CLEAR'];

/// Which host command `code` is, if any.
///
/// Two rules keep a host command from eating a program. The name has to be the
/// *whole* input, so `1 CLEAR` is a program and fails as an unknown word if
/// `CLEAR` is not defined. And a User Word of that name wins: nothing reserves
/// these names — `DEF` accepts both — so without the check, defining `CLEAR`
/// would produce a word that could never be called on its own, because the host
/// would answer first and silently. The language is what the user is here for;
/// the convenience yields to it.
///
/// Matching is case-insensitive, as Word lookup is.
export const resolveHostCommand = (
    code: string,
    isUserWord: (name: string) => boolean
): HostCommand | null => {
    const name = code.trim().toUpperCase();
    const command = HOST_COMMANDS.find((candidate) => candidate === name);
    if (!command) return null;
    return isUserWord(name) ? null : command;
};
