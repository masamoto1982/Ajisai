/* tslint:disable */
/* eslint-disable */

export class AjisaiInterpreter {
    free(): void;
    [Symbol.dispose](): void;
    collect_core_word_aliases_info(): any;
    collect_core_words_info(): any;
    collect_dictionary_dependencies(): any;
    collect_hedged_trace(): any;
    collect_imported_modules(): any;
    collect_input_helper_words_info(): any;
    collect_module_sample_words_info(_module_name: string): any;
    collect_module_words_info(_module_name: string): any;
    collect_stack(): any;
    collect_user_words_info(): any;
    execute(code: string): any;
    execute_step(code: string): any;
    get_execution_mode(): any;
    lookup_word_definition(name: string): string | undefined;
    constructor();
    push_json_string(_json: string): any;
    remove_word(name: string): void;
    reset(): any;
    restore_imported_modules(_modules: any): void;
    restore_stack(_stack_js: any): void;
    restore_user_words(words: any): void;
    set_execution_mode(_mode: string): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_ajisaiinterpreter_free: (a: number, b: number) => void;
    readonly ajisaiinterpreter_collect_core_word_aliases_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_core_words_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_dictionary_dependencies: (a: number) => any;
    readonly ajisaiinterpreter_collect_input_helper_words_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_module_sample_words_info: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_collect_stack: (a: number) => any;
    readonly ajisaiinterpreter_collect_user_words_info: (a: number) => any;
    readonly ajisaiinterpreter_execute: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_get_execution_mode: (a: number) => any;
    readonly ajisaiinterpreter_lookup_word_definition: (a: number, b: number, c: number) => [number, number];
    readonly ajisaiinterpreter_new: () => number;
    readonly ajisaiinterpreter_push_json_string: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_remove_word: (a: number, b: number, c: number) => void;
    readonly ajisaiinterpreter_reset: (a: number) => any;
    readonly ajisaiinterpreter_restore_imported_modules: (a: number, b: any) => void;
    readonly ajisaiinterpreter_restore_user_words: (a: number, b: any) => void;
    readonly ajisaiinterpreter_set_execution_mode: (a: number, b: number, c: number) => void;
    readonly ajisaiinterpreter_collect_hedged_trace: (a: number) => any;
    readonly ajisaiinterpreter_collect_imported_modules: (a: number) => any;
    readonly ajisaiinterpreter_collect_module_words_info: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_restore_stack: (a: number, b: any) => void;
    readonly ajisaiinterpreter_execute_step: (a: number, b: number, c: number) => any;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
