import type { AjisaiInterpreter } from '../../wasm-interpreter-types';

function getOptionalInterpreter(): AjisaiInterpreter | null {
    return window.ajisaiInterpreter ?? null;
}

function getRequiredInterpreter(): AjisaiInterpreter {
    const interpreter = getOptionalInterpreter();
    if (!interpreter) {
        throw new Error('Ajisai interpreter is not initialized.');
    }
    return interpreter;
}

export type InterpreterClient = {
    readonly getOptional: () => AjisaiInterpreter | null;
    readonly getRequired: () => AjisaiInterpreter;
    readonly collectCoreWordsInfo: () => ReturnType<AjisaiInterpreter['collect_core_words_info']>;
    readonly collectUserWordsInfo: () => ReturnType<AjisaiInterpreter['collect_user_words_info']>;
    readonly collectStack: () => ReturnType<AjisaiInterpreter['collect_stack']>;
};

export function createInterpreterClient(): InterpreterClient {
    return {
        getOptional: getOptionalInterpreter,
        getRequired: getRequiredInterpreter,
        collectCoreWordsInfo: () => getRequiredInterpreter().collect_core_words_info(),
        collectUserWordsInfo: () => getRequiredInterpreter().collect_user_words_info(),
        collectStack: () => getRequiredInterpreter().collect_stack()
    };
}
