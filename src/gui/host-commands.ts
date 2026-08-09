// The names the *host* answers to, rather than the language.
//
// Kept apart from `execution-controller` so it is a pure function with no
// worker, no interpreter and no DOM behind it — the shape the test suite
// exercises directly.

/// A **host command**: input the host acts on instead of handing to the
/// interpreter.
///
/// These are not Words, and the distinction is worth keeping sharp. `RESET`
/// discards the session and `CLEAR` discards the stack, and neither belongs in
/// the vocabulary: a program must not be able to throw away the values it was
/// handed, or the dictionary it was defined in. They are the typed spelling of
/// controls that also exist as buttons and shortcuts — `Ctrl+Alt+Enter`, and
/// the Stack area's `×` / `Shift+Alt+C` — for people who would rather not leave
/// the keyboard.
///
/// A lookup belongs here for a different reason. It used to be `LOOKUP`, a
/// Word, and asking what `ADD` does went through the interpreter and came back
/// on a channel no evaluation rule read: the answer was never a value, and no
/// program could do anything with it. Reading the reference is something the
/// person at the keyboard does, so the host reads the request itself.
export type HostCommand =
    | { readonly kind: 'RESET' }
    | { readonly kind: 'CLEAR' }
    | { readonly kind: 'LOOKUP'; readonly name: string };

/// The bare names, each its own whole input.
const BARE_COMMANDS = ['RESET', 'CLEAR'] as const;

/// The two spellings of a lookup, `'ADD' ?` and `'ADD' LOOKUP`, in either the
/// quoted form the reference teaches or a bare name.
///
/// The quoted form is kept because it is what every example, every reference
/// entry and everybody's fingers already have; nothing anyone learned about
/// looking a Word up stopped being true. The bare form is accepted because the
/// quotes were a *language* construct — they made a String for a Word to
/// consume — and there is no longer a Word consuming anything, so requiring
/// them would be asking for punctuation that no longer means what it did. No
/// program can be caught by the bare form: a trailing `?` is not a Word, so
/// `ADD ?` has no reading as a program to lose.
const LOOKUP_PATTERN = /^(?:'([^']*)'|(\S+))\s+(\?|LOOKUP)$/i;

/// Which host command `code` is, if any.
///
/// Two rules keep a host command from eating a program. The request has to be
/// the *whole* input, so `1 CLEAR` is a program and fails as an unknown word if
/// `CLEAR` is not defined. And a User Word of the command's name wins: nothing
/// reserves these names — `DEF` accepts them all — so without the check,
/// defining `CLEAR` would produce a word that could never be called on its own,
/// because the host would answer first and silently. For a lookup the name that
/// must not be shadowed is the mark, `?` or `LOOKUP`, not the word being looked
/// up: `'CLEAR' ?` asks about `CLEAR` whether or not `CLEAR` is defined. The
/// language is what the user is here for; the convenience yields to it.
///
/// Matching is case-insensitive, as Word lookup is.
export const resolveHostCommand = (
    code: string,
    isUserWord: (name: string) => boolean
): HostCommand | null => {
    const trimmed = code.trim();

    const bare = BARE_COMMANDS.find((candidate) => candidate === trimmed.toUpperCase());
    if (bare) return isUserWord(bare) ? null : { kind: bare };

    const lookup = LOOKUP_PATTERN.exec(trimmed);
    if (!lookup) return null;
    const [, quotedName, bareName, mark] = lookup;
    if (!mark || isUserWord(mark.toUpperCase())) return null;

    const name = (quotedName ?? bareName ?? '').trim();
    if (!name) return null;
    return { kind: 'LOOKUP', name };
};
