export interface DialogPort {
    confirm(message: string, options?: { title?: string; okLabel?: string; cancelLabel?: string }): Promise<boolean>;
    alert(message: string, options?: { title?: string; kind?: 'info' | 'warning' | 'error'; okLabel?: string }): Promise<void>;
}
