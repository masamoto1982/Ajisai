// js/workers/worker-manager.ts

import type { ExecuteResult, Value, CustomWord } from '../wasm-types';

interface WorkerTask {
    id: string;
    code: string;
    state: {
        stack: Value[];
        customWords: CustomWord[];
    };
    resolve: (result: any) => void;
    reject: (error: any) => void;
}

interface WorkerInstance {
    worker: Worker;
    busy: boolean;
    currentTaskId: string | null;
}

export class WorkerManager {
    private workers: WorkerInstance[] = [];
    private taskQueue: WorkerTask[] = [];
    private activeTasks = new Map<string, WorkerTask>();
    private maxWorkers = navigator.hardwareConcurrency || 4;
    private taskIdCounter = 0;

    constructor() {
        this.setupGlobalAbortHandler();
    }

    async init(): Promise<void> {
        console.log('[WorkerManager] Initializing worker pool...');
        this.workers = [];
        for (let i = 0; i < this.maxWorkers; i++) {
            this.createWorker();
        }
    }

    private createWorker(): void {
        const worker = new Worker(new URL('./ajisai-worker.ts', import.meta.url), { type: 'module' });
        const instance: WorkerInstance = { worker, busy: false, currentTaskId: null };

        worker.onmessage = (event) => this.handleWorkerMessage(instance, event.data);
        worker.onerror = (error) => this.handleWorkerError(instance, error);
        
        this.workers.push(instance);
    }
    
    private handleWorkerMessage(instance: WorkerInstance, message: any): void {
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
        this.completeTask(instance, task);
    }
    
    private handleWorkerError(instance: WorkerInstance, error: ErrorEvent): void {
        console.error('[WorkerManager] Worker error:', error.message);
        if (instance.currentTaskId) {
            const task = this.activeTasks.get(instance.currentTaskId);
            task?.reject(new Error(`Worker error: ${error.message}`));
            this.activeTasks.delete(instance.currentTaskId);
        }
        // ワーカーを再作成
        const index = this.workers.indexOf(instance);
        if (index > -1) this.workers.splice(index, 1);
        this.createWorker();
        this.processQueue();
    }
    
    private completeTask(instance: WorkerInstance, task: WorkerTask): void {
        instance.busy = false;
        instance.currentTaskId = null;
        this.activeTasks.delete(task.id);
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
            state: task.state
        });
    }

    execute(code: string, state: { stack: Value[], customWords: CustomWord[] }): Promise<ExecuteResult> {
        return new Promise((resolve, reject) => {
            const task: WorkerTask = {
                id: `task_${++this.taskIdCounter}`,
                code,
                state,
                resolve,
                reject
            };
            this.taskQueue.push(task);
            this.processQueue();
        });
    }

    abortAll(): void {
        console.log('[WorkerManager] Aborting all tasks...');
        this.taskQueue = [];
        for (const [id, task] of this.activeTasks.entries()) {
            const worker = this.workers.find(w => w.currentTaskId === id)?.worker;
            worker?.postMessage({ type: 'abort', id });
        }
    }

    async resetAllWorkers(): Promise<void> {
        this.abortAll();
        // ワーカーをすべて終了させて再作成
        this.workers.forEach(w => w.worker.terminate());
        await this.init();
    }

    private setupGlobalAbortHandler(): void {
        window.addEventListener('keydown', (event) => {
            if (event.key === 'Escape') {
                this.abortAll();
            }
        });
    }
}

export const WORKER_MANAGER = new WorkerManager();
