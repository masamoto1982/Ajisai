

export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

export interface UserWord {
    dictionary?: string | null;
    name: string;
    definition: string | null;
    description: string | null;
}

export interface AjisaiInterpreter {
    execute(code: string): Promise<ExecuteResult>;
    execute_step(code: string): ExecuteResult;
    reset(): ExecuteResult;
    collect_stack(): Value[];
    collect_user_words_info(): Array<[string, string, string | null, boolean]>;
    collect_core_words_info(): Array<[string, string, string]>;
    lookup_word_definition(name: string): string | null;
    restore_stack(stack_js: Value[]): void;
    restore_user_words(words: UserWord[]): void;
    remove_word(name: string): void;
    push_json_string(json: string): { status: string; message?: string };
    collect_imported_modules(): string[];
    collect_module_words_info(module_name: string): Array<[string, string | null]>;
    collect_module_sample_words_info(module_name: string): Array<[string, string | null]>;
    collect_dictionary_dependencies(): Array<[string, string[], string[]]>;
    restore_imported_modules(modules: string[]): void;
}

export interface ExecuteResult {
    status: 'OK' | 'ERROR';
    output?: string;
    debugOutput?: string;
    message?: string;
    error?: boolean;
    hasMore?: boolean;
    definition_to_load?: string;
    inputHelper?: string;

    stack?: Value[];
    userWords?: UserWord[];
    importedModules?: string[];
    // Flow-token diagnostics are internal runtime invariants and are intentionally not part of default WASM result shape.
}

export interface Fraction {
    numerator: string;
    denominator: string;
}

export interface Value {
    type: string;
    value: any | Fraction | Value[];
    displayHint?: 'auto' | 'number' | 'string' | 'boolean' | 'datetime' | 'nil';
}

export interface WasmModule {
    AjisaiInterpreter: AjisaiInterpreterClass;
    default?: () => Promise<any>;
    init?: () => Promise<any>;
}
