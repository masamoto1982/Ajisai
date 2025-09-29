// js/workers/parallel-executor.ts - 並列実行制御

import { WORKER_MANAGER } from './worker-manager';

interface ParallelTask {
    id: string;
    code: string;
    promise: Promise<any>;
    startTime: number;
}

export class ParallelExecutor {
    private activeTasks = new Map<string, ParallelTask>();
    private taskCounter = 0;

    async executeParallel(codes: string[]): Promise<any[]> {
        console.log(`[ParallelExecutor] Starting ${codes.length} parallel executions`);
        
        const tasks: ParallelTask[] = codes.map((code, index) => {
            const id = `parallel_${++this.taskCounter}_${index}`;
            const promise = WORKER_MANAGER.execute(code);
            const task: ParallelTask = {
                id,
                code,
                promise,
                startTime: Date.now()
            };
            
            this.activeTasks.set(id, task);
            console.log(`[ParallelExecutor] Created task ${id}: ${code.substring(0, 30)}...`);
            
            return task;
        });

        try {
            // Wait for all tasks to complete
            const results = await Promise.allSettled(tasks.map(task => task.promise));
            
            console.log(`[ParallelExecutor] All ${tasks.length} tasks completed`);
            
            // Clean up completed tasks
            for (const task of tasks) {
                this.activeTasks.delete(task.id);
                const duration = Date.now() - task.startTime;
                console.log(`[ParallelExecutor] Task ${task.id} completed in ${duration}ms`);
            }
            
            return results.map((result, index) => {
                if (result.status === 'fulfilled') {
                    return result.value;
                } else {
                    console.error(`[ParallelExecutor] Task ${index} failed:`, result.reason);
                    throw result.reason;
                }
            });
            
        } catch (error) {
            console.error('[ParallelExecutor] Parallel execution failed:', error);
            
            // Clean up failed tasks
            for (const task of tasks) {
                this.activeTasks.delete(task.id);
            }
            
            throw error;
        }
    }

    async executeSequential(codes: string[]): Promise<any[]> {
        console.log(`[ParallelExecutor] Starting ${codes.length} sequential executions`);
        
        const results = [];
        
        for (let i = 0; i < codes.length; i++) {
            const code = codes[i];
            if (!code) {
                console.warn(`[ParallelExecutor] Skipping undefined code at index ${i}`);
                continue;
            }
            
            const id = `sequential_${++this.taskCounter}_${i}`;
            
            console.log(`[ParallelExecutor] Executing task ${i + 1}/${codes.length}: ${id}`);
            
            try {
                const result = await WORKER_MANAGER.execute(code);
                results.push(result);
                console.log(`[ParallelExecutor] Task ${id} completed successfully`);
            } catch (error) {
                console.error(`[ParallelExecutor] Task ${id} failed:`, error);
                throw error;
            }
        }
        
        console.log(`[ParallelExecutor] All ${codes.length} sequential tasks completed`);
        return results;
    }

    abortAll(): void {
        console.log(`[ParallelExecutor] Aborting ${this.activeTasks.size} active tasks`);
        WORKER_MANAGER.abortAll();
        this.activeTasks.clear();
    }

    getStatus(): { activeTasks: number } {
        return {
            activeTasks: this.activeTasks.size
        };
    }
}

export const PARALLEL_EXECUTOR = new ParallelExecutor();
