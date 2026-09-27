// Canonical Core Word spelling, as the Dictionary sheet must recognize it.
//
// The Core sheet lists what the interpreter reports as Core vocabulary and
// drops anything that is not a canonical name (a symbolic alias is a surface
// form of a Word, not a second Word, and must not be listed twice). That
// filter is only as honest as its idea of "canonical": a predicate narrower
// than the real name grammar silently deletes a real Word from the sheet, and
// the sheet is exactly where a reader — a human or an agent — goes to find out
// what the language has. `NIL?` disappeared that way, leaving 56 of the 57
// canonical Words visible while `NIL?` kept working when typed.
//
// The grammar below is the canonical-name pattern from spec/words.schema.json,
// which is the schema every entry in spec/words.json is validated against. It
// is duplicated here rather than derived because the GUI cannot read the spec
// at runtime; `core-word-name.test.ts` closes that gap by asserting the
// predicate against spec/words.json itself, so a future name shape that the
// schema admits cannot go missing from the sheet unnoticed.
export const isCanonicalCoreWordName = (name: string): boolean =>
    /^(?:[A-Z][A-Z0-9@?-]*(?:@[A-Z][A-Z0-9@?-]*)?|>[A-Z][A-Z0-9@?-]*)$/.test(name);
