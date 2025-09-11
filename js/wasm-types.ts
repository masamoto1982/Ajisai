// js/wasm-types.ts (RESET対応版)

export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

export interface AjisaiInterpreter {
    execute(code: string): ExecuteResult;
    init_step(code: string): string;
    step(): StepResult;
    reset(): ExecuteResult;
    get_workspace(): Value[];
    get_custom_words(): string[];
    get_custom_words_with_descriptions(): Array<[string, string | null]>;
    get_custom_words_info(): Array<[string, string | null, boolean]>;
    get_builtin_words_info(): Array<[string, string | null]>;
    get_builtin_words_by_category(): any;
    reset_workspace(): void;
    save_table(name: string, schema: any, records: any): void;
    load_table(name: string): any;
    get_all_tables(): string[];
    restore_workspace(workspace_js: Value[]): void;
    get_word_definition(name: string): string | null;
    restore_word(name: string, definition: string, description?: string | null): void;
}

export interface ExecuteResult {
    status: 'OK' | 'ERROR';
    output?: string;
    debugOutput?: string;
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
    error?: boolean;
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
