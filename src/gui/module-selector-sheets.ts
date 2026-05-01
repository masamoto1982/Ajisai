

import {
    createEmptyWordsElement,
    createNoResultsElement,
    createWordButtonElement,
    checkWordMatchesFilter,
    registerBackgroundClickListeners,
    compareWordName,
    renderWordInfo,
    resetWordInfoDisplay,
} from './dictionary-element-builders';
import { formatDictionaryTabName } from './vocabulary-state-controller';

export interface ModuleSheet {
    readonly moduleName: string;
    readonly sheetId: string;
    readonly optionEl: HTMLOptionElement;
    readonly sheetEl: HTMLElement;
}

export interface ModuleTabManager {
    readonly syncModuleTabs: () => string[];
    readonly clearModuleTabs: () => void;
    readonly lookupModuleArea: (sheetId: string) => HTMLElement | null;
    readonly collectSheets: () => ModuleSheet[];
    readonly updateSearchFilter: (filter: string) => void;
}

export interface ModuleActionConfig {
    readonly label: string;
    readonly className: string;
    readonly ariaLabel: string;
    readonly onClick: () => void;
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
    readonly moduleActions?: Record<string, readonly ModuleActionConfig[]>;
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

    const createOptionElement = (moduleName: string, sheetId: string): HTMLOptionElement => {
        const option = document.createElement('option');
        option.value = sheetId;
        option.textContent = formatDictionaryTabName(moduleName);
        return option;
    };

    const createSheetElement = (sheetId: string, moduleName: string): HTMLElement => {
        const sheet = document.createElement('div');
        sheet.className = 'dictionary-sheet';
        sheet.id = `dictionary-sheet-${sheetId}`;
        sheet.hidden = true;

        const wordInfoDisplay = document.createElement('span');
        wordInfoDisplay.className = 'word-info-display module-word-info';
        resetWordInfoDisplay(wordInfoDisplay);
        sheet.appendChild(wordInfoDisplay);

        const wordsDisplay = document.createElement('div');
        wordsDisplay.className = 'words-display module-words-display';
        sheet.appendChild(wordsDisplay);
        registerBackgroundClickListeners(wordsDisplay, onBackgroundClick, onBackgroundDoubleClick);

        const actions = options.moduleActions?.[moduleName];
        if (actions) {
            const actionsDiv = document.createElement('div');
            actionsDiv.className = 'vocabulary-actions';
            for (const action of actions) {
                const btn = document.createElement('button');
                btn.type = 'button';
                btn.className = `header-btn ${action.className}`;
                btn.setAttribute('aria-label', action.ariaLabel);
                btn.textContent = action.label;
                btn.addEventListener('click', action.onClick);
                actionsDiv.appendChild(btn);
            }
            sheet.appendChild(actionsDiv);
        }

        return sheet;
    };

    const renderModuleWords = (moduleSheet: ModuleSheet): void => {
        if (!window.ajisaiInterpreter) return;

        const wordsDisplay = moduleSheet.sheetEl.querySelector('.module-words-display');
        const wordInfo = moduleSheet.sheetEl.querySelector('.module-word-info');
        if (!wordsDisplay || !wordInfo) return;

        wordsDisplay.innerHTML = '';
        resetWordInfoDisplay(wordInfo as HTMLElement);

        try {
            const moduleWords: Array<[string, string | null]> =
                window.ajisaiInterpreter.collect_module_words_info(moduleSheet.moduleName);

            const sorted = [...moduleWords].sort((a, b) => compareWordName(a[0], b[0]));
            const matched = sorted.filter(wd => checkWordMatchesFilter(wd[0], searchFilter));
            const prefix = `${moduleSheet.moduleName}@`;

            matched.forEach(wordData => {
                const name = wordData[0];
                const shortName = name.startsWith(prefix) ? name.slice(prefix.length) : name;
                const description = wordData[1] || name;

                const button = createWordButtonElement(
                    shortName,
                    description,
                    'word-button module',
                    () => onWordClick(shortName),
                    () => { renderWordInfo(wordInfo as HTMLElement, description); },
                    () => { resetWordInfoDisplay(wordInfo as HTMLElement); }
                );

                wordsDisplay.appendChild(button);
            });

            if (searchFilter && matched.length === 0) {
                wordsDisplay.classList.add('is-empty');
                wordsDisplay.appendChild(createNoResultsElement());
                return;
            }

            if (!searchFilter && sorted.length === 0) {
                wordsDisplay.classList.add('is-empty');
                wordsDisplay.appendChild(createEmptyWordsElement('No words available in this module.'));
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
            const importedModules: string[] = window.ajisaiInterpreter.collect_imported_modules();
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
                    const optionEl = createOptionElement(moduleName, sheetId);
                    const sheetEl = createSheetElement(sheetId, moduleName);

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

    const lookupModuleSheet = (sheetId: string): HTMLElement | null => {
        const sheet = sheets.find(s => s.sheetId === sheetId);
        return sheet?.sheetEl ?? null;
    };

    const collectSheets = (): ModuleSheet[] => sheets;

    const updateSearchFilter = (filter: string): void => {
        searchFilter = filter.trim();
        for (const sheet of sheets) {
            renderModuleWords(sheet);
        }
    };

    return {
        syncModuleTabs,
        clearModuleTabs,
        lookupModuleArea: lookupModuleSheet,
        collectSheets,
        updateSearchFilter,
    };
};
