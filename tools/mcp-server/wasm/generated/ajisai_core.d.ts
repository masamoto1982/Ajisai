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
     * Answer the host's lookup of `name` against the current dictionary.
     *
     * This is a *query*, not a run. Looking a Word up used to be the Word
     * `LOOKUP`, which meant asking what `ADD` does went through `execute` and
     * came back on a side channel that no evaluation rule read. The host asks
     * here instead, so nothing about a lookup touches the stack, the
     * dictionary, or the output buffer.
     *
     * Returns `{ kind: "documentation" | "definition", text }`, or `NULL` for a
     * name the dictionary does not hold — the caller reports the unknown name
     * itself, since it is the one that read it off the input.
     */
    resolve_host_lookup(name: string): any;
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
 * Parse and resolve `source` without executing it; also verifies declared
 * `#:contract` declarations conservatively, matching `ajisai agent check`.
 */
export function agent_check(source: string): string;

/**
 * Execute one Ajisai source document under the same tightened
 * agent-profile runtime limits the native `ajisai agent compute` CLI
 * applies. `step_limit` overrides the default execution step budget when
 * positive; `0` or omitted keeps the interpreter default.
 */
export function agent_compute(source: string, step_limit?: number | null): Promise<string>;

/**
 * Infer machine-readable contracts for user-defined Words without
 * executing their bodies, matching `ajisai agent infer-contracts`.
 */
export function agent_infer_contracts(source: string): string;

/**
 * Install console_error_panic_hook so any panic on the WASM side
 * surfaces in the browser console with a JS-friendly stack trace
 * instead of an opaque `RuntimeError: unreachable executed` trap.
 * Idempotent (`set_once`). Called from the TS loader exactly once
 * right after wasm-bindgen `init`.
 */
export function init_panic_hook(): void;
