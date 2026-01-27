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
        this.completeTask(instance);
    }
    
    private handleWorkerError(instance: WorkerInstance, error: ErrorEvent): void {
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
            state: task.state
        });
    }

    private generateTaskId(): string {
        // crypto.randomUUID()が利用可能な場合はそれを使用、そうでなければフォールバック
        if (typeof crypto !== 'undefined' && crypto.randomUUID) {
            return crypto.randomUUID();
        }
        // フォールバック: タイムスタンプ + ランダム値
        return `${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
    }

    execute(code: string, state: { stack: Value[], customWords: CustomWord[] }): Promise<ExecuteResult> {
        return new Promise((resolve, reject) => {
            const task: WorkerTask = {
                id: this.generateTaskId(),
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

        // キュー内のタスクのPromiseをrejectしてからクリア
        const abortError = new Error('Execution aborted');
        for (const task of this.taskQueue) {
            task.reject(abortError);
        }
        this.taskQueue = [];

        // 実行中のタスクにabortメッセージを送信
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
