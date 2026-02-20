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
    remove_word(name: string): void;
    set_input_buffer(text: string): void;
    get_io_output_buffer(): string;
    clear_io_output_buffer(): void;
}

export interface ExecuteResult {
    status: 'OK' | 'ERROR';
    output?: string;
    debugOutput?: string; // Add this back for debug messages
    message?: string;
    error?: boolean;
    hasMore?: boolean;
    definition_to_load?: string;
    inputHelper?: string; // Input helper text to insert into the editor
    // Workerから返されるインタプリタの状態
    stack?: Value[];
    customWords?: CustomWord[];
    // I/O: OUTPUTワードの出力バッファ
    ioOutput?: string;
}

export interface Fraction {
    numerator: string;
    denominator: string;
}

export interface Value {
    type: string;
    value: any | Fraction | Value[];
}

export interface WasmModule {
    AjisaiInterpreter: AjisaiInterpreterClass;
    default?: () => Promise<any>;
    init?: () => Promise<any>;
}
