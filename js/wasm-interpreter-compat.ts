import type { AjisaiInterpreter } from './wasm-interpreter-types';

type MaybeFn = ((...args: any[]) => any) | undefined;

type LegacyInterpreter = {
    [key: string]: unknown;
    get_core_words_info?: MaybeFn;
    get_idiolect_words_info?: MaybeFn;
    get_imported_modules?: MaybeFn;
    get_module_words_info?: MaybeFn;
    get_stack?: MaybeFn;
    get_word_definition?: MaybeFn;
    restore_idiolect?: MaybeFn;
};

const aliasMethod = (instance: LegacyInterpreter, modernName: string, legacyName: keyof LegacyInterpreter): void => {
    const modern = instance[modernName] as MaybeFn;
    const legacy = instance[legacyName] as MaybeFn;
    if (typeof modern === 'function' || typeof legacy !== 'function') return;
    Object.defineProperty(instance, modernName, {
        configurable: true,
        enumerable: false,
        writable: true,
        value: (...args: any[]) => legacy(...args),
    });
};

const ensureRequiredMethods = (instance: LegacyInterpreter): string[] => {
    const required = [
        'collect_stack',
        'collect_user_words_info',
        'collect_core_words_info',
        'collect_imported_modules',
        'collect_module_words_info',
        'lookup_word_definition',
        'restore_user_words',
    ];

    return required.filter((name) => typeof instance[name] !== 'function');
};

export const normalizeInterpreterApi = (interpreter: AjisaiInterpreter): AjisaiInterpreter => {
    const instance = interpreter as unknown as LegacyInterpreter;

    aliasMethod(instance, 'collect_core_words_info', 'get_core_words_info');
    aliasMethod(instance, 'collect_user_words_info', 'get_idiolect_words_info');
    aliasMethod(instance, 'collect_imported_modules', 'get_imported_modules');
    aliasMethod(instance, 'collect_module_words_info', 'get_module_words_info');
    aliasMethod(instance, 'collect_stack', 'get_stack');
    aliasMethod(instance, 'lookup_word_definition', 'get_word_definition');
    aliasMethod(instance, 'restore_user_words', 'restore_idiolect');

    const missing = ensureRequiredMethods(instance);
    if (missing.length > 0) {
        console.warn(
            '[WASM] Interpreter API mismatch. Rebuild js/pkg from rust sources. Missing methods:',
            missing.join(', ')
        );
    }

    return instance as unknown as AjisaiInterpreter;
};
