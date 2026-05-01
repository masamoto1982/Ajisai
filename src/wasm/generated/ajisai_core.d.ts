/* tslint:disable */
/* eslint-disable */

export class AjisaiInterpreter {
    free(): void;
    [Symbol.dispose](): void;
    clear_io_output_buffer(): void;
    collect_builtin_word_registry(): any;
    /**
     * Returns Core-listed words (canonical core + Canonical Module words
     * that are core-listed, e.g. SORT). This is the listing-based Core
     * view defined by the redesigned vocabulary system; bare module words
     * are surfaced for visibility only — invoking SORT bare still requires
     * `'ALGO' IMPORT` per current execution semantics.
     *
     * Tuple shape: `(name, description, syntax, signature_type)` — same as
     * `collect_core_words_info` so the GUI can render either list with the
     * same code path.
     */
    collect_core_listed_words_info(): any;
    collect_core_word_aliases_info(): any;
    collect_core_words_info(): any;
    collect_dictionary_dependencies(): any;
    collect_hedged_trace(): any;
    collect_imported_modules(): any;
    collect_input_helper_words_info(): any;
    collect_module_sample_words_info(module_name: string): any;
    collect_module_words_info(module_name: string): any;
    collect_stack(): any;
    collect_user_words_info(): any;
    execute(code: string): Promise<any>;
    execute_step(code: string): any;
    extract_io_output_buffer(): string;
    get_execution_mode(): string;
    is_safe_preview_word(name: string): boolean;
    lookup_word_definition(name: string): any;
    constructor();
    push_json_string(json_string: string): any;
    remove_word(name: string): void;
    reset(): any;
    restore_imported_modules(modules_js: any): void;
    restore_stack(stack_js: any): void;
    restore_user_words(words_js: any): void;
    set_execution_mode(mode: string): void;
    update_input_buffer(text: string): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_ajisaiinterpreter_free: (a: number, b: number) => void;
    readonly ajisaiinterpreter_clear_io_output_buffer: (a: number) => void;
    readonly ajisaiinterpreter_collect_builtin_word_registry: (a: number) => any;
    readonly ajisaiinterpreter_collect_core_listed_words_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_core_word_aliases_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_core_words_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_dictionary_dependencies: (a: number) => any;
    readonly ajisaiinterpreter_collect_hedged_trace: (a: number) => any;
    readonly ajisaiinterpreter_collect_imported_modules: (a: number) => any;
    readonly ajisaiinterpreter_collect_input_helper_words_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_module_sample_words_info: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_collect_module_words_info: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_collect_stack: (a: number) => any;
    readonly ajisaiinterpreter_collect_user_words_info: (a: number) => any;
    readonly ajisaiinterpreter_execute: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_execute_step: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_extract_io_output_buffer: (a: number) => [number, number];
    readonly ajisaiinterpreter_get_execution_mode: (a: number) => [number, number];
    readonly ajisaiinterpreter_is_safe_preview_word: (a: number, b: number, c: number) => number;
    readonly ajisaiinterpreter_lookup_word_definition: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_new: () => number;
    readonly ajisaiinterpreter_push_json_string: (a: number, b: number, c: number) => [number, number, number];
    readonly ajisaiinterpreter_remove_word: (a: number, b: number, c: number) => void;
    readonly ajisaiinterpreter_reset: (a: number) => any;
    readonly ajisaiinterpreter_restore_imported_modules: (a: number, b: any) => void;
    readonly ajisaiinterpreter_restore_stack: (a: number, b: any) => [number, number];
    readonly ajisaiinterpreter_restore_user_words: (a: number, b: any) => [number, number];
    readonly ajisaiinterpreter_set_execution_mode: (a: number, b: number, c: number) => void;
    readonly ajisaiinterpreter_update_input_buffer: (a: number, b: number, c: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h5d0e415c658b5598: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h168756633bc0765f: (a: number, b: number, c: any, d: any) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
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
