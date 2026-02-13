/* tslint:disable */
/* eslint-disable */

export class AjisaiInterpreter {
    free(): void;
    [Symbol.dispose](): void;
    execute(code: string): Promise<any>;
    execute_step(code: string): any;
    get_builtin_words_info(): any;
    get_custom_words_info(): any;
    get_stack(): any;
    get_word_definition(name: string): any;
    constructor();
    /**
     * 辞書から指定されたワードを直接削除する（依存関係チェックなし）。
     * syncInterpreterState で Worker 側の削除をメインスレッドに反映するために使用。
     */
    remove_word(name: string): void;
    reset(): any;
    restore_custom_words(words_js: any): void;
    restore_stack(stack_js: any): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_ajisaiinterpreter_free: (a: number, b: number) => void;
    readonly ajisaiinterpreter_execute: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_execute_step: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_get_builtin_words_info: (a: number) => any;
    readonly ajisaiinterpreter_get_custom_words_info: (a: number) => any;
    readonly ajisaiinterpreter_get_stack: (a: number) => any;
    readonly ajisaiinterpreter_get_word_definition: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_new: () => number;
    readonly ajisaiinterpreter_remove_word: (a: number, b: number, c: number) => void;
    readonly ajisaiinterpreter_reset: (a: number) => any;
    readonly ajisaiinterpreter_restore_custom_words: (a: number, b: any) => [number, number];
    readonly ajisaiinterpreter_restore_stack: (a: number, b: any) => [number, number];
    readonly wasm_bindgen__closure__destroy__hc7ddc5b9c6ef0bf5: (a: number, b: number) => void;
    readonly wasm_bindgen__closure__destroy__hcf049d74e7de4874: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h6f232364e7aff175: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h782fde46ff4164f1: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h24bf0fdab81abd1b: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
