// js/wasm-types.ts (BigInt対応版)

export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

export interface AjisaiInterpreter {
    execute(code: string): ExecuteResult;
    init_step(code: string): string;
    step(): StepResult;
    reset(): ExecuteResult;
    get_workspace(): Value[];
    get_custom_words_info(): Array<[string, string | null, boolean]>;
    get_builtin_words_info(): Array<[string, string]>;
    get_word_definition(name: string): string | null;
    restore_workspace(workspace_js: Value[]): void;
    restore_word(name: string, definition: string, description?: string | null): void;
}

export interface ExecuteResult {
    status: 'OK' | 'ERROR';
    output?: string;
    debugOutput?: string;
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
    default?: () => Promise<void>;
    init?: () => Promise<void>;
}
