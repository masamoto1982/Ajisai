

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
import type { DictionarySheetSelector, SelectorEntry } from './dictionary-sheet-selector';

export interface ModuleSheet {
    readonly moduleName: string;
    readonly sheetId: string;
    readonly sheetEl: HTMLElement;
}

export interface ModuleTabManager {
    readonly syncModuleTabs: () => void;
    readonly lookupModuleArea: (sheetId: string) => HTMLElement | null;
    readonly collectSheets: () => ModuleSheet[];
    readonly updateSearchFilter: (filter: string) => void;
    readonly toggleModule: (moduleName: string, currentlyActive: boolean) => void;
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
    // Native popover: the browser puts it in the top layer and light-dismisses it
    // on outside click / Escape, so no document-level listeners are needed here.
    menu.popover = 'auto';
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
            if (menu.matches(':popover-open')) menu.hidePopover();
            if (!action.disabled) action.onClick();
        });
        menu.appendChild(button);
    }
    menu.style.left = `${event.clientX}px`;
    menu.style.top = `${event.clientY}px`;
    if (menu.matches(':popover-open')) menu.hidePopover();
    menu.showPopover();
};

const quoteAjisaiString = (value: string): string => value.replace(/\\/g, '\\\\').replace(/'/g, "\\'");

export interface ModuleTabManagerOptions {
    readonly selector: DictionarySheetSelector;
    readonly sheetContainerEl: HTMLElement;
    readonly onWordClick: (word: string) => void;
    readonly onBackgroundClick: () => void;
    readonly onBackgroundDoubleClick: () => void;
    readonly onUpdateDisplays?: () => void;
    readonly onSaveState?: () => Promise<void>;
    readonly showInfo?: (msg: string, clear: boolean) => void;
    readonly revealSheet?: (sheetId: string) => void;
    readonly moduleActions?: Record<string, readonly ModuleActionConfig[]>;
}

export const createModuleTabManager = (
    options: ModuleTabManagerOptions
): ModuleTabManager => {
    const {
        selector,
        sheetContainerEl,
        onWordClick,
        onBackgroundClick,
        onBackgroundDoubleClick,
        onUpdateDisplays,
        onSaveState,
        showInfo,
        revealSheet,
    } = options;

    const sheets: ModuleSheet[] = [];
    let searchFilter = '';
    const contextMenu = createContextMenuElement();

    // Activation toggles are intentionally implemented as real IMPORT /
    // UNIMPORT / IMPORT-ONLY / UNIMPORT-ONLY executions so the GUI gesture is
    // semantically identical to typing the word (SPECIFICATION.html §9.2).
    const runModuleCode = async (code: string, successMessage: string): Promise<void> => {
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
            const message = `Failed to update module state: ${error}`;
            showInfo?.(message, true);
            alert(message);
        }
    };

    const importModule = (moduleName: string): void => {
        const quoted = quoteAjisaiString(moduleName);
        void runModuleCode(`'${quoted}' IMPORT`, `Imported ${moduleName}`);
    };

    const unimportModule = (moduleName: string): void => {
        const quoted = quoteAjisaiString(moduleName);
        void runModuleCode(`'${quoted}' UNIMPORT`, `Unimported unused words from ${moduleName}`);
    };

    const importModuleWord = (moduleName: string, shortName: string): void => {
        const quotedModule = quoteAjisaiString(moduleName);
        const quotedWord = quoteAjisaiString(shortName);
        void runModuleCode(
            `'${quotedModule}' [ '${quotedWord}' ] IMPORT-ONLY`,
            `Imported ${moduleName}@${shortName}`
        );
    };

    const unimportModuleWord = (moduleName: string, shortName: string): void => {
        const quotedModule = quoteAjisaiString(moduleName);
        const quotedWord = quoteAjisaiString(shortName);
        void runModuleCode(
            `'${quotedModule}' [ '${quotedWord}' ] UNIMPORT-ONLY`,
            `Unimported ${moduleName}@${shortName}`
        );
    };

    const toggleModule = (moduleName: string, currentlyActive: boolean): void => {
        if (currentlyActive) {
            unimportModule(moduleName);
        } else {
            importModule(moduleName);
        }
        revealSheet?.(`module-${moduleName}`);
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
        wordsDisplay.addEventListener('contextmenu', (event) => {
            if ((event.target as HTMLElement).closest('.word-button')) return;
            event.preventDefault();
            const active = isModuleActive(moduleName);
            renderContextMenu(contextMenu, event, [active ? {
                label: `Unimport this module (${moduleName})`,
                onClick: () => unimportModule(moduleName),
            } : {
                label: `Import this module (${moduleName})`,
                onClick: () => importModule(moduleName),
            }]);
        });

        const actions = options.moduleActions?.[moduleName];
        if (actions) {
            const actionsDiv = document.createElement('div');
            actionsDiv.className = 'vocabulary-actions';
            for (const action of actions) {
                const btn = document.createElement('button');
                btn.type = 'button';
                btn.className = action.className;
                btn.setAttribute('aria-label', action.ariaLabel);
                btn.textContent = action.label;
                btn.addEventListener('click', action.onClick);
                actionsDiv.appendChild(btn);
            }
            sheet.appendChild(actionsDiv);
        }

        return sheet;
    };

    const isModuleActive = (moduleName: string): boolean => {
        if (!window.ajisaiInterpreter) return false;
        return window.ajisaiInterpreter.collect_imported_modules().includes(moduleName);
    };

    const renderModuleWords = (moduleSheet: ModuleSheet): void => {
        if (!window.ajisaiInterpreter) return;

        const wordsDisplay = moduleSheet.sheetEl.querySelector('.module-words-display');
        const wordInfo = moduleSheet.sheetEl.querySelector('.module-word-info');
        if (!wordsDisplay || !wordInfo) return;

        wordsDisplay.innerHTML = '';
        resetWordInfoDisplay(wordInfo as HTMLElement);

        try {
            // Full catalog (active + inactive) so inactive words render greyed
            // and can be activated with a long-press, not just the imported set.
            const catalog: Array<[string, string, boolean]> =
                window.ajisaiInterpreter.collect_module_catalog_words_info(moduleSheet.moduleName);

            const sorted = [...catalog].sort((a, b) => compareWordName(a[0], b[0]));
            const matched = sorted.filter(wd => checkWordMatchesFilter(wd[0], searchFilter));

            const fragment = document.createDocumentFragment();
            matched.forEach(wordData => {
                const shortName = wordData[0];
                const description = wordData[1] || shortName;
                const imported = wordData[2];
                const className = `word-button core module${imported ? '' : ' is-inactive'}`;
                const button = createWordButtonElement(
                    shortName,
                    className,
                    () => onWordClick(shortName),
                    () => { renderWordInfo(wordInfo as HTMLElement, description); },
                    () => { resetWordInfoDisplay(wordInfo as HTMLElement); },
                    (event) => renderContextMenu(contextMenu, event, imported ? [{
                        label: `Unimport ${moduleSheet.moduleName}@${shortName}`,
                        onClick: () => unimportModuleWord(moduleSheet.moduleName, shortName),
                    }, {
                        label: `Unimport this module (${moduleSheet.moduleName})`,
                        onClick: () => unimportModule(moduleSheet.moduleName),
                    }] : [{
                        label: `Import ${moduleSheet.moduleName}@${shortName}`,
                        onClick: () => importModuleWord(moduleSheet.moduleName, shortName),
                    }, {
                        label: `Import this module (${moduleSheet.moduleName})`,
                        onClick: () => importModule(moduleSheet.moduleName),
                    }]),
                    () => {
                        if (imported) {
                            unimportModuleWord(moduleSheet.moduleName, shortName);
                        } else {
                            importModuleWord(moduleSheet.moduleName, shortName);
                        }
                    }
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

    const buildSelectorEntries = (moduleNames: string[]): SelectorEntry[] => {
        const importedSet = new Set(
            window.ajisaiInterpreter?.collect_imported_modules() ?? []
        );
        const entries: SelectorEntry[] = [
            { sheetId: 'core', label: formatDictionaryTabName('CORE'), kind: 'core' },
            { sheetId: 'user', label: formatDictionaryTabName('USER'), kind: 'user' },
        ];
        for (const moduleName of moduleNames) {
            entries.push({
                sheetId: `module-${moduleName}`,
                label: formatDictionaryTabName(moduleName),
                kind: 'module',
                moduleName,
                active: importedSet.has(moduleName),
            });
        }
        return entries;
    };

    const syncModuleTabs = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            // Every importable module gets a persistent sheet so inactive
            // candidates are browsable and toggleable, not just imported ones.
            const available: string[] = window.ajisaiInterpreter.collect_available_modules();

            for (const moduleName of available) {
                if (!findSheet(moduleName)) {
                    const sheetId = `module-${moduleName}`;
                    const sheetEl = createSheetElement(sheetId, moduleName);
                    sheetContainerEl.appendChild(sheetEl);
                    sheets.push({ moduleName, sheetId, sheetEl });
                }
            }

            for (const sheet of sheets) {
                renderModuleWords(sheet);
            }

            selector.setEntries(buildSelectorEntries(available));
        } catch (error) {
            console.error('Failed to sync module sheets:', error);
        }
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
        lookupModuleArea: lookupModuleSheet,
        collectSheets,
        updateSearchFilter,
        toggleModule,
    };
};
