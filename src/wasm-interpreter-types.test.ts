import { describe, expect, test } from 'vitest';
import type {
    ErrorFlowTraceEvent,
    ProtocolAbsence,
    ProtocolDiagnosis,
    Value
} from './wasm-interpreter-types';

const diagnosis: ProtocolDiagnosis = {
    when: 'safeProjection',
    where: {
        kind: 'coreWord',
        word: 'DIV'
    },
    why: 'domain',
    summary: 'Division by zero was projected to NIL.',
    evidence: ['right operand was zero'],
    nextChecks: [
        {
            label: 'Check divisor',
            detail: 'Ensure the divisor is non-zero before division.'
        }
    ]
};

const absence: ProtocolAbsence = {
    reason: 'safeCaught',
    origin: 'safeProjection',
    recoverability: 'recoverable',
    caughtCategory: 'divisionByZero',
    diagnosis
};

describe('Semantic Firewall protocol payload types', () => {
    test('Value carries structured semantics and absence metadata', () => {
        const value: Value = {
            type: 'nil',
            value: null,
            displayHint: 'nil',
            semantics: {
                semanticKind: 'absence',
                shape: 'absence',
                capabilities: [
                    'stackItem',
                    'serializable',
                    'displayable',
                    'nilPassthrough',
                    'diagnosable',
                    'aiExplainable'
                ],
                origin: 'safeProjection',
                absence
            }
        };

        expect(value.semantics?.semanticKind).toBe('absence');
        expect(value.semantics?.absence?.reason).toBe('safeCaught');
        expect(value.semantics?.absence?.caughtCategory).toBe('divisionByZero');
        expect(Object.hasOwn(value, ['nil', 'Reason'].join(''))).toBe(false);
        expect(Object.hasOwn(value, ['error', 'Category'].join(''))).toBe(false);
    });

    test('Error flow trace uses absence instead of legacy top-level fields', () => {
        const event: ErrorFlowTraceEvent = {
            kind: 'nilProduced',
            word: 'DIV',
            absence,
            stackLenBefore: 2,
            stackLenAfter: 3,
            message: 'NIL produced by SAFE word=DIV stack_len_after=3',
            diagnosis
        };

        expect(event.absence?.diagnosis?.when).toBe('safeProjection');
        expect(event.diagnosis?.where.kind).toBe('coreWord');
        expect(Object.hasOwn(event, ['nil', 'Reason'].join(''))).toBe(false);
        expect(Object.hasOwn(event, ['error', 'Category'].join(''))).toBe(false);
    });
});
