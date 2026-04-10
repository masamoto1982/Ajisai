import type { AjisaiRuntime } from '../core/ajisai-runtime-types';

export interface AutocompleteService {
    readonly collectAutocompleteWords: () => string[];
}

export const createAutocompleteService = (runtime: AjisaiRuntime): AutocompleteService => {
    const collectAutocompleteWords = (): string[] => {
        const coreWordsInfo = runtime.collectCoreWordsInfo();
        const coreWords: string[] = coreWordsInfo.map(word => word[0]).filter((w): w is string => w !== undefined);

        const userWordsInfo = runtime.collectUserWordsInfo();
        const userWords: string[] = userWordsInfo.flatMap(word => [
            word[1],
            `${word[0]}@${word[1]}`
        ]);

        const moduleWords: string[] = [];
        try {
            const importedModules: string[] = runtime.collectImportedModules();
            for (const moduleName of importedModules) {
                const words = runtime.collectModuleWordsInfo(moduleName);
                const prefix: string = `${moduleName}@`;
                for (const word of words) {
                    const name: string = word[0] ?? '';
                    moduleWords.push(name.startsWith(prefix) ? name.slice(prefix.length) : name);
                }
                const sampleWords = runtime.collectModuleSampleWordsInfo(moduleName);
                for (const word of sampleWords) {
                    const sampleName: string = word[0] ?? '';
                    moduleWords.push(sampleName);
                }
            }
        } catch {
            // Imported module retrieval is optional.
        }

        const allWords: Set<string> = new Set([...coreWords, ...userWords, ...moduleWords]);
        return Array.from(allWords).sort((a: string, b: string) => a.localeCompare(b));
    };

    return {
        collectAutocompleteWords
    };
};
