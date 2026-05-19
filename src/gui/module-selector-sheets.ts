

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

const AVAILABLE_MODULE_NAMES: readonly string[] = [
    'MUSIC',
    'JSON',
    'IO',
    'TIME',
    'CRYPTO',
    'ALGO',
    'MATH',
];

type ModuleSheetState = 'imported' | 'available';

export interface ModuleSheet {
    readonly moduleName: string;
    readonly sheetId: string;
    readonly optionEl: HTMLOptionElement;
    readonly sheetEl: HTMLElement;
    readonly state: ModuleSheetState;
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


type ContextMenuAction = {
    readonly label: string;
    readonly onClick: () => void;
    readonly disabled?: boolean;
};

const createContextMenuElement = (): HTMLDivElement => {
    const menu = document.createElement('div');
    menu.hidden = true;
    menu.className = 'context-menu module-context-menu';
    document.body.appendChild(menu);
    return menu;
};

const renderContextMenu = (
    menu: HTMLDivElement,
    event: MouseEvent,
    actions: readonly ContextMenuAction[]
): void => {
    menu.innerHTML = '';
    for (const action of actions) {
        const button = document.createElement('button');
        button.type = 'button';
        button.textContent = action.label;
        button.disabled = Boolean(action.disabled);
        button.addEventListener('click', (clickEvent) => {
            clickEvent.stopPropagation();
            menu.hidden = true;
            if (!action.disabled) action.onClick();
        });
        menu.appendChild(button);
    }
    menu.hidden = false;
    menu.style.left = `${event.clientX}px`;
    menu.style.top = `${event.clientY}px`;
};

const quoteAjisaiString = (value: string): string => value.replace(/\\/g, '\\\\').replace(/'/g, "\\'");

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
        onUpdateDisplays,
        onSaveState,
        showInfo,
    } = options;

    const sheets: ModuleSheet[] = [];
    let searchFilter = '';
    const contextMenu = createContextMenuElement();

    const hideContextMenu = (): void => {
        contextMenu.hidden = true;
    };

    document.addEventListener('click', hideContextMenu);
    window.addEventListener('blur', hideContextMenu);

    const runModuleMutationCode = async (
        code: string,
        successMessage: string,
        failurePrefix: string
    ): Promise<void> => {
        if (!window.ajisaiInterpreter) return;
        try {
            const result = await window.ajisaiInterpreter.execute(code);
            if (result.status === 'ERROR') {
                const message = result.message ?? 'Unknown error';
                showInfo?.(message, true);
                alert(message);
                return;
            }
            onUpdateDisplays?.();
            await onSaveState?.();
            showInfo?.(successMessage, true);
        } catch (error) {
            const message = `${failurePrefix}: ${error}`;
            showInfo?.(message, true);
            alert(message);
        }
    };

    const importModule = (moduleName: string): void => {
        const quotedModule = quoteAjisaiString(moduleName);
        void runModuleMutationCode(
            `'${quotedModule}' IMPORT`,
            `Imported ${moduleName}`,
            'Failed to import module'
        );
    };

    const unimportModule = (moduleName: string): void => {
        const quotedModule = quoteAjisaiString(moduleName);
        void runModuleMutationCode(
            `'${quotedModule}' UNIMPORT`,
            `Unimported unused words from ${moduleName}`,
            'Failed to unimport module item'
        );
    };

    const unimportModuleWord = (moduleName: string, shortName: string): void => {
        const quotedModule = quoteAjisaiString(moduleName);
        const quotedWord = quoteAjisaiString(shortName);
        void runModuleMutationCode(
            `'${quotedModule}' [ '${quotedWord}' ] UNIMPORT-ONLY`,
            `Unimported ${moduleName}@${shortName}`,
            'Failed to unimport module item'
        );
    };

    const createOptionElement = (
        moduleName: string,
        sheetId: string,
        state: ModuleSheetState
    ): HTMLOptionElement => {
        const option = document.createElement('option');
        option.value = sheetId;
        option.textContent = formatDictionaryTabName(moduleName);
        option.dataset.moduleName = moduleName;
        option.dataset.moduleState = state;
        if (state === 'available') {
            option.className = 'module-option available-module-option';
            option.title = `${moduleName} is available. Select it and right-click this selector to import.`;
            option.style.opacity = '0.58';
        }
        return option;
    };

    const createAvailableSheetElement = (sheetId: string, moduleName: string): HTMLElement => {
        const sheet = document.createElement('div');
        sheet.className = 'dictionary-sheet module-sheet module-sheet-available';
        sheet.id = `dictionary-sheet-${sheetId}`;
        sheet.hidden = true;
        sheet.appendChild(createEmptyWordsElement(
            `${moduleName} is available but not imported. Select this module in the dictionary selector and right-click the selector to import it.`
        ));
        return sheet;
    };

    const createImportedSheetElement = (sheetId: string, moduleName: string): HTMLElement => {
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
        wordsDisplay.addEventListener('contextmenu', (event) => {
            if ((event.target as HTMLElement).closest('.word-button')) return;
            event.preventDefault();
            renderContextMenu(contextMenu, event, [{
                label: `Unimport this module (${moduleName})`,
                onClick: () => unimportModule(moduleName),
            }]);
        });

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

    const createSheetElement = (
        sheetId: string,
        moduleName: string,
        state: ModuleSheetState
    ): HTMLElement => state === 'imported'
        ? createImportedSheetElement(sheetId, moduleName)
        : createAvailableSheetElement(sheetId, moduleName);

    const renderModuleWords = (moduleSheet: ModuleSheet): void => {
        if (moduleSheet.state !== 'imported') return;
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

            const fragment = document.createDocumentFragment();
            matched.forEach(wordData => {
                const name = wordData[0];
                const shortName = name.startsWith(prefix) ? name.slice(prefix.length) : name;
                const description = wordData[1] || name;
                const moduleTitle = `${shortName}\nBuilt-in word from module ${moduleSheet.moduleName}.\nRight-click to unimport this word.`;
                const moduleInfo = `${description}\n\nBuilt-in word from module ${moduleSheet.moduleName}.\nRight-click to unimport this word.`;
                const button = createWordButtonElement(
                    shortName,
                    moduleTitle,
                    `word-button core module`,
                    () => onWordClick(shortName),
                    () => { renderWordInfo(wordInfo as HTMLElement, moduleInfo); },
                    () => { resetWordInfoDisplay(wordInfo as HTMLElement); },
                    (event) => renderContextMenu(contextMenu, event, [{
                        label: `Unimport ${moduleSheet.moduleName}@${shortName}`,
                        onClick: () => unimportModuleWord(moduleSheet.moduleName, shortName),
                    }, {
                        label: `Unimport this module (${moduleSheet.moduleName})`,
                        onClick: () => unimportModule(moduleSheet.moduleName),
                    }])
                );

                fragment.appendChild(button);
            });
            wordsDisplay.appendChild(fragment);

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

    const removeSheet = (sheet: ModuleSheet, index: number): void => {
        sheet.optionEl.remove();
        sheet.sheetEl.remove();
        sheets.splice(index, 1);
    };

    const syncModuleTabs = (): string[] => {
        if (!window.ajisaiInterpreter) return [];

        const newSheetIds: string[] = [];

        try {
            const importedModules: string[] = window.ajisaiInterpreter.collect_imported_modules();
            const importedSet = new Set(importedModules);
            const knownModules = new Set([...AVAILABLE_MODULE_NAMES, ...importedModules]);

            for (let i = sheets.length - 1; i >= 0; i--) {
                const sheet = sheets[i]!;
                const shouldBeImported = importedSet.has(sheet.moduleName);
                const desiredState: ModuleSheetState = shouldBeImported ? 'imported' : 'available';
                if (!knownModules.has(sheet.moduleName) || sheet.state !== desiredState) {
                    removeSheet(sheet, i);
                }
            }

            for (const moduleName of knownModules) {
                if (findSheet(moduleName)) continue;

                const state: ModuleSheetState = importedSet.has(moduleName) ? 'imported' : 'available';
                const sheetId = `${state === 'imported' ? 'module' : 'module-available'}-${moduleName}`;
                const optionEl = createOptionElement(moduleName, sheetId, state);
                const sheetEl = createSheetElement(sheetId, moduleName, state);

                selectEl.appendChild(optionEl);
                sheetContainerEl.appendChild(sheetEl);

                const moduleSheet: ModuleSheet = { moduleName, sheetId, optionEl, sheetEl, state };
                sheets.push(moduleSheet);
                if (state === 'imported') {
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

    selectEl.addEventListener('contextmenu', (event) => {
        const selectedOption = selectEl.selectedOptions[0];
        const selectedModuleName = selectedOption?.dataset.moduleName;
        const selectedState = selectedOption?.dataset.moduleState as ModuleSheetState | undefined;
        if (!selectedModuleName || !selectedState) return;

        event.preventDefault();
        if (selectedState === 'available') {
            renderContextMenu(contextMenu, event, [{
                label: `Import this module (${selectedModuleName})`,
                onClick: () => importModule(selectedModuleName),
            }]);
            return;
        }

        renderContextMenu(contextMenu, event, [{
            label: `Unimport this module (${selectedModuleName})`,
            onClick: () => unimportModule(selectedModuleName),
        }]);
    });

    return {
        syncModuleTabs,
        clearModuleTabs,
        lookupModuleArea: lookupModuleSheet,
        collectSheets,
        updateSearchFilter,
    };
};
