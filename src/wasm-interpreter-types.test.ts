import { describe, expect, test } from 'vitest';
import type {
    ErrorFlowTraceEvent,
    ProtocolAbsence,
    ProtocolDiagnosis,
    Value
} from './wasm-interpreter-types';

const diagnosis: ProtocolDiagnosis = {
    when: 'executeWord',
    where: {
        kind: 'coreWord',
        word: 'DIV'
    },
    why: 'domain',
    summary: 'Division by zero produced a Bubble/NIL.',
    evidence: ['right operand was zero'],
    nextChecks: [
        {
            label: 'Check divisor',
            detail: 'Ensure the divisor is non-zero before division.'
        }
    ]
};

const absence: ProtocolAbsence = {
    reason: 'divisionByZero',
    origin: 'divisionByZero',
    recoverability: 'recoverable',
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
                origin: 'unknown',
                absence
            }
        };

        expect(value.semantics?.semanticKind).toBe('absence');
        expect(value.semantics?.absence?.reason).toBe('divisionByZero');
        expect(value.semantics?.absence?.origin).toBe('divisionByZero');
        expect(Object.hasOwn(value, ['nil', 'Reason'].join(''))).toBe(false);
        expect(Object.hasOwn(value, ['error', 'Category'].join(''))).toBe(false);
    });

    test('Error flow trace uses absence fields only', () => {
        const event: ErrorFlowTraceEvent = {
            kind: 'nilProduced',
            word: 'DIV',
            absence,
            stackLenBefore: 2,
            stackLenAfter: 3,
            message: 'NIL produced by DIV stack_len_after=3',
            diagnosis
        };

        expect(event.absence?.diagnosis?.when).toBe('executeWord');
        expect(event.diagnosis?.where.kind).toBe('coreWord');
        expect(Object.hasOwn(event, ['nil', 'Reason'].join(''))).toBe(false);
        expect(Object.hasOwn(event, ['error', 'Category'].join(''))).toBe(false);
    });
});
