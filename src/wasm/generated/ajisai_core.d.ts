/* tslint:disable */
/* eslint-disable */

export class AjisaiInterpreter {
    free(): void;
    [Symbol.dispose](): void;
    clear_io_output_buffer(): void;
    /**
     * Clear all injected serial receive buffers and disconnected flags.
     */
    clear_serial_inboxes(): void;
    /**
     * Discard every value on the stack, leaving the dictionary, the output
     * and every other piece of session state untouched.
     *
     * A REPL keeps its stack between runs, which is right, and until now the
     * only way to get rid of a leftover intermediate was the full reset — and
     * that takes the User dictionary with it. Clearing values is not a
     * language operation (no Word does it, and none should: a program's own
     * values are its own business), so it belongs here, on the host, where the
     * person at the keyboard is the one asking.
     */
    clear_stack(): void;
    collect_builtin_word_registry(): any;
    /**
     * Returns the canonical Core-listed words.
     *
     * Tuple shape: `(name, description, syntax)` — same as
     * `collect_core_words_info` so the GUI can render either list with the
     * same code path.
     */
    collect_core_listed_words_info(): any;
    collect_core_word_aliases_info(): any;
    collect_core_words_info(): any;
    collect_error_flow_trace(): any;
    collect_input_helper_words_info(): any;
    /**
     * Runtime counters for the Playground. Counts are session-cumulative and
     * reset with the interpreter. Observational only.
     */
    collect_runtime_metrics(): any;
    collect_stack(): any;
    collect_user_words_info(): any;
    /**
     * Content identity (Section 8.6) of each user word, as `[fqName, id]`
     * pairs. The host uses these to deduplicate identical definitions on
     * import and to key shared word groups by content rather than by name.
     */
    collect_word_identities(): any;
    execute(code: string): Promise<any>;
    execute_step(code: string): any;
    extract_io_output_buffer(): string;
    is_safe_preview_word(name: string): boolean;
    lookup_word_definition(name: string): any;
    /**
     * Mark a serial port as disconnected by the host. Once its inbox is empty,
     * `SERIAL@READ` projects `NilReason::PortDisconnected`.
     */
    mark_serial_disconnected(_port_id: string): void;
    constructor();
    push_json_string(json_string: string): any;
    remove_word(name: string): void;
    reset(): any;
    /**
     * Compatibility alias for [`Self::reset`].
     */
    reset_session(): any;
    /**
     * Restore a stack from a `snapshot_stack` payload, reinstating exact
     * values (CodeBlock, ExactScalar, …) and their stack-position roles.
     */
    restore_stack_snapshot(snapshot_json: string): void;
    restore_user_words(words_js: any): void;
    /**
     * Override the execution step budget (water level, SPEC §5.3) for
     * subsequent executions. A runtime safety control, not a language
     * semantic: the host may raise or lower it; never calling this keeps
     * the default (100,000). A zero or non-positive value is ignored so a
     * malformed host call cannot disable the safety budget entirely.
     */
    set_max_execution_steps(steps: number): void;
    /**
     * The one stack format persistence accepts (SPEC §2.3). Unlike
     * `collect_stack`, which serializes the *observation* wire format (a
     * CodeBlock shows as `nil`, an ExactScalar as a marked rational
     * approximation), this captures the exact value so `restore_stack_snapshot`
     * returns identical values. The two surfaces are deliberately distinct:
     * observation is lossy-but-honest, persistence is lossless. Restoring the
     * observation format is not offered — it would silently downgrade exact
     * values. The payload is an opaque JSON string produced by
     * `crate::types::value_persist`.
     */
    snapshot_stack(): string;
    update_input_buffer(_text: string): void;
    /**
     * Inject the host-received bytes for a serial port (Section 9.4). Replaces
     * any buffer previously set for this port id and clears the port's
     * disconnected flag. `SERIAL@READ` drains this buffer.
     */
    update_serial_inbox(_port_id: string, _bytes: Uint8Array): void;
}

/**
 * Install console_error_panic_hook so any panic on the WASM side
 * surfaces in the browser console with a JS-friendly stack trace
 * instead of an opaque `RuntimeError: unreachable executed` trap.
 * Idempotent (`set_once`). Called from the TS loader exactly once
 * right after wasm-bindgen `init`.
 */
export function init_panic_hook(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_ajisaiinterpreter_free: (a: number, b: number) => void;
    readonly ajisaiinterpreter_clear_io_output_buffer: (a: number) => void;
    readonly ajisaiinterpreter_clear_stack: (a: number) => void;
    readonly ajisaiinterpreter_collect_builtin_word_registry: (a: number) => any;
    readonly ajisaiinterpreter_collect_core_listed_words_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_core_word_aliases_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_core_words_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_error_flow_trace: (a: number) => any;
    readonly ajisaiinterpreter_collect_input_helper_words_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_runtime_metrics: (a: number) => any;
    readonly ajisaiinterpreter_collect_stack: (a: number) => any;
    readonly ajisaiinterpreter_collect_user_words_info: (a: number) => any;
    readonly ajisaiinterpreter_collect_word_identities: (a: number) => any;
    readonly ajisaiinterpreter_execute: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_execute_step: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_extract_io_output_buffer: (a: number) => [number, number];
    readonly ajisaiinterpreter_is_safe_preview_word: (a: number, b: number, c: number) => number;
    readonly ajisaiinterpreter_lookup_word_definition: (a: number, b: number, c: number) => any;
    readonly ajisaiinterpreter_mark_serial_disconnected: (a: number, b: number, c: number) => void;
    readonly ajisaiinterpreter_new: () => number;
    readonly ajisaiinterpreter_push_json_string: (a: number, b: number, c: number) => [number, number, number];
    readonly ajisaiinterpreter_remove_word: (a: number, b: number, c: number) => void;
    readonly ajisaiinterpreter_reset: (a: number) => any;
    readonly ajisaiinterpreter_restore_stack_snapshot: (a: number, b: number, c: number) => [number, number];
    readonly ajisaiinterpreter_restore_user_words: (a: number, b: any) => [number, number];
    readonly ajisaiinterpreter_set_max_execution_steps: (a: number, b: number) => void;
    readonly ajisaiinterpreter_snapshot_stack: (a: number) => [number, number, number, number];
    readonly ajisaiinterpreter_update_serial_inbox: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly init_panic_hook: () => void;
    readonly ajisaiinterpreter_update_input_buffer: (a: number, b: number, c: number) => void;
    readonly ajisaiinterpreter_reset_session: (a: number) => any;
    readonly ajisaiinterpreter_clear_serial_inboxes: (a: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hf668d5029c28e014: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h0896cde0637cafae: (a: number, b: number, c: any, d: any) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
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
