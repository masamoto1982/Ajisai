// js/wasm-types.ts (LPL対応)

export interface LPLInterpreterClass {
    new(): LPLInterpreter;
}

export interface LPLInterpreter {
    execute(code: string): ExecuteResult;
    init_step(code: string): string;
    step(): StepResult;
    amnesia(): ExecuteResult;
    get_bookshelf(): Value[];  // get_workspace → get_bookshelf
    get_custom_words(): string[];
    get_custom_words_with_descriptions(): Array<[string, string | null]>;
    get_custom_words_info(): Array<[string, string | null, boolean]>;
    get_builtin_words_info(): Array<[string, string | null]>;
    get_builtin_words_by_category(): any;
    reset(): void;
    save_table(name: string, schema: any, records: any): void;
    load_table(name: string): any;
    get_all_tables(): string[];
    restore_bookshelf(bookshelf_js: Value[]): void;  // restore_workspace → restore_bookshelf
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
    error?: boolean;
}

export interface Value {
    type: string;
    value: any;
}

export interface WasmModule {
    LPLInterpreter: LPLInterpreterClass;  // AjisaiInterpreter → LPLInterpreter
    default?: () => Promise<void>;
    init?: () => Promise<void>;
}
