import type { SerialAdapter, SerialInboxData, SerialPortInfo } from '../platform-adapter';

// Avoid statically bundling the Tauri API into the web build; resolved only at
// runtime inside the Tauri WebView (matches tauri-file-io.ts).
const dynamicImport = (specifier: string): Promise<any> =>
    (0, eval)(`import(${JSON.stringify(specifier)})`);

const invoke = async (command: string, args?: Record<string, unknown>): Promise<any> => {
    const { invoke } = await dynamicImport('@tauri-apps/api/core');
    return invoke(command, args);
};

const DEFAULT_BAUD_RATE = 9600;

interface PortState {
    opened: boolean;
    baudRate: number;
    rx: number[];
    disconnected: boolean;
}

/**
 * Native serial backend for the Tauri desktop channel. Outbound calls go to the
 * `serial_*` commands in `src-tauri`; received bytes arrive as `serial-rx`
 * events from the native reader thread and accumulate in a local buffer so
 * `drainAllInboxes()` can return them synchronously when a run's snapshot is
 * built — the same shape as the Web Serial adapter.
 */
export class TauriSerialAdapter implements SerialAdapter {
    readonly available = true;

    private readonly ports = new Map<string, PortState>();
    private listenersReady: Promise<void> | null = null;

    private entry(portId: string): PortState {
        let state = this.ports.get(portId);
        if (!state) {
            state = { opened: false, baudRate: DEFAULT_BAUD_RATE, rx: [], disconnected: false };
            this.ports.set(portId, state);
        }
        return state;
    }

    private ensureListeners(): Promise<void> {
        if (!this.listenersReady) {
            this.listenersReady = (async () => {
                const { listen } = await dynamicImport('@tauri-apps/api/event');
                await listen('serial-rx', (event: { payload: { portId: string; bytes: number[] } }) => {
                    const entry = this.entry(event.payload.portId);
                    for (const b of event.payload.bytes) entry.rx.push(b);
                });
                await listen('serial-disconnect', (event: { payload: { portId: string } }) => {
                    this.entry(event.payload.portId).disconnected = true;
                });
            })();
        }
        return this.listenersReady;
    }

    async requestAccess(): Promise<SerialPortInfo | null> {
        await this.ensureListeners();
        const ports = await this.listPorts();
        return ports[0] ?? null;
    }

    async listPorts(): Promise<SerialPortInfo[]> {
        const names: string[] = await invoke('serial_list_ports');
        return names.map(portId => ({ portId }));
    }

    async open(portId: string): Promise<void> {
        await this.ensureListeners();
        const entry = this.entry(portId);
        await invoke('serial_open', { portId, baudRate: entry.baudRate });
        entry.opened = true;
        entry.disconnected = false;
    }

    async configure(portId: string, options: { readonly baudRate: number }): Promise<void> {
        const entry = this.entry(portId);
        entry.baudRate = options.baudRate;
        if (entry.opened) {
            await invoke('serial_configure', { portId, baudRate: options.baudRate });
        }
    }

    async write(portId: string, bytes: Uint8Array): Promise<void> {
        await invoke('serial_write', { portId, bytes: Array.from(bytes) });
    }

    async flush(portId: string): Promise<void> {
        await invoke('serial_flush', { portId });
    }

    drainInbox(portId: string): Uint8Array {
        const entry = this.ports.get(portId);
        if (!entry || entry.rx.length === 0) return new Uint8Array(0);
        const bytes = Uint8Array.from(entry.rx);
        entry.rx = [];
        return bytes;
    }

    drainAllInboxes(): SerialInboxData[] {
        const result: SerialInboxData[] = [];
        for (const [portId, entry] of this.ports) {
            if (entry.opened || entry.rx.length > 0 || entry.disconnected) {
                const bytes = entry.rx;
                entry.rx = [];
                result.push({ portId, bytes, disconnected: entry.disconnected });
            }
        }
        return result;
    }

    async close(portId: string): Promise<void> {
        await invoke('serial_close', { portId });
        const entry = this.ports.get(portId);
        if (entry) entry.opened = false;
    }
}
