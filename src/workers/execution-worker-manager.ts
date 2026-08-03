

import type { ExecuteResult } from '../wasm-interpreter-types';
import type { InterpreterSnapshot } from './interpreter-snapshot';
import { extractCompiledWasmModule } from '../wasm-module-loader';
import {
    detectParallelCapability,
    describeParallelCapability,
    type ParallelCapability,
} from '../platform/cross-origin-isolation';

interface WorkerTask {
    id: string;
    code: string;
    state: InterpreterSnapshot;
    resolve: (result: any) => void;
    reject: (error: any) => void;
    timeoutHandle: ReturnType<typeof setTimeout> | null;
}

interface WorkerInstance {
    worker: Worker;
    busy: boolean;
    currentTaskId: string | null;
}

const MOBILE_BREAKPOINT = 768;
const MAX_MOBILE_WORKERS = 2;

// Per-task wall-clock cap on worker execution. The recursion guard returns an
// AjisaiError immediately for blown-stack programs; this is the second line of
// defence for "still running" non-recursive loops that neither hit the
// execution-step cap fast enough nor produce a recursion error. Set well above
// the longest legitimate run so a legal program never trips it.
const EXECUTION_TIMEOUT_MS = 5_000;

export class WorkerManager {
    private workers: WorkerInstance[] = [];
    private taskQueue: WorkerTask[] = [];
    private activeTasks = new Map<string, WorkerTask>();
    private compiledModule: WebAssembly.Module | null = null;
    private maxWorkers = window.innerWidth <= MOBILE_BREAKPOINT
        ? Math.min(navigator.hardwareConcurrency || 2, MAX_MOBILE_WORKERS)
        : navigator.hardwareConcurrency || 4;
    // Whether SharedArrayBuffer-backed wasm threading can run in this page
    // (implicit-parallelism roadmap Phase 5). Observational for now: the pool
    // still uses snapshot-copying Web Workers regardless. Once a threaded wasm
    // build ships, `recommendedThreads` drives the wasm-bindgen-rayon pool size
    // and `threadsAvailable` gates the SharedArrayBuffer snapshot transport.
    private parallelCapability: ParallelCapability = detectParallelCapability();

    async init(): Promise<void> {
        console.log('[WorkerManager] Initializing worker pool...');
        console.log(`[WorkerManager] parallel capability: ${describeParallelCapability(this.parallelCapability)}`);
        this.workers = [];


        this.compiledModule = extractCompiledWasmModule();

        if (!this.compiledModule) {
            console.warn('[WorkerManager] Compiled WASM module not available; workers will init independently');
        }


        for (let i = 0; i < this.maxWorkers; i++) {
            this.createWorker();
        }
    }

    private createWorker(): void {
        const worker = new Worker(new URL('./interpreter-execution-worker.ts', import.meta.url), { type: 'module' });
        const instance: WorkerInstance = { worker, busy: false, currentTaskId: null };

        worker.onmessage = (event) => this.resolveWorkerMessage(instance, event.data);
        worker.onerror = (error) => this.resolveWorkerError(instance, error);


        if (this.compiledModule) {
            worker.postMessage({ type: 'init', wasmModule: this.compiledModule });
        }

        this.workers.push(instance);
    }


    private ensureWorkers(): void {
        if (this.workers.length > 0) return;
        for (let i = 0; i < this.maxWorkers; i++) {
            this.createWorker();
        }
    }

    private resolveWorkerMessage(instance: WorkerInstance, message: any): void {
        const task = this.activeTasks.get(message.id);
        if (!task) return;

        switch (message.type) {
            case 'result':
                task.resolve(message.data);
                break;
            case 'error':
                task.reject(new Error(message.data));
                break;
            case 'aborted':
                task.reject(new Error('Execution aborted'));
                break;
        }
        this.completeTask(instance);
    }

    private resolveWorkerError(instance: WorkerInstance, error: ErrorEvent): void {
        console.error('[WorkerManager] Worker error:', error.message);
        if (instance.currentTaskId) {
            const task = this.activeTasks.get(instance.currentTaskId);
            task?.reject(new Error(`Worker error: ${error.message}`));
        }
        this.completeTask(instance);
        const index = this.workers.indexOf(instance);
        if (index > -1) this.workers.splice(index, 1);
        this.createWorker();
    }

    private completeTask(instance: WorkerInstance): void {
        if (instance.currentTaskId) {
            const task = this.activeTasks.get(instance.currentTaskId);
            if (task) {
                if (task.timeoutHandle !== null) {
                    clearTimeout(task.timeoutHandle);
                    task.timeoutHandle = null;
                }
            }
            this.activeTasks.delete(instance.currentTaskId);
        }
        instance.busy = false;
        instance.currentTaskId = null;
        this.processQueue();
    }

    private handleTaskTimeout(taskId: string): void {
        const task = this.activeTasks.get(taskId);
        if (!task) return;
        task.timeoutHandle = null;

        // Terminate the worker carrying the runaway task; a terminated
        // worker cannot be reused, so we drop it from the pool and spawn
        // a replacement immediately to keep the pool size constant.
        const instance = this.workers.find(w => w.currentTaskId === taskId);
        if (instance) {
            instance.worker.terminate();
            const index = this.workers.indexOf(instance);
            if (index > -1) this.workers.splice(index, 1);
        }

        this.activeTasks.delete(taskId);

        task.reject(new Error(`Execution timed out after ${EXECUTION_TIMEOUT_MS} ms`));

        this.createWorker();
        this.processQueue();
    }

    private processQueue(): void {
        // Drain as many queued tasks as there are idle workers.
        while (this.taskQueue.length > 0) {
            const availableWorker = this.workers.find(w => !w.busy);
            if (!availableWorker) break;
            const nextTask = this.taskQueue.shift()!;
            this.assignTaskToWorker(availableWorker, nextTask);
        }
    }

    private assignTaskToWorker(instance: WorkerInstance, task: WorkerTask): void {
        instance.busy = true;
        instance.currentTaskId = task.id;
        this.activeTasks.set(task.id, task);

        task.timeoutHandle = setTimeout(
            () => this.handleTaskTimeout(task.id),
            EXECUTION_TIMEOUT_MS
        );

        instance.worker.postMessage({
            type: 'execute',
            id: task.id,
            code: task.code,
            state: task.state
        });
    }

    private createTaskId(): string {

        if (typeof crypto !== 'undefined' && crypto.randomUUID) {
            return crypto.randomUUID();
        }

        return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
    }

    execute(code: string, state: InterpreterSnapshot): Promise<ExecuteResult> {
        this.ensureWorkers();
        return new Promise((resolve, reject) => {
            const shared = { settled: false };
            const wrapResolve = (result: ExecuteResult): void => {
                if (shared.settled) return;
                shared.settled = true;
                resolve(result);
            };
            const wrapReject = (error: Error): void => {
                if (shared.settled) return;
                shared.settled = true;
                reject(error);
            };

            // One execution path: a run is one task on one worker.
            this.taskQueue.push({
                id: this.createTaskId(),
                code,
                state,
                resolve: wrapResolve,
                reject: wrapReject,
                timeoutHandle: null
            });
            this.processQueue();
        });
    }

    abortAll(): void {
        console.log('[WorkerManager] Aborting all tasks...');


        const abortError = new Error('Execution aborted');
        for (const task of this.taskQueue) {
            task.reject(abortError);
        }
        this.taskQueue = [];


        for (const id of this.activeTasks.keys()) {
            const worker = this.workers.find(w => w.currentTaskId === id)?.worker;
            if (worker) {
                worker.postMessage({ type: 'abort', id });
            }
        }
    }

    async resetAllWorkers(): Promise<void> {
        this.abortAll();
        this.workers.forEach(w => w.worker.terminate());
        await this.init();
    }
}

export const WORKER_MANAGER = new WorkerManager();
