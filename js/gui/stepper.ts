// js/gui/stepper.ts

import type { AjisaiInterpreter, StepResult } from '../wasm-types';

export class Stepper {
    private getInterpreter!: () => AjisaiInterpreter;

    init(getInterpreter: () => AjisaiInterpreter): void {
        this.getInterpreter = getInterpreter;
    }

    async start(code: string): Promise<{ ok: boolean; error?: string }> {
        try {
            const result = this.getInterpreter().init_step(code);
            if (result === 'OK') {
                return { ok: true };
            } else {
                return { ok: false, error: result };
            }
        } catch (error) {
            return { ok: false, error: String(error) };
        }
    }

    async step(): Promise<StepResult> {
        try {
            const result = this.getInterpreter().step();
            return result;
        } catch (error) {
            this.reset();
            throw error;
        }
    }

    reset(): void {
        // 現在は状態を持たないためリセット処理は不要だが、将来のために残す
    }
}
