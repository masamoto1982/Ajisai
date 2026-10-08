// A worker that errors is replaced, and the task queued behind the one it was
// running goes to the replacement. It used to go to the broken worker, which
// was still in the pool when the queue was drained, and answered only at the
// wall-clock guard, reported as a program that ran too long.
//
// `Worker` is a fake that records what it is sent; the pool holds one worker.

import { afterEach, expect, it, vi } from 'vitest';

interface FakeWorkerShape {
    onerror: ((event: { message: string }) => void) | null;
    posted: Array<{ type: string; code?: string }>;
    terminated: boolean;
}

const env = vi.hoisted(() => {
    const workers: FakeWorkerShape[] = [];
    class FakeWorker {
        onmessage: ((event: unknown) => void) | null = null;
        onerror: ((event: { message: string }) => void) | null = null;
        posted: Array<{ type: string; code?: string }> = [];
        terminated = false;
        constructor() { workers.push(this); }
        postMessage(message: { type: string; code?: string }) { this.posted.push(message); }
        terminate() { this.terminated = true; }
    }
    (globalThis as any).window = { innerWidth: 1200 };
    (globalThis as any).Worker = FakeWorker;
    // Node 20 (CI) has no global `navigator`; later versions do.
    Object.defineProperty(globalThis, 'navigator', {
        value: { hardwareConcurrency: 1 },
        configurable: true,
        writable: true
    });
    return { workers };
});

import { WorkerManager } from './execution-worker-manager';

const executed = (worker: FakeWorkerShape): string[] =>
    worker.posted.filter((message) => message.type === 'execute').map((message) => message.code!);

afterEach(() => { vi.useRealTimers(); });

it('hands the task queued behind an errored worker to its replacement', async () => {
    vi.useFakeTimers();
    const manager = new WorkerManager();
    await manager.init();
    const [broken] = env.workers;
    const settled: string[] = [];
    manager.execute('A', { userWords: [] }).catch((error: Error) => settled.push(`A: ${error.message}`));
    void manager.execute('B', { userWords: [] }).then(() => settled.push('B ok'));

    broken!.onerror!({ message: 'boom' });
    await Promise.resolve();

    expect(executed(broken!)).toEqual(['A']);
    expect(broken!.terminated).toBe(true);
    expect(executed(env.workers[1]!)).toEqual(['B']);
    expect(settled).toEqual(['A: Worker error: boom']);
});
