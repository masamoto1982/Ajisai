import type { AjisaiInterpreter, UserWord, Value } from '../wasm-interpreter-types';
import type { AjisaiRuntime } from './ajisai-runtime-types';

export const createAjisaiRuntime = (interpreter: AjisaiInterpreter): AjisaiRuntime => ({
    execute: (code) => interpreter.execute(code),
    executeStep: (code) => interpreter.execute_step(code),
    reset: () => interpreter.reset(),
    collectStack: () => interpreter.collect_stack(),
    collectUserWordsInfo: () => interpreter.collect_user_words_info(),
    collectCoreWordsInfo: () => interpreter.collect_core_words_info(),
    lookupWordDefinition: (name) => interpreter.lookup_word_definition(name),
    restoreStack: (stack: Value[]) => interpreter.restore_stack(stack),
    restoreUserWords: async (words: UserWord[]) => interpreter.restore_user_words(words),
    removeWord: (name: string) => interpreter.remove_word(name),
    pushJsonString: (json: string) => interpreter.push_json_string(json),
    collectImportedModules: () => interpreter.collect_imported_modules(),
    collectModuleWordsInfo: (moduleName: string) => interpreter.collect_module_words_info(moduleName),
    collectModuleSampleWordsInfo: (moduleName: string) => interpreter.collect_module_sample_words_info(moduleName),
    collectDictionaryDependencies: () => interpreter.collect_dictionary_dependencies(),
    restoreImportedModules: (modules: string[]) => interpreter.restore_imported_modules(modules)
});
