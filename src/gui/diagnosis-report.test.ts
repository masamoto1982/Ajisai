import { describe, it, expect } from 'vitest';
import { renderDiagnosisReport } from './diagnosis-report';
import { describeTimeoutDiagnosis } from './interpreter-execution-utils';
import type { ProtocolDiagnosis } from '../wasm-interpreter-types';

const check = (code: string, title: string, detail: string) => ({
    code,
    title: { en: title, ja: title },
    detail: { en: detail, ja: detail }
});

// The reading format is asserted here and nowhere else. Both the diagnoses the
// interpreter sends and the one the host writes for a wall-clock stop render
// through this module, so a change to the frame — a renamed question, a fourth
// one, a moved ceiling line — fails here once rather than passing in one
// surface and drifting in the other.
describe('renderDiagnosisReport', () => {
    it('renders the frame a reader learns once', () => {
        const diagnosis: ProtocolDiagnosis = {
            when: 'wordExecution',
            where: { kind: 'coreWord', word: 'DIV' },
            why: 'domain',
            summary: 'wordExecution / DIV (coreWord) / domain',
            evidence: ['sourceLine=3', 'sourceColumn=7', 'insideWords=SAFE-DIV,REPORT'],
            candidates: [],
            nextChecks: [check('checkDivisor', 'Check the divisor', 'A zero divisor projects NIL.')]
        };

        expect(renderDiagnosisReport(diagnosis, { stackLenBefore: 2 })).toBe(
            [
                '[DIAGNOSIS] wordExecution / DIV (coreWord) / domain',
                'Q1 when: wordExecution',
                'Q2 where: DIV (coreWord), inside SAFE-DIV, REPORT at line 3, column 7, stack depth 2',
                'Q3 why: domain',
                'next: Check the divisor - A zero divisor projects NIL.'
            ].join('\n')
        );
    });

    // `1 + 2`: the failure is `ADD`'s, and `ADD` is not what the reader wrote.
    it('names the alias the program was written with, in front of the Word it resolved to', () => {
        const diagnosis: ProtocolDiagnosis = {
            when: 'wordExecution',
            where: { kind: 'coreWord', word: 'ADD' },
            why: 'stackShape',
            summary: 'wordExecution / ADD (coreWord) / stackShape',
            evidence: ['sourceLine=1', 'sourceColumn=3', 'sourceWord=+'],
            candidates: [],
            nextChecks: []
        };

        expect(renderDiagnosisReport(diagnosis, { stackLenBefore: 1 })).toContain(
            'Q2 where: + (alias of ADD, coreWord) at line 1, column 3, stack depth 1'
        );
    });

    it('omits the position, the depth and the hints it was given nothing for', () => {
        const diagnosis: ProtocolDiagnosis = {
            when: 'nameResolution',
            where: { kind: 'dictionary' },
            why: 'typoOrUnknownName',
            summary: 'nameResolution / dictionary / typoOrUnknownName',
            evidence: [],
            candidates: ['DUP', 'DROP'],
            nextChecks: []
        };

        expect(renderDiagnosisReport(diagnosis)).toBe(
            [
                '[DIAGNOSIS] nameResolution / dictionary / typoOrUnknownName',
                'Q1 when: nameResolution',
                'Q2 where: dictionary',
                'Q3 why: typoOrUnknownName',
                'did you mean: DUP, DROP'
            ].join('\n')
        );
    });

    it('reports a declared ceiling with what was observed against it', () => {
        const diagnosis: ProtocolDiagnosis = {
            when: 'wordExecution',
            where: { kind: 'coreWord', word: 'RANGE' },
            why: 'resourceLimit',
            summary: 'wordExecution / RANGE (coreWord) / resourceLimit',
            evidence: [],
            resourceLimit: { resource: 'materializedElements', limit: 1_000_000, observed: 4_000_000 },
            nextChecks: []
        };

        expect(renderDiagnosisReport(diagnosis)).toContain(
            'limit materializedElements: 4000000 against 1000000'
        );
    });
});

// The wall-clock guard's whole output, locked. It was a hand-written string
// literal in the shape of the frame above; this is the same text, now produced
// by the frame itself.
describe('describeTimeoutDiagnosis', () => {
    it('renders the host guard through the shared frame', () => {
        expect(describeTimeoutDiagnosis(5_000)).toBe(
            [
                '[DIAGNOSIS] hostGuard / playground (hostEnvironment) / resourceLimit (executionTimeout)',
                'Q1 when: hostGuard',
                'Q2 where: playground (hostEnvironment)',
                'Q3 why: resourceLimit',
                'limit executionTimeoutMs: 5000 (wall clock, this host only)',
                'next: Check which guard stopped it - The playground stops a run on wall-clock time. '
                    + "The interpreter's own budgets (execution steps, materialized elements, numeric work) "
                    + 'did not refuse this program; it was still running when the time ran out.',
                'next: Rewrite the loop as a bulk operation - A whole-vector Word does in one step what a '
                    + 'per-element loop does in as many, and only the loop is charged per step.',
                'next: Trim what the run carries - Exact values grow as they are combined; rounding to a grid '
                    + '(x d MUL FLOOR d DIV) bounds a denominator that is otherwise free to grow every iteration.',
                'next: Check the host profile - This guard is not part of the language. Another host '
                    + '(the MCP server) applies different limits; the profile badge beside the build version '
                    + 'lists the ones in force here.'
            ].join('\n')
        );
    });

    it('offers four next-steps, each a protocol check rather than a printed line', () => {
        // The renderer prints the `en` side of a check; assembling these as
        // `ProtocolDebugCheck`s is what gives them a `ja` side at all — a
        // hand-written English line could not carry one, which is the second
        // thing the duplicated format cost.
        const rendered = describeTimeoutDiagnosis(5_000);
        const steps = rendered.split('\n').filter((line) => line.startsWith('next: '));

        expect(steps).toHaveLength(4);
    });
});
