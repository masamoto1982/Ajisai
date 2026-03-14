/* tslint:disable */
/* eslint-disable */

export class AjisaiInterpreter {
    free(): void;
    [Symbol.dispose](): void;
    clear_io_output_buffer(): void;
    execute(code: string): Promise<any>;
    execute_step(code: string): any;
    get_builtin_words_info(): any;
    get_custom_words_info(): any;
    /**
     * IMPORT済みモジュール名の一覧を返す。
     * 例: ["MUSIC", "JSON"]
     */
    get_imported_modules(): any;
    get_io_output_buffer(): string;
    /**
     * 指定モジュールが公開するワード情報を返す。
     * 返却形式は Array<[name, description]>
     */
    get_module_words_info(module_name: string): any;
    get_stack(): any;
    get_word_definition(name: string): any;
    constructor();
    push_json_string(json_string: string): any;
    remove_word(name: string): void;
    reset(): any;
    restore_custom_words(words_js: any): void;
    /**
     * JS側からモジュール状態を復元する。
     * 配列 ["MUSIC", "JSON"] のような形式で受け取り、各モジュールを再登録する。
     */
    restore_imported_modules(modules_js: any): void;
    restore_stack(stack_js: any): void;
    set_input_buffer(text: string): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_ajisaiinterpreter_free: (a: number, b: number) => void;
    readonly ajisaiinterpreter_clear_io_output_buffer: (a: number) => void;
    readonly ajisaiinterpreter_execute: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_execute_step: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_get_builtin_words_info: (a: number) => any;
    readonly ajisaiinterpreter_get_custom_words_info: (a: number) => any;
    readonly ajisaiinterpreter_get_imported_modules: (a: number) => any;
    readonly ajisaiinterpreter_get_io_output_buffer: (a: number) => [number, number];
    readonly ajisaiinterpreter_get_module_words_info: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_get_stack: (a: number) => any;
    readonly ajisaiinterpreter_get_word_definition: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_new: () => number;
    readonly ajisaiinterpreter_push_json_string: (a: number, b: number, c: number) => [number, number, number];
    readonly ajisaiinterpreter_remove_word: (a: number, b: number, c: number) => void;
    readonly ajisaiinterpreter_reset: (a: number) => any;
    readonly ajisaiinterpreter_restore_custom_words: (a: number, b: any) => [number, number];
    readonly ajisaiinterpreter_restore_imported_modules: (a: number, b: any) => void;
    readonly ajisaiinterpreter_restore_stack: (a: number, b: any) => [number, number];
    readonly ajisaiinterpreter_set_input_buffer: (a: number, b: number, c: number) => void;
    readonly wasm_bindgen__closure__destroy__h881c6ff590a92e5d: (a: number, b: number) => void;
    readonly wasm_bindgen__closure__destroy__hf49e72bbb773a9bc: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1a9b24889a5b283c: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h3a5ef20a20cefb4e: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__he32a8c35689af319: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
