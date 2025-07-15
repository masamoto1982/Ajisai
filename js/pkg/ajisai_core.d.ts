/* tslint:disable */
/* eslint-disable */
export class AjisaiInterpreter {
  free(): void;
  constructor();
  execute(code: string): any;
  init_step(code: string): string;
  step(): any;
  get_stack(): any;
  get_register(): any;
  get_custom_words(): string[];
  get_custom_words_with_descriptions(): any;
  get_custom_words_info(): any;
  reset(): void;
  save_table(name: string, schema: any, records: any): void;
  load_table(name: string): any;
  get_all_tables(): string[];
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_ajisaiinterpreter_free: (a: number, b: number) => void;
  readonly ajisaiinterpreter_new: () => number;
  readonly ajisaiinterpreter_execute: (a: number, b: number, c: number) => [number, number, number];
  readonly ajisaiinterpreter_init_step: (a: number, b: number, c: number) => [number, number, number, number];
  readonly ajisaiinterpreter_step: (a: number) => [number, number, number];
  readonly ajisaiinterpreter_get_stack: (a: number) => any;
  readonly ajisaiinterpreter_get_register: (a: number) => any;
  readonly ajisaiinterpreter_get_custom_words: (a: number) => [number, number];
  readonly ajisaiinterpreter_get_custom_words_with_descriptions: (a: number) => any;
  readonly ajisaiinterpreter_get_custom_words_info: (a: number) => any;
  readonly ajisaiinterpreter_reset: (a: number) => void;
  readonly ajisaiinterpreter_save_table: (a: number, b: number, c: number, d: any, e: any) => [number, number];
  readonly ajisaiinterpreter_load_table: (a: number, b: number, c: number) => any;
  readonly ajisaiinterpreter_get_all_tables: (a: number) => [number, number];
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __externref_drop_slice: (a: number, b: number) => void;
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
