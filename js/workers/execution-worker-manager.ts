

import type { ExecuteResult } from '../wasm-interpreter-types';
import type { InterpreterSnapshot } from './interpreter-snapshot';
import { extractCompiledWasmModule } from '../wasm-module-loader';

interface WorkerTask {
    id: string;
    code: string;
    state: InterpreterSnapshot;
    hedgedRequestId: string;
    strategyLabel: string;
    resolve: (result: any) => void;
    reject: (error: any) => void;
}

interface HedgedGroup {
    winnerTaskId: string | null;
    taskIds: Set<string>;
    cancelledStrategies: string[];
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
    private hedgedGroups = new Map<string, HedgedGroup>();
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
            case 'result': {
                const handled = this.resolveHedgedWinner(task, message.data);
                if (!handled) {
                    task.resolve(message.data);
                }
                break;
            }
            case 'error':
                if (!this.resolveHedgedError(task, message.data)) {
                    task.reject(new Error(message.data));
                }
                break;
            case 'aborted':
                if (!this.isLoserTask(task)) {
                    task.reject(new Error('Execution aborted'));
                }
                break;
        }
        this.completeTask(instance);
    }

    private getOrCreateHedgedGroup(hedgedRequestId: string): HedgedGroup {
        let group = this.hedgedGroups.get(hedgedRequestId);
        if (!group) {
            group = {
                winnerTaskId: null,
                taskIds: new Set<string>(),
                cancelledStrategies: []
            };
            this.hedgedGroups.set(hedgedRequestId, group);
        }
        return group;
    }

    private isLoserTask(task: WorkerTask): boolean {
        const group = this.hedgedGroups.get(task.hedgedRequestId);
        return !!group && !!group.winnerTaskId && group.winnerTaskId !== task.id;
    }

    private resolveHedgedWinner(task: WorkerTask, data: ExecuteResult): boolean {
        const group = this.hedgedGroups.get(task.hedgedRequestId);
        if (!group || group.taskIds.size <= 1) return false;

        if (!group.winnerTaskId) {
            group.winnerTaskId = task.id;
            const cancelled = this.abortLosers(task);
            const enriched: ExecuteResult = {
                ...data,
                hedgedWinner: task.strategyLabel,
                hedgedCancelled: cancelled,
                hedgedFallbackReason: cancelled.length > 0 ? 'LoserDiscarded' : undefined
            };
            task.resolve(enriched);
            return true;
        }
        return true;
    }

    private resolveHedgedError(task: WorkerTask, errorText: string): boolean {
        const group = this.hedgedGroups.get(task.hedgedRequestId);
        if (!group || group.taskIds.size <= 1) return false;
        group.taskIds.delete(task.id);
        if (group.taskIds.size === 0) {
            this.hedgedGroups.delete(task.hedgedRequestId);
            task.reject(new Error(errorText));
        }
        return true;
    }

    private abortLosers(winnerTask: WorkerTask): string[] {
        const group = this.hedgedGroups.get(winnerTask.hedgedRequestId);
        if (!group) return [];
        const cancelled: string[] = [];

        for (const [taskId, task] of this.activeTasks.entries()) {
            if (task.hedgedRequestId === winnerTask.hedgedRequestId && taskId !== winnerTask.id) {
                const worker = this.workers.find(w => w.currentTaskId === taskId)?.worker;
                if (worker) {
                    worker.postMessage({ type: 'abort', id: taskId });
                    cancelled.push(task.strategyLabel);
                }
            }
        }

        this.taskQueue = this.taskQueue.filter(task => {
            if (task.hedgedRequestId === winnerTask.hedgedRequestId && task.id !== winnerTask.id) {
                cancelled.push(task.strategyLabel);
                return false;
            }
            return true;
        });

        group.cancelledStrategies = cancelled;
        return cancelled;
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
                const group = this.hedgedGroups.get(task.hedgedRequestId);
                if (group) {
                    group.taskIds.delete(task.id);
                    if (group.taskIds.size === 0) {
                        this.hedgedGroups.delete(task.hedgedRequestId);
                    }
                }
            }
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
        this.getOrCreateHedgedGroup(task.hedgedRequestId).taskIds.add(task.id);

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

            const hedgedRequestId = this.createHedgedRequestId();
            if (state.executionMode === 'hedged-trace' && this.maxWorkers >= 2) {
                const hedgedSafeState: InterpreterSnapshot = { ...state, executionMode: 'hedged-safe' };
                const greedyState: InterpreterSnapshot = { ...state, executionMode: 'greedy' };
                this.taskQueue.push({
                    id: this.createTaskId(),
                    code,
                    state: hedgedSafeState,
                    hedgedRequestId,
                    strategyLabel: 'hedged-safe',
                    resolve: wrapResolve,
                    reject: wrapReject
                });
                this.taskQueue.push({
                    id: this.createTaskId(),
                    code,
                    state: greedyState,
                    hedgedRequestId,
                    strategyLabel: 'plain-greedy',
                    resolve: wrapResolve,
                    reject: wrapReject
                });
            } else {
                this.taskQueue.push({
                    id: this.createTaskId(),
                    code,
                    state,
                    hedgedRequestId,
                    strategyLabel: state.executionMode,
                    resolve: wrapResolve,
                    reject: wrapReject
                });
            }
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
