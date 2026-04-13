

import type { ExecuteResult } from '../wasm-interpreter-types';
import type { InterpreterSnapshot } from './interpreter-snapshot';
import { extractCompiledWasmModule } from '../wasm-module-loader';

interface WorkerTask {
    id: string;
    code: string;
    state: InterpreterSnapshot;
    hedgedRequestId: string;
    resolve: (result: any) => void;
    reject: (error: any) => void;
}

interface WorkerInstance {
    worker: Worker;
    busy: boolean;
    currentTaskId: string | null;
}

const MOBILE_BREAKPOINT = 768;
const MAX_MOBILE_WORKERS = 2;

export class WorkerManager {
    private workers: WorkerInstance[] = [];
    private taskQueue: WorkerTask[] = [];
    private activeTasks = new Map<string, WorkerTask>();
    private compiledModule: WebAssembly.Module | null = null;
    private maxWorkers = window.innerWidth <= MOBILE_BREAKPOINT
        ? Math.min(navigator.hardwareConcurrency || 2, MAX_MOBILE_WORKERS)
        : navigator.hardwareConcurrency || 4;

    async init(): Promise<void> {
        console.log('[WorkerManager] Initializing worker pool...');
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
            this.activeTasks.delete(instance.currentTaskId);
        }
        instance.busy = false;
        instance.currentTaskId = null;
        this.processQueue();
    }

    private processQueue(): void {
        const availableWorker = this.workers.find(w => !w.busy);
        const nextTask = this.taskQueue.shift();
        if (availableWorker && nextTask) {
            this.assignTaskToWorker(availableWorker, nextTask);
        }
    }

    private assignTaskToWorker(instance: WorkerInstance, task: WorkerTask): void {
        instance.busy = true;
        instance.currentTaskId = task.id;
        this.activeTasks.set(task.id, task);

        instance.worker.postMessage({
            type: 'execute',
            id: task.id,
            code: task.code,
            state: task.state,
            executionMode: task.state.executionMode,
            hedgedRequestId: task.hedgedRequestId
        });
    }

    private createTaskId(): string {

        if (typeof crypto !== 'undefined' && crypto.randomUUID) {
            return crypto.randomUUID();
        }

        return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
    }

    private createHedgedRequestId(): string {
        return `hedged-${this.createTaskId()}`;
    }

    execute(code: string, state: InterpreterSnapshot): Promise<ExecuteResult> {
        this.ensureWorkers();
        return new Promise((resolve, reject) => {
            const task: WorkerTask = {
                id: this.createTaskId(),
                code,
                state,
                hedgedRequestId: this.createHedgedRequestId(),
                resolve,
                reject
            };
            this.taskQueue.push(task);
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
