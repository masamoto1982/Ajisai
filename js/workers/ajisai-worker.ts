// js/workers/ajisai-worker.ts - AjisaiWorker実装

import type { WasmModule, AjisaiInterpreter } from '../wasm-types';

interface WorkerMessage {
    type: 'execute' | 'step' | 'abort' | 'init' | 'progressive' | 'progressive_step' | 'sync_words' | 'sync_stack';
    id: string;
    code?: string;
    payload?: any;
    customWords?: any[];
    stack?: any[];
}

interface WorkerResponse {
    type: 'result' | 'progress' | 'error' | 'debug' | 'aborted' | 'progressive_init' | 'progressive_step';
    id: string;
    data?: any;
    progress?: { current: number; total: number };
}

class AjisaiWorkerInstance {
    private interpreter: AjisaiInterpreter | null = null;
    private isAborted = false;
    private currentExecutionId: string | null = null;
    private progressiveExecution: {
        id: string;
        delayMs: number;
        totalIterations: number;
        currentIteration: number;
    } | null = null;

    async init(): Promise<void> {
        console.log('[Worker] Initializing WASM...');
        try {
            // Import WASM module in worker context
            const wasmModule = await import('../pkg/ajisai_core.js') as unknown as WasmModule;
            
            if (wasmModule.default) {
                await wasmModule.default();
            } else if (wasmModule.init) {
                await wasmModule.init();
            }
            
            this.interpreter = new wasmModule.AjisaiInterpreter();
            console.log('[Worker] WASM initialized successfully');
            
            this.postMessage({
                type: 'debug',
                id: 'init',
                data: 'Worker initialized successfully'
            });
        } catch (error) {
            console.error('[Worker] Failed to initialize WASM:', error);
            this.postMessage({
                type: 'error',
                id: 'init',
                data: `Worker initialization failed: ${error}`
            });
        }
    }

    private postMessage(response: WorkerResponse): void {
        self.postMessage(response);
    }

    async handleMessage(message: WorkerMessage): Promise<void> {
        console.log(`[Worker] Received message: ${message.type}, ID: ${message.id}`);
        
        switch (message.type) {
            case 'init':
                await this.init();
                break;
                
            case 'sync_words':
                await this.syncCustomWords(message.id, message.customWords || []);
                break;
                
            case 'sync_stack':
                await this.syncStack(message.id, message.stack || []);
                break;
                
            case 'execute':
                await this.executeCode(message.id, message.code || '');
                break;
                
            case 'progressive':
                await this.initProgressiveExecution(message.id, message.code || '');
                break;
                
            case 'progressive_step':
                await this.executeProgressiveStep(message.id);
                break;
                
            case 'step':
                await this.stepCode(message.id, message.code || '');
                break;
                
            case 'abort':
                this.abortExecution(message.id);
                break;
                
            default:
                console.warn(`[Worker] Unknown message type: ${message.type}`);
        }
    }

    private async syncCustomWords(id: string, customWords: any[]): Promise<void> {
        if (!this.interpreter) {
            this.postMessage({
                type: 'error',
                id,
                data: 'Interpreter not initialized'
            });
            return;
        }
        
        try {
            console.log(`[Worker] Syncing ${customWords?.length || 0} custom words`);
            
            if (customWords && customWords.length > 0) {
                // カスタムワードを復元
                await this.interpreter.restore_custom_words(customWords);
            }
            
            this.postMessage({
                type: 'result',
                id,
                data: { status: 'OK', synced: customWords?.length || 0 }
            });
        } catch (error) {
            console.error(`[Worker] Failed to sync custom words:`, error);
            this.postMessage({
                type: 'error',
                id,
                data: `Failed to sync custom words: ${error}`
            });
        }
    }

    private async syncStack(id: string, stack: any[]): Promise<void> {
        if (!this.interpreter) {
            this.postMessage({
                type: 'error',
                id,
                data: 'Interpreter not initialized'
            });
            return;
        }
        
        try {
            console.log(`[Worker] Syncing stack with ${stack?.length || 0} items`);
            
            if (stack && stack.length > 0) {
                await this.interpreter.restore_stack(stack);
            } else {
                // 空のスタックをセット（リセット）
                await this.interpreter.restore_stack([]);
            }
            
            this.postMessage({
                type: 'result',
                id,
                data: { status: 'OK', synced: stack?.length || 0 }
            });
        } catch (error) {
            console.error(`[Worker] Failed to sync stack:`, error);
            this.postMessage({
                type: 'error',
                id,
                data: `Failed to sync stack: ${error}`
            });
        }
    }

    private async executeCode(id: string, code: string): Promise<void> {
        if (!this.interpreter) {
            this.postMessage({
                type: 'error',
                id,
                data: 'Interpreter not initialized'
            });
            return;
        }

        this.currentExecutionId = id;
        this.isAborted = false;
        
        console.log(`[Worker] Executing code for ID: ${id}`);
        this.postMessage({
            type: 'debug',
            id,
            data: `Starting execution: ${code.substring(0, 50)}...`
        });

        try {
            // Check for abort before execution
            if (this.isAborted) {
                this.postMessage({ type: 'aborted', id });
                return;
            }

            const result = await this.interpreter.execute(code);
            
            if (this.isAborted) {
                this.postMessage({ type: 'aborted', id });
                return;
            }

            console.log(`[Worker] Execution completed for ID: ${id}`);
            this.postMessage({
                type: 'result',
                id,
                data: result
            });
            
        } catch (error) {
            console.error(`[Worker] Execution error for ID: ${id}:`, error);
            this.postMessage({
                type: 'error',
                id,
                data: `Execution error: ${error}`
            });
        } finally {
            this.currentExecutionId = null;
        }
    }

    private async initProgressiveExecution(id: string, code: string): Promise<void> {
        if (!this.interpreter) {
            this.postMessage({
                type: 'error',
                id,
                data: 'Interpreter not initialized'
            });
            return;
        }

        this.currentExecutionId = id;
        this.isAborted = false;
        
        console.log(`[Worker] Initializing progressive execution for ID: ${id}`);

        try {
            if (this.isAborted) {
                this.postMessage({ type: 'aborted', id });
                return;
            }

            const result = await this.interpreter.init_progressive_execution(code);
            
            if (this.isAborted) {
                this.postMessage({ type: 'aborted', id });
                return;
            }

            // Check if it's actually progressive
            if (result.status === 'PROGRESSIVE') {
                this.progressiveExecution = {
                    id,
                    delayMs: result.delayMs || 0,
                    totalIterations: result.totalIterations || 1,
                    currentIteration: 0
                };
                
                console.log(`[Worker] Progressive execution initialized: ${this.progressiveExecution.totalIterations} iterations with ${this.progressiveExecution.delayMs}ms delay`);
                
                this.postMessage({
                    type: 'progressive_init',
                    id,
                    data: result
                });
            } else {
                // Not progressive, execute normally
                this.postMessage({
                    type: 'result',
                    id,
                    data: result
                });
            }
            
        } catch (error) {
            console.error(`[Worker] Progressive init error for ID: ${id}:`, error);
            this.postMessage({
                type: 'error',
                id,
                data: `Progressive init error: ${error}`
            });
        } finally {
            if (!this.progressiveExecution) {
                this.currentExecutionId = null;
            }
        }
    }

    private async executeProgressiveStep(id: string): Promise<void> {
        if (!this.interpreter || !this.progressiveExecution || this.progressiveExecution.id !== id) {
            this.postMessage({
                type: 'error',
                id,
                data: 'Progressive execution not initialized or ID mismatch'
            });
            return;
        }

        try {
            if (this.isAborted) {
                this.postMessage({ type: 'aborted', id });
                this.progressiveExecution = null;
                this.currentExecutionId = null;
                return;
            }

            const result = await this.interpreter.execute_progressive_step();
            
            if (this.isAborted) {
                this.postMessage({ type: 'aborted', id });
                this.progressiveExecution = null;
                this.currentExecutionId = null;
                return;
            }

            this.progressiveExecution.currentIteration = result.currentIteration || 0;
            
            console.log(`[Worker] Progressive step completed: ${this.progressiveExecution.currentIteration}/${this.progressiveExecution.totalIterations}`);
            
            if (result.status === 'COMPLETED' || !result.hasMore) {
                console.log(`[Worker] Progressive execution completed for ID: ${id}`);
                this.progressiveExecution = null;
                this.currentExecutionId = null;
            }
            
            this.postMessage({
                type: 'progressive_step',
                id,
                data: result
            });
            
        } catch (error) {
            console.error(`[Worker] Progressive step error for ID: ${id}:`, error);
            this.progressiveExecution = null;
            this.currentExecutionId = null;
            this.postMessage({
                type: 'error',
                id,
                data: `Progressive step error: ${error}`
            });
        }
    }

    private async stepCode(id: string, code: string): Promise<void> {
        if (!this.interpreter) {
            this.postMessage({
                type: 'error',
                id,
                data: 'Interpreter not initialized'
            });
            return;
        }

        this.currentExecutionId = id;
        this.isAborted = false;
        
        console.log(`[Worker] Step execution for ID: ${id}`);
        
        try {
            if (this.isAborted) {
                this.postMessage({ type: 'aborted', id });
                return;
            }

            const result = this.interpreter.execute_step(code);
            
            if (this.isAborted) {
                this.postMessage({ type: 'aborted', id });
                return;
            }

            this.postMessage({
                type: 'result',
                id,
                data: result
            });
            
        } catch (error) {
            console.error(`[Worker] Step execution error for ID: ${id}:`, error);
            this.postMessage({
                type: 'error',
                id,
                data: `Step execution error: ${error}`
            });
        } finally {
            this.currentExecutionId = null;
        }
    }

    private abortExecution(id: string): void {
        console.log(`[Worker] Abort requested for ID: ${id}`);
        this.isAborted = true;
        
        if (this.currentExecutionId === id || id === '*') {
            console.log(`[Worker] Aborting execution for ID: ${this.currentExecutionId || 'any'}`);
            
            // Clean up progressive execution
            if (this.progressiveExecution) {
                console.log(`[Worker] Cleaning up progressive execution`);
                this.progressiveExecution = null;
            }
            
            this.postMessage({
                type: 'aborted',
                id: this.currentExecutionId || id
            });
            this.currentExecutionId = null;
        }
    }
}

// Worker instance
const workerInstance = new AjisaiWorkerInstance();

// Message handler
self.addEventListener('message', async (event: MessageEvent<WorkerMessage>) => {
    try {
        await workerInstance.handleMessage(event.data);
    } catch (error) {
        console.error('[Worker] Error handling message:', error);
        self.postMessage({
            type: 'error',
            id: event.data.id,
            data: `Worker error: ${error}`
        });
    }
});

console.log('[Worker] Ajisai Worker loaded');
