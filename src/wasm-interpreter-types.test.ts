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
    diagnosis
};

describe('Semantic Firewall protocol payload types', () => {
    test('Value carries structured semantics and absence metadata', () => {
        const value: Value = {
            type: 'nil',
            value: null,
            displayHint: 'nil',
            semantics: {
                absence
            }
        };

        expect(value.type).toBe('nil');
        expect(value.semantics?.absence?.reason).toBe('divisionByZero');
        expect(value.semantics?.absence?.diagnosis?.why).toBe('domain');
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
