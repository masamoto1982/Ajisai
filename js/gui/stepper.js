// js/gui/stepper.js

export class Stepper {
    init(getInterpreter) {
        this.getInterpreter = getInterpreter;
    }

    async start(code) {
        try {
            const result = this.getInterpreter().init_step(code);
            if (result === 'OK') {
                return { ok: true };
            } else {
                return { ok: false, error: result };
            }
        } catch (error) {
            return { ok: false, error };
        }
    }

    async step() {
        try {
            const result = this.getInterpreter().step();
            return result; // { hasMore, output, position, total }
        } catch (error) {
            this.reset();
            throw error;
        }
    }

    reset() {
        // 現在は状態を持たないためリセット処理は不要だが、将来のために残す
    }
}
