// js/wasm-types.ts

export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

export interface AjisaiInterpreter {
    execute(code: string): ExecuteResult;
    init_step(code: string): string;
    step(): StepResult;
    get_workspace(): Value[];  // get_stack → get_workspace
    get_custom_words(): string[];
    get_custom_words_with_descriptions(): Array<[string, string | null]>;
    get_custom_words_info(): Array<[string, string | null, boolean]>;
    get_builtin_words_info(): Array<[string, string | null]>;
    get_builtin_words_by_category(): any;
    reset(): void;
    save_table(name: string, schema: any, records: any): void;
    load_table(name: string): any;
    get_all_tables(): string[];
    restore_workspace(workspace_js: Value[]): void;  // restore_stack → restore_workspace
    get_word_definition(name: string): string | null;
    restore_word(name: string, definition: string, description?: string | null): void;
}

export interface ExecuteResult {
    status: 'OK' | 'ERROR';
    output?: string;
    autoNamed?: boolean;
    autoNamedWord?: string;
    message?: string;
    error?: boolean;
}

export interface StepResult {
    hasMore: boolean;
    output?: string;
    position?: number;
    total?: number;
}

export interface Value {
    type: string;
    value: any;
}

export interface WasmModule {
    AjisaiInterpreter: AjisaiInterpreterClass;
    default?: () => Promise<void>;
    init?: () => Promise<void>;
}
