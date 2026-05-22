import type { SerialAdapter, SerialPortInfo } from '../platform-adapter';

/**
 * Tauri serial backend — typed stub (Phase 3).
 *
 * The contract is fixed now so the rest of the system compiles against a single
 * `SerialAdapter` interface. The implementation will route to native `serial_*`
 * Tauri commands backed by the `serialport` crate in `src-tauri`. Until then the
 * adapter reports itself unavailable and the outbound methods throw.
 *
 * See `docs/dev/web-serial-module-design.md`.
 */
export class TauriSerialAdapter implements SerialAdapter {
    readonly available = false;

    private notImplemented(): never {
        throw new Error('SERIAL is not yet implemented in the Tauri backend (Phase 3).');
    }

    async requestAccess(): Promise<SerialPortInfo | null> {
        return null;
    }

    async listPorts(): Promise<SerialPortInfo[]> {
        return [];
    }

    async open(_portId: string): Promise<void> {
        this.notImplemented();
    }

    async configure(_portId: string, _options: { readonly baudRate: number }): Promise<void> {
        this.notImplemented();
    }

    async write(_portId: string, _bytes: Uint8Array): Promise<void> {
        this.notImplemented();
    }

    async flush(_portId: string): Promise<void> {
        this.notImplemented();
    }

    drainInbox(_portId: string): Uint8Array {
        return new Uint8Array(0);
    }

    async close(_portId: string): Promise<void> {
        this.notImplemented();
    }
}
