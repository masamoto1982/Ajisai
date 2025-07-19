export class Stepper {
    constructor() {
        this.active = false;
        this.callback = null;
    }

    init() {
        this.active = false;
    }

    isActive() {
        return this.active;
    }

    async start(code, callback) {
        if (!window.ajisaiInterpreter) {
            throw new Error('WASM not loaded');
        }
        
        this.callback = callback;
        
        try {
            const result = window.ajisaiInterpreter.init_step(code);
            if (result === 'OK') {
                this.active = true;
                window.dispatchEvent(new CustomEvent('step-mode-changed', {
                    detail: { active: true }
                }));
                this.step();
            } else {
                this.callback({ output: result, hasMore: false });
            }
        } catch (error) {
            this.reset();
            throw error;
        }
    }

    async step() {
        if (!this.active) return;
        
        try {
            const stepResult = window.ajisaiInterpreter.step();
            this.callback(stepResult);
            
            if (!stepResult.hasMore) {
                this.reset();
            }
        } catch (error) {
            this.reset();
            throw error;
        }
    }

    reset() {
        this.active = false;
        window.dispatchEvent(new CustomEvent('step-mode-changed', {
            detail: { active: false }
        }));
    }
}
