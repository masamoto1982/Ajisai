// js/wasm-types.ts

export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

export interface AjisaiInterpreter {
    execute(code: string): ExecuteResult;
    init_step(code: string): string;
    step(): StepResult;
    get_stack(): Value[];
    get_register(): Value | null;
    get_custom_words(): string[];
    get_custom_words_with_descriptions(): Array<[string, string | null]>;
    get_custom_words_info(): Array<[string, string | null, boolean]>;
    reset(): void;
    save_table(name: string, schema: any, records: any): void;
    load_table(name: string): any;
    get_all_tables(): string[];
    restore_stack(stack_js: Value[]): void;
    restore_register(register_js: Value | null): void;
    get_word_definition(name: string): string | null;
    restore_word(name: string, definition: string, description?: string | null): void;
}

export interface ExecuteResult {
    status: string;
    output?: string;
    autoNamed?: boolean;
    autoNamedWord?: string;
    message?: string; // エラーの場合のメッセージ
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
