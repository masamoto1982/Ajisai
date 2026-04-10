declare module '@tauri-apps/plugin-dialog' {
  export function confirm(
    message: string,
    options?: { title?: string; okLabel?: string; cancelLabel?: string }
  ): Promise<boolean>;
  export function message(
    message: string,
    options?: { title?: string; kind?: 'info' | 'warning' | 'error'; okLabel?: string }
  ): Promise<void>;
  export function open(options?: {
    title?: string;
    multiple?: boolean;
    filters?: Array<{ name: string; extensions: string[] }>;
  }): Promise<string | string[] | null>;
  export function save(options?: {
    title?: string;
    defaultPath?: string;
    filters?: Array<{ name: string; extensions: string[] }>;
  }): Promise<string | null>;
}

declare module '@tauri-apps/plugin-fs' {
  export function readTextFile(path: string): Promise<string>;
  export function writeTextFile(path: string, contents: string): Promise<void>;
}
