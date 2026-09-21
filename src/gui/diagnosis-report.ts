import type { ProtocolDiagnosis } from '../wasm-interpreter-types';

// The one place that knows what a diagnosis looks like when it is read.
//
// The `[DIAGNOSIS]` heading, the three numbered questions and the `next:`
// lines are a presentation contract: a reader learns the shape once and then
// reads every refusal the same way. That shape used to be written twice — the
// renderer below, fed by the protocol, and a hand-assembled string literal for
// the wall-clock timeout, which is the one refusal the interpreter never gets
// to explain (the playground terminates the worker where it stands, so no
// diagnosis arrives with the result). The two copies were the same knowledge,
// not two look-alike blocks: renaming `Q3 why:` or adding a fourth question
// would have moved one and left the other, and the timeout's copy is the
// refusal a reader is least likely to have seen before.
//
// So the timeout builds a `ProtocolDiagnosis` like any other and renders
// through here. What is genuinely its own — a ceiling with no observed value,
// enforced by the host rather than the language — arrives as `extraLines`
// rather than as a second renderer.

const evidenceValue = (
    entries: readonly string[] | undefined,
    key: string
): string | null => {
    const hit = entries?.find((entry) => entry.startsWith(`${key}=`));
    return hit ? hit.slice(key.length + 1) : null;
};

export interface DiagnosisReportContext {
    /**
     * Stack depth at the failing step, from the error-flow event that carried
     * the diagnosis. Absent when the diagnosis did not come from a step (the
     * host-guard case).
     */
    readonly stackLenBefore?: number;
    /**
     * Lines to print where a protocol `resourceLimit` would go, for a ceiling
     * the protocol cannot describe.
     */
    readonly extraLines?: readonly string[];
}

export const renderDiagnosisReport = (
    diagnosis: ProtocolDiagnosis,
    context: DiagnosisReportContext = {}
): string => {
    // The alias the program was written with, when it is not the name the
    // Word answers to. `1 + 2` reported `ADD (coreWord)`, which is true and
    // names nothing the reader typed; the spelling they typed goes in front of
    // it, and the canonical name stays where every other line refers to it.
    const sourceWord = evidenceValue(diagnosis.evidence, 'sourceWord');
    const where = diagnosis.where.word
        ? sourceWord
            ? `${sourceWord} (alias of ${diagnosis.where.word}, ${diagnosis.where.kind})`
            : `${diagnosis.where.word} (${diagnosis.where.kind})`
        : diagnosis.where.kind;
    const depth =
        typeof context.stackLenBefore === 'number'
            ? `, stack depth ${context.stackLenBefore}`
            : '';
    // Where in the source the run was when it failed. The host records it
    // as evidence — the same `key=value` channel `stackLenBefore` uses —
    // so nothing about the protocol had to change to carry it.
    const sourceLine = evidenceValue(diagnosis.evidence, 'sourceLine');
    const at = sourceLine
        ? ` at line ${sourceLine}, column ${evidenceValue(diagnosis.evidence, 'sourceColumn')}`
        : '';
    // The Words the failure happened *inside*, innermost first. A block and
    // a Word body are each their own token stream with no source of their
    // own, so the position above is the top-level token that reached the
    // failure; this says which construct the failing word was written in.
    const insideWords = evidenceValue(diagnosis.evidence, 'insideWords');
    const inside = insideWords ? `, inside ${insideWords.split(',').join(', ')}` : '';
    // The known Words closest to a name that did not resolve. Telling a
    // reader to check the spelling without saying what it might have been
    // is the one hint nobody can act on.
    const candidates = diagnosis.candidates?.length
        ? [`did you mean: ${diagnosis.candidates.join(', ')}`]
        : [];
    // Which declared ceiling fired, so "too big" says what was too big.
    const limit = diagnosis.resourceLimit
        ? [
              `limit ${diagnosis.resourceLimit.resource}: ${
                  diagnosis.resourceLimit.observed ?? '?'
              } against ${diagnosis.resourceLimit.limit}`
          ]
        : [];
    return [
        `[DIAGNOSIS] ${diagnosis.summary}`,
        `Q1 when: ${diagnosis.when}`,
        `Q2 where: ${where}${inside}${at}${depth}`,
        `Q3 why: ${diagnosis.why}`,
        ...candidates,
        ...limit,
        ...(context.extraLines ?? []),
        // One locale per line. Each check carries both, and this used to
        // print the English heading in front of the Japanese sentence, so
        // every next-step read as half a message in each language. The
        // playground's own text is English (`<html lang="en">`), so English
        // is the side that matches its surroundings; the `ja` half stays in
        // the protocol for a host that renders in Japanese.
        ...diagnosis.nextChecks.map((check) => `next: ${check.title.en} - ${check.detail.en}`)
    ].join('\n');
};
