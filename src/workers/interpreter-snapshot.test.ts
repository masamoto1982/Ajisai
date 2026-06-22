// Adversarial robustness for snapshot application. The parameter is an
// explicitly partial/untrusted snapshot, so a malformed serialInbox (non-array,
// null entries, missing or non-array bytes) must be tolerated rather than throw
// a TypeError out of `Uint8Array.from(...)` and abort the whole restore.

import { describe, expect, test, vi } from 'vitest';
import { applyInterpreterSnapshot } from './interpreter-snapshot';
import type { AjisaiInterpreter } from '../wasm-interpreter-types';

const makeMock = () => {
    const fns = {
        reset: vi.fn(() => ({})),
        restore_imported_modules: vi.fn(),
        restore_stack: vi.fn(),
        restore_user_words: vi.fn(),
        set_execution_mode: vi.fn(),
        update_serial_inbox: vi.fn(),
        mark_serial_disconnected: vi.fn(),
    };
    return { fns, interpreter: fns as unknown as AjisaiInterpreter };
};

describe('applyInterpreterSnapshot robustness', () => {
    const malformed: unknown[] = [
        null, undefined, {},
        { serialInbox: 5 },
        { serialInbox: 'abc' },
        { serialInbox: [null] },
        { serialInbox: [{ portId: 'a' }] },
        { serialInbox: [{ portId: 'a', bytes: null }] },
        { serialInbox: [{ portId: 'a', bytes: 5 }] },
        { serialInbox: [{ bytes: [1, 2, 3] }] },
        { serialInbox: [{ portId: 123, bytes: [1] }] },
    ];
    for (const snapshot of malformed) {
        test(`never throws on ${JSON.stringify(snapshot)}`, () => {
            const { interpreter } = makeMock();
            expect(() => applyInterpreterSnapshot(interpreter, snapshot as never)).not.toThrow();
        });
    }

    test('applies a valid serial entry and marks disconnect', () => {
        const { fns, interpreter } = makeMock();
        applyInterpreterSnapshot(interpreter, {
            serialInbox: [{ portId: 'COM1', bytes: [1, 2, 3], disconnected: true }],
        } as never);
        expect(fns.update_serial_inbox).toHaveBeenCalledTimes(1);
        const [portId, bytes] = fns.update_serial_inbox.mock.calls[0]!;
        expect(portId).toBe('COM1');
        expect(Array.from(bytes as Uint8Array)).toEqual([1, 2, 3]);
        expect(fns.mark_serial_disconnected).toHaveBeenCalledWith('COM1');
    });

    test('skips a malformed entry but keeps a valid sibling', () => {
        const { fns, interpreter } = makeMock();
        applyInterpreterSnapshot(interpreter, {
            serialInbox: [null, { portId: 'COM2', bytes: [9] }, { portId: 'bad' }],
        } as never);
        expect(fns.update_serial_inbox).toHaveBeenCalledTimes(1);
        expect(fns.update_serial_inbox.mock.calls[0]![0]).toBe('COM2');
    });
});
