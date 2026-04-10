import type { ExecuteResult, UserWord, Value } from '../wasm-interpreter-types';

export interface AjisaiRuntime {
    readonly execute: (code: string) => Promise<ExecuteResult>;
    readonly executeStep: (code: string) => ExecuteResult;
    readonly reset: () => ExecuteResult;
    readonly collectStack: () => Value[];
    readonly collectUserWordsInfo: () => Array<[string, string, string | null, boolean]>;
    readonly collectCoreWordsInfo: () => Array<[string, string, string]>;
    readonly lookupWordDefinition: (name: string) => string | null;
    readonly restoreStack: (stack: Value[]) => void;
    readonly restoreUserWords: (words: UserWord[]) => Promise<void>;
    readonly removeWord: (name: string) => void;
    readonly pushJsonString: (json: string) => { status: string; message?: string };
    readonly collectImportedModules: () => string[];
    readonly collectModuleWordsInfo: (moduleName: string) => Array<[string, string | null]>;
    readonly collectModuleSampleWordsInfo: (moduleName: string) => Array<[string, string | null]>;
    readonly collectDictionaryDependencies: () => Array<[string, string[], string[]]>;
    readonly restoreImportedModules: (modules: string[]) => void;
}
