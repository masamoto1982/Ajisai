// js/gui/module-tabs.ts

import {
    createEmptyWordsMessage,
    createNoResultsMessage,
    createWordButton,
    matchesFilter,
    setupBackgroundClickHandlers,
    sortWordName,
} from './dictionary-ui';

export interface ModuleSheet {
    readonly moduleName: string;
    readonly sheetId: string;
    readonly optionEl: HTMLOptionElement;
    readonly sheetEl: HTMLElement;
}

export interface ModuleTabManager {
    readonly syncModuleTabs: () => string[];
    readonly clearModuleTabs: () => void;
    readonly getModuleArea: (sheetId: string) => HTMLElement | null;
    readonly getSheets: () => ModuleSheet[];
    readonly setSearchFilter: (filter: string) => void;
}

export interface ModuleTabManagerOptions {
    readonly selectEl: HTMLSelectElement;
    readonly sheetContainerEl: HTMLElement;
    readonly onWordClick: (word: string) => void;
    readonly onBackgroundClick: () => void;
    readonly onBackgroundDoubleClick: () => void;
    readonly onSheetChange: (sheetId: string) => void;
    readonly onSearchInput: (filter: string) => void;
    readonly onUpdateDisplays?: () => void;
    readonly onSaveState?: () => Promise<void>;
    readonly showInfo?: (msg: string, clear: boolean) => void;
}

export const createModuleTabManager = (
    options: ModuleTabManagerOptions
): ModuleTabManager => {
    const {
        selectEl,
        sheetContainerEl,
        onWordClick,
        onBackgroundClick,
        onBackgroundDoubleClick,
    } = options;

    const sheets: ModuleSheet[] = [];
    let searchFilter = '';

    const createOption = (moduleName: string, sheetId: string): HTMLOptionElement => {
        const option = document.createElement('option');
        option.value = sheetId;
        option.textContent = `${moduleName} word`;
        return option;
    };

    const createSheetElement = (sheetId: string): HTMLElement => {
        const sheet = document.createElement('div');
        sheet.className = 'dictionary-sheet';
        sheet.id = `dictionary-sheet-${sheetId}`;
        sheet.style.display = 'none';

        const wordInfoDisplay = document.createElement('span');
        wordInfoDisplay.className = 'word-info-display module-word-info';

        const wordsArea = document.createElement('div');
        wordsArea.className = 'words-area';
        wordsArea.appendChild(wordInfoDisplay);

        const wordsDisplay = document.createElement('div');
        wordsDisplay.className = 'words-display module-words-display';
        wordsArea.appendChild(wordsDisplay);
        setupBackgroundClickHandlers(wordsDisplay, onBackgroundClick, onBackgroundDoubleClick);

        const container = document.createElement('div');
        container.className = 'vocabulary-container';
        container.appendChild(wordsArea);

        sheet.appendChild(container);

        return sheet;
    };

    const renderModuleWords = (moduleSheet: ModuleSheet): void => {
        if (!window.ajisaiInterpreter) return;

        const wordsDisplay = moduleSheet.sheetEl.querySelector('.module-words-display');
        const wordInfo = moduleSheet.sheetEl.querySelector('.module-word-info');
        if (!wordsDisplay || !wordInfo) return;

        wordsDisplay.innerHTML = '';

        try {
            const moduleWords: Array<[string, string | null]> =
                window.ajisaiInterpreter.get_module_words_info(moduleSheet.moduleName);

            const sorted = [...moduleWords].sort((a, b) => sortWordName(a[0], b[0]));
            const matched = sorted.filter(wd => matchesFilter(wd[0], searchFilter));
            const prefix = `${moduleSheet.moduleName}::`;

            matched.forEach(wordData => {
                const name = wordData[0];
                const shortName = name.startsWith(prefix) ? name.slice(prefix.length) : name;
                const description = wordData[1] || name;

                const button = createWordButton(
                    shortName,
                    description,
                    'word-button module',
                    () => onWordClick(shortName),
                    () => { (wordInfo as HTMLElement).textContent = description; },
                    () => { (wordInfo as HTMLElement).textContent = ''; }
                );

                wordsDisplay.appendChild(button);
            });

            if (searchFilter && matched.length === 0) {
                wordsDisplay.classList.add('is-empty');
                wordsDisplay.appendChild(createNoResultsMessage());
                return;
            }

            if (!searchFilter && sorted.length === 0) {
                wordsDisplay.classList.add('is-empty');
                wordsDisplay.appendChild(createEmptyWordsMessage('No words available in this module.'));
                return;
            }

            wordsDisplay.classList.toggle('is-empty', matched.length === 0);
        } catch (error) {
            console.error(`Failed to render module words for ${moduleSheet.moduleName}:`, error);
        }
    };

    const findSheet = (moduleName: string): ModuleSheet | undefined =>
        sheets.find(s => s.moduleName === moduleName);

    const syncModuleTabs = (): string[] => {
        if (!window.ajisaiInterpreter) return [];

        const newSheetIds: string[] = [];

        try {
            const importedModules: string[] = window.ajisaiInterpreter.get_imported_modules();
            const importedSet = new Set(importedModules);

            for (let i = sheets.length - 1; i >= 0; i--) {
                const sheet = sheets[i]!;
                if (!importedSet.has(sheet.moduleName)) {
                    sheet.optionEl.remove();
                    sheet.sheetEl.remove();
                    sheets.splice(i, 1);
                }
            }

            for (const moduleName of importedModules) {
                if (!findSheet(moduleName)) {
                    const sheetId = `module-${moduleName}`;
                    const optionEl = createOption(moduleName, sheetId);
                    const sheetEl = createSheetElement(sheetId);

                    selectEl.appendChild(optionEl);
                    sheetContainerEl.appendChild(sheetEl);

                    const moduleSheet: ModuleSheet = { moduleName, sheetId, optionEl, sheetEl };
                    sheets.push(moduleSheet);
                    newSheetIds.push(sheetId);
                }
            }

            for (const sheet of sheets) {
                renderModuleWords(sheet);
            }
        } catch (error) {
            console.error('Failed to sync module sheets:', error);
        }

        return newSheetIds;
    };

    const clearModuleTabs = (): void => {
        for (const sheet of sheets) {
            sheet.optionEl.remove();
            sheet.sheetEl.remove();
        }
        sheets.length = 0;
    };

    const getModuleSheet = (sheetId: string): HTMLElement | null => {
        const sheet = sheets.find(s => s.sheetId === sheetId);
        return sheet?.sheetEl ?? null;
    };

    const getSheets = (): ModuleSheet[] => sheets;

    const setSearchFilter = (filter: string): void => {
        searchFilter = filter.trim();
        for (const sheet of sheets) {
            renderModuleWords(sheet);
        }
    };

    return {
        syncModuleTabs,
        clearModuleTabs,
        getModuleArea: getModuleSheet,
        getSheets,
        setSearchFilter,
    };
};
