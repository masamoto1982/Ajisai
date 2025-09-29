// js/workers/ajisai-worker.ts - AjisaiWorker実装

import type { WasmModule, AjisaiInterpreter } from '../wasm-types';

interface WorkerMessage {
    type: 'execute' | 'step' | 'abort' | 'init';
    id: string;
    code?: string;
    payload?: any;
}

interface WorkerResponse {
    type: 'result' | 'progress' | 'error' | 'debug' | 'aborted';
    id: string;
    data?: any;
    progress?: { current: number; total: number };
}

class AjisaiWorkerInstance {
    private interpreter: AjisaiInterpreter | null = null;
    private isAborted = false;
    private currentExecutionId: string | null = null;

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
                
            case 'execute':
                await this.executeCode(message.id, message.code || '');
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
