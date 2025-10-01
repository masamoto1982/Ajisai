// js/workers/worker-manager.ts - Worker管理とプール

interface WorkerTask {
    id: string;
    code: string;
    type: 'execute' | 'step' | 'progressive';
    resolve: (result: any) => void;
    reject: (error: any) => void;
    worker?: Worker;
    isProgressive?: boolean;
    progressiveState?: {
        delayMs: number;
        totalIterations: number;
        currentIteration: number;
    };
    onProgress?: (result: any) => void;
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
    private maxWorkers = 4;
    private taskIdCounter = 0;

    constructor() {
        this.setupGlobalAbortHandler();
    }

    async init(): Promise<void> {
        console.log('[WorkerManager] Initializing worker pool...');
        
        try {
            // Create initial worker pool
            for (let i = 0; i < this.maxWorkers; i++) {
                await this.createWorker();
            }
            console.log(`[WorkerManager] Created ${this.workers.length} workers`);
        } catch (error) {
            console.error('[WorkerManager] Failed to initialize workers:', error);
            throw error;
        }
    }

    private async createWorker(): Promise<WorkerInstance> {
        console.log('[WorkerManager] Creating new worker...');
        
        const worker = new Worker(
            new URL('./ajisai-worker.ts', import.meta.url),
            { type: 'module' }
        );

        const workerInstance: WorkerInstance = {
            worker,
            busy: false,
            currentTaskId: null
        };

        // Setup worker message handler
        worker.addEventListener('message', (event) => {
            this.handleWorkerMessage(workerInstance, event.data);
        });

        worker.addEventListener('error', (error) => {
            console.error('[WorkerManager] Worker error:', error);
            this.handleWorkerError(workerInstance, error);
        });

        // Initialize worker
        await this.initWorker(worker);
        
        this.workers.push(workerInstance);
        console.log(`[WorkerManager] Worker created. Total workers: ${this.workers.length}`);
        
        return workerInstance;
    }

    private initWorker(worker: Worker): Promise<void> {
        return new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                reject(new Error('Worker initialization timeout'));
            }, 10000);

            const handleMessage = (event: MessageEvent) => {
                if (event.data.type === 'debug' && event.data.id === 'init') {
                    clearTimeout(timeout);
                    worker.removeEventListener('message', handleMessage);
                    resolve();
                } else if (event.data.type === 'error' && event.data.id === 'init') {
                    clearTimeout(timeout);
                    worker.removeEventListener('message', handleMessage);
                    reject(new Error(event.data.data));
                }
            };

            worker.addEventListener('message', handleMessage);
            worker.postMessage({ type: 'init', id: 'init' });
        });
    }

    private handleWorkerMessage(workerInstance: WorkerInstance, message: any): void {
        // 初期化メッセージはタスクではないため、ここで無視して警告を抑制する
        if (message.id === 'init' || message.id?.startsWith('sync_')) {
            return;
        }
        
        console.log(`[WorkerManager] Worker message: ${message.type}, ID: ${message.id}`);

        const task = this.activeTasks.get(message.id);
        if (!task) {
            console.warn(`[WorkerManager] No task found for ID: ${message.id}`);
            return;
        }

        switch (message.type) {
            case 'result':
                console.log(`[WorkerManager] Task ${message.id} completed successfully`);
                task.resolve(message.data);
                this.completeTask(workerInstance, task);
                break;

            case 'progressive_init':
                console.log(`[WorkerManager] Progressive execution initialized for ${message.id}`);
                task.isProgressive = true;
                task.progressiveState = {
                    delayMs: message.data.delayMs || 0,
                    totalIterations: message.data.totalIterations || 1,
                    currentIteration: 0
                };
                this.startProgressiveExecution(workerInstance, task);
                break;

            case 'progressive_step':
                console.log(`[WorkerManager] Progressive step for ${message.id}:`, message.data);
                if (task.progressiveState) {
                    task.progressiveState.currentIteration = message.data.currentIteration || 0;
                }
                
                // プログレスコールバックを呼び出してGUIに即座に通知
                if (task.onProgress) {
                    console.log(`[WorkerManager] Calling progress callback for ${message.id}`);
                    task.onProgress(message.data);
                }
                
                if (message.data.status === 'COMPLETED' || !message.data.hasMore) {
                    console.log(`[WorkerManager] Progressive execution completed for ${message.id}`);
                    task.resolve(message.data);
                    this.completeTask(workerInstance, task);
                } else {
                    // Continue progressive execution
                    this.scheduleNextProgressiveStep(workerInstance, task);
                }
                break;

            case 'error':
                console.error(`[WorkerManager] Task ${message.id} failed:`, message.data);
                task.reject(new Error(message.data));
                this.completeTask(workerInstance, task);
                break;

            case 'aborted':
                console.log(`[WorkerManager] Task ${message.id} aborted`);
                task.reject(new Error('Execution aborted'));
                this.completeTask(workerInstance, task);
                break;

            case 'progress':
                // Handle progress updates if needed
                console.log(`[WorkerManager] Progress for ${message.id}:`, message.progress);
                break;

            case 'debug':
                console.log(`[WorkerManager] Debug for ${message.id}:`, message.data);
                break;

            default:
                console.warn(`[WorkerManager] Unknown message type: ${message.type}`);
        }
    }

    private startProgressiveExecution(workerInstance: WorkerInstance, task: WorkerTask): void {
        console.log(`[WorkerManager] Starting progressive execution for ${task.id}`);
        this.scheduleNextProgressiveStep(workerInstance, task);
    }

    private scheduleNextProgressiveStep(workerInstance: WorkerInstance, task: WorkerTask): void {
        if (!task.progressiveState) {
            console.error(`[WorkerManager] No progressive state for task ${task.id}`);
            return;
        }

        const delayMs = task.progressiveState.delayMs;
        
        console.log(`[WorkerManager] Scheduling next step for ${task.id} in ${delayMs}ms`);
        
        setTimeout(() => {
            if (this.activeTasks.has(task.id)) {
                console.log(`[WorkerManager] Executing progressive step for ${task.id}`);
                workerInstance.worker.postMessage({
                    type: 'progressive_step',
                    id: task.id
                });
            } else {
                console.log(`[WorkerManager] Task ${task.id} no longer active, skipping step`);
            }
        }, delayMs);
    }

    private handleWorkerError(workerInstance: WorkerInstance, error: ErrorEvent): void {
        console.error('[WorkerManager] Worker error:', error);
        
        // Mark worker as failed and reject current task
        if (workerInstance.currentTaskId) {
            const task = this.activeTasks.get(workerInstance.currentTaskId);
            if (task) {
                task.reject(new Error(`Worker error: ${error.message}`));
                this.activeTasks.delete(workerInstance.currentTaskId);
            }
        }

        // Remove failed worker
        const index = this.workers.indexOf(workerInstance);
        if (index !== -1) {
            this.workers.splice(index, 1);
        }

        // Create replacement worker
        this.createWorker().catch(console.error);
    }

    private completeTask(workerInstance: WorkerInstance, task: WorkerTask): void {
        workerInstance.busy = false;
        workerInstance.currentTaskId = null;
        this.activeTasks.delete(task.id);
        
        // Process next task in queue
        this.processQueue();
    }

    private processQueue(): void {
        const availableWorker = this.workers.find(w => !w.busy);
        const nextTask = this.taskQueue.shift();
        
        if (availableWorker && nextTask) {
            this.executeTask(availableWorker, nextTask);
        }
    }

    private executeTask(workerInstance: WorkerInstance, task: WorkerTask): void {
        console.log(`[WorkerManager] Executing task ${task.id} on worker`);
        
        workerInstance.busy = true;
        workerInstance.currentTaskId = task.id;
        task.worker = workerInstance.worker;
        
        this.activeTasks.set(task.id, task);
        
        let messageType = task.type;
        if (task.type === 'progressive') {
            messageType = 'progressive';
        }
        
        workerInstance.worker.postMessage({
            type: messageType,
            id: task.id,
            code: task.code
        });
    }

    async syncCustomWords(customWords: any[]): Promise<void> {
        console.log(`[WorkerManager] Syncing ${customWords.length} custom words to all workers`);
        
        // すべてのWorkerに同期
        const syncPromises = this.workers.map(workerInstance => {
            return new Promise<void>((resolve, reject) => {
                const syncId = `sync_${++this.taskIdCounter}`;
                
                const handleMessage = (event: MessageEvent) => {
                    if (event.data.id === syncId) {
                        workerInstance.worker.removeEventListener('message', handleMessage);
                        if (event.data.type === 'result') {
                            console.log(`[WorkerManager] Worker synced: ${event.data.data?.synced || 0} words`);
                            resolve();
                        } else {
                            reject(new Error(event.data.data));
                        }
                    }
                };
                
                workerInstance.worker.addEventListener('message', handleMessage);
                
                workerInstance.worker.postMessage({
                    type: 'sync_words',
                    id: syncId,
                    customWords: customWords
                });
                
                // タイムアウト
                setTimeout(() => {
                    workerInstance.worker.removeEventListener('message', handleMessage);
                    reject(new Error('Sync timeout'));
                }, 5000);
            });
        });
        
        try {
            await Promise.all(syncPromises);
            console.log('[WorkerManager] All workers synced');
        } catch (error) {
            console.error('[WorkerManager] Failed to sync some workers:', error);
            throw error;
        }
    }

    async execute(code: string, onProgress?: (result: any) => void): Promise<any> {
        const taskId = `task_${++this.taskIdCounter}`;
        console.log(`[WorkerManager] Queuing execute task: ${taskId}`);
        
        // Check if this should be progressive execution
        const isProgressive = this.shouldUseProgressiveExecution(code);
        const taskType = isProgressive ? 'progressive' : 'execute';
        
        console.log(`[WorkerManager] Task ${taskId} type: ${taskType}`);
        
        return new Promise((resolve, reject) => {
            const task: WorkerTask = {
                id: taskId,
                code,
                type: taskType as 'execute' | 'step' | 'progressive',
                resolve,
                reject,
                onProgress
            };

            const availableWorker = this.workers.find(w => !w.busy);
            if (availableWorker) {
                this.executeTask(availableWorker, task);
            } else {
                console.log(`[WorkerManager] No available workers, queuing task: ${taskId}`);
                this.taskQueue.push(task);
            }
        });
    }

    private shouldUseProgressiveExecution(code: string): boolean {
        // Check for delay and repeat modifiers
        return /\d+x\s+\d+m?s|\d+m?s\s+\d+x/.test(code) || /\d+(ms|s)\s/.test(code);
    }

    async executeStep(code: string): Promise<any> {
        const taskId = `step_${++this.taskIdCounter}`;
        console.log(`[WorkerManager] Queuing step task: ${taskId}`);
        
        return new Promise((resolve, reject) => {
            const task: WorkerTask = {
                id: taskId,
                code,
                type: 'step',
                resolve,
                reject
            };

            const availableWorker = this.workers.find(w => !w.busy);
            if (availableWorker) {
                this.executeTask(availableWorker, task);
            } else {
                console.log(`[WorkerManager] No available workers, queuing step task: ${taskId}`);
                this.taskQueue.push(task);
            }
        });
    }

    abortAll(): void {
        console.log('[WorkerManager] Aborting all tasks...');
        
        // Abort all active tasks
        for (const [taskId, task] of this.activeTasks.entries()) {
            if (task.worker) {
                console.log(`[WorkerManager] Sending abort to task: ${taskId}`);
                task.worker.postMessage({
                    type: 'abort',
                    id: taskId
                });
            }
        }
        
        // Clear task queue
        const queuedTasks = this.taskQueue.splice(0);
        for (const task of queuedTasks) {
            task.reject(new Error('Execution aborted'));
        }
        
        console.log(`[WorkerManager] Aborted ${this.activeTasks.size} active tasks and ${queuedTasks.length} queued tasks`);
    }

    private setupGlobalAbortHandler(): void {
        // Setup escape key handler for global abort
        if (typeof window !== 'undefined') {
            window.addEventListener('keydown', (event) => {
                if (event.key === 'Escape') {
                    console.log('[WorkerManager] Escape key pressed - aborting all tasks');
                    this.abortAll();
                    event.preventDefault();
                    event.stopPropagation();
                }
            }, true); // Use capture phase for priority
        }
    }

    getStatus(): { activeJobs: number; queuedJobs: number; workers: number } {
        return {
            activeJobs: this.activeTasks.size,
            queuedJobs: this.taskQueue.length,
            workers: this.workers.length
        };
    }

    terminate(): void {
        console.log('[WorkerManager] Terminating all workers...');
        
        this.abortAll();
        
        for (const workerInstance of this.workers) {
            workerInstance.worker.terminate();
        }
        
        this.workers.length = 0;
        this.activeTasks.clear();
        this.taskQueue.length = 0;
        
        console.log('[WorkerManager] All workers terminated');
    }
}

export const WORKER_MANAGER = new WorkerManager();
