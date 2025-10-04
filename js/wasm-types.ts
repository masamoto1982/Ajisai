// js/wasm-types.ts

export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

export interface CustomWord {
    name: string;
    definition: string | null;
    description: string | null;
}

export interface AjisaiInterpreter {
    execute(code: string): Promise<ExecuteResult>;
    execute_step(code: string): ExecuteResult;
    reset(): ExecuteResult;
    get_stack(): Value[];
    get_custom_words_info(): Array<[string, string | null, boolean]>;
    get_builtin_words_info(): Array<[string, string, string]>;
    get_word_definition(name: string): string | null;
    restore_stack(stack_js: Value[]): void;
    restore_custom_words(words: CustomWord[]): void;
}

export interface ExecuteResult {
    status: 'OK' | 'ERROR';
    output?: string;
    message?: string;
    error?: boolean;
    definition_to_load?: string;
    // Workerから返されるインタプリタの状態
    stack?: Value[];
    customWords?: CustomWord[];
}

export interface Fraction {
    numerator: string;
    denominator: string;
}

export interface Value {
    type: string;
    value: any | Fraction | Value[];
    bracketType?: 'square' | 'curly' | 'round';
}

export interface WasmModule {
    AjisaiInterpreter: AjisaiInterpreterClass;
    default?: () => Promise<any>;
    init?: () => Promise<any>;
}
