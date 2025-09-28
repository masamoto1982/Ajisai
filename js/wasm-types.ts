export interface AjisaiInterpreterClass {
    new(): AjisaiInterpreter;
}

export interface CustomWord {
    name: string;
    definition: string | null;
    description: string | null;
}

export interface AjisaiInterpreter {
    execute(code: string): ExecuteResult;
    execute_step(code: string): ExecuteResult;
    init_step(code: string): string;
    step(): StepResult;
    reset(): ExecuteResult;
    get_stack(): Value[];
    get_custom_words_info(): Array<[string, string | null, boolean]>;
    get_builtin_words_info(): Array<[string, string, string]>;
    get_word_definition(name: string): string | null;
    restore_stack(stack_js: Value[]): void;
    restore_word(name: string, definition: string, description?: string | null): void;
    restore_custom_words(words: CustomWord[]): void;
    rebuild_dependencies(): { status: string; message: string };
}

export interface ExecuteResult {
    status: 'OK' | 'ERROR';
    output?: string;
    debugOutput?: string;
    message?: string;
    error?: boolean;
    hasMore?: boolean;
    position?: number;
    total?: number;
    definition_to_load?: string;
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
    default?: () => Promise<any>;
    init?: () => Promise<any>;
}
