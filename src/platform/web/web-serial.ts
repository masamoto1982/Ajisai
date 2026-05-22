import type { SerialAdapter, SerialPortInfo } from '../platform-adapter';

// Minimal local declarations for the W3C Web Serial API so the build does not
// depend on the surrounding TS DOM lib shipping these types yet.
interface WebSerialPort {
    open(options: { baudRate: number }): Promise<void>;
    close(): Promise<void>;
    readonly writable: WritableStream<Uint8Array> | null;
    getInfo?(): { usbVendorId?: number; usbProductId?: number };
}

interface WebSerial {
    requestPort(): Promise<WebSerialPort>;
    getPorts(): Promise<WebSerialPort[]>;
}

const getNavigatorSerial = (): WebSerial | undefined =>
    (navigator as unknown as { serial?: WebSerial }).serial;

const DEFAULT_BAUD_RATE = 9600;

interface GrantedPort {
    readonly id: string;
    readonly port: WebSerialPort;
    baudRate: number;
    opened: boolean;
    writer: WritableStreamDefaultWriter<Uint8Array> | null;
}

/**
 * Web Serial backend. Real `SerialPort` objects live here, on the main thread,
 * keyed by an opaque string id that Ajisai programs use as the connection
 * handle. Operations are serialized through a single promise chain so the
 * fire-and-forget command stream (open → configure → write → close) executes
 * in order even though the dispatcher does not await between commands.
 */
export class WebSerialAdapter implements SerialAdapter {
    private readonly ports: GrantedPort[] = [];
    private queue: Promise<unknown> = Promise.resolve();

    get available(): boolean {
        return getNavigatorSerial() !== undefined;
    }

    private enqueue<T>(op: () => Promise<T>): Promise<T> {
        const result = this.queue.then(op);
        // Keep the chain alive even if an op rejects.
        this.queue = result.catch(() => undefined);
        return result;
    }

    private register(port: WebSerialPort): GrantedPort {
        const existing = this.ports.find(p => p.port === port);
        if (existing) return existing;
        const entry: GrantedPort = {
            id: `serial-${this.ports.length}`,
            port,
            baudRate: DEFAULT_BAUD_RATE,
            opened: false,
            writer: null
        };
        this.ports.push(entry);
        return entry;
    }

    private require(portId: string): GrantedPort {
        const entry = this.ports.find(p => p.id === portId);
        if (!entry) {
            throw new Error(`Serial port '${portId}' has not been granted. Use the connect control first.`);
        }
        return entry;
    }

    requestAccess(): Promise<SerialPortInfo | null> {
        return this.enqueue(async () => {
            const serial = getNavigatorSerial();
            if (!serial) return null;
            const port = await serial.requestPort();
            const entry = this.register(port);
            return { portId: entry.id };
        });
    }

    listPorts(): Promise<SerialPortInfo[]> {
        return this.enqueue(async () => {
            const serial = getNavigatorSerial();
            if (!serial) return [];
            const granted = await serial.getPorts();
            granted.forEach(port => this.register(port));
            return this.ports.map(entry => ({ portId: entry.id }));
        });
    }

    private async reopen(entry: GrantedPort): Promise<void> {
        if (entry.writer) {
            try { entry.writer.releaseLock(); } catch { /* ignore */ }
            entry.writer = null;
        }
        if (entry.opened) {
            await entry.port.close();
            entry.opened = false;
        }
        await entry.port.open({ baudRate: entry.baudRate });
        entry.opened = true;
    }

    open(portId: string): Promise<void> {
        return this.enqueue(async () => {
            const entry = this.require(portId);
            if (!entry.opened) {
                await entry.port.open({ baudRate: entry.baudRate });
                entry.opened = true;
            }
        });
    }

    configure(portId: string, options: { readonly baudRate: number }): Promise<void> {
        return this.enqueue(async () => {
            const entry = this.require(portId);
            const changed = entry.baudRate !== options.baudRate;
            entry.baudRate = options.baudRate;
            // Web Serial cannot change baud rate while open; reopen if needed.
            if (entry.opened && changed) {
                await this.reopen(entry);
            }
        });
    }

    write(portId: string, bytes: Uint8Array): Promise<void> {
        return this.enqueue(async () => {
            const entry = this.require(portId);
            if (!entry.opened) {
                await entry.port.open({ baudRate: entry.baudRate });
                entry.opened = true;
            }
            const writable = entry.port.writable;
            if (!writable) {
                throw new Error(`Serial port '${portId}' is not writable.`);
            }
            if (!entry.writer) {
                entry.writer = writable.getWriter();
            }
            await entry.writer.write(bytes);
        });
    }

    flush(portId: string): Promise<void> {
        // Writes are not buffered at this layer; flush is a no-op acknowledgement.
        return this.enqueue(async () => {
            this.require(portId);
        });
    }

    drainInbox(_portId: string): Uint8Array {
        // Phase 2: a per-port RX reader loop will fill a buffer drained here and
        // injected into the next execution snapshot. Phase 1 is send-only.
        return new Uint8Array(0);
    }

    close(portId: string): Promise<void> {
        return this.enqueue(async () => {
            const entry = this.require(portId);
            if (entry.writer) {
                try { entry.writer.releaseLock(); } catch { /* ignore */ }
                entry.writer = null;
            }
            if (entry.opened) {
                await entry.port.close();
                entry.opened = false;
            }
        });
    }
}
