// js/gui/module-tabs.ts

import type { ViewMode } from './mobile';
import {
    createNoResultsMessage,
    createWordButton,
    matchesFilter,
    setupBackgroundClickHandlers,
    sortWordName,
} from './dictionary-ui';

export interface ModuleTab {
    readonly moduleName: string;
    readonly viewMode: ViewMode;
    readonly tabBtn: HTMLElement;
    readonly areaEl: HTMLElement;
}

export interface ModuleTabManager {
    readonly syncModuleTabs: () => ViewMode[];
    readonly clearModuleTabs: () => void;
    readonly getModuleArea: (viewMode: string) => HTMLElement | null;
    readonly getTabs: () => ModuleTab[];
    readonly setSearchFilter: (filter: string) => void;
}

export interface ModuleTabManagerOptions {
    readonly tabGroupEl: HTMLElement;
    readonly areaContainerEl: HTMLElement;
    readonly onWordClick: (word: string) => void;
    readonly onBackgroundClick: () => void;
    readonly onBackgroundDoubleClick: () => void;
    readonly onTabClick: (mode: ViewMode) => void;
    readonly onSearchInput: (filter: string) => void;
    readonly coreTabBtn: HTMLElement;
    readonly onUpdateDisplays?: () => void;
    readonly onSaveState?: () => Promise<void>;
    readonly showInfo?: (msg: string, clear: boolean) => void;
}

export const createModuleTabManager = (
    options: ModuleTabManagerOptions
): ModuleTabManager => {
    const {
        tabGroupEl,
        areaContainerEl,
        onWordClick,
        onBackgroundClick,
        onBackgroundDoubleClick,
        onTabClick,
        onSearchInput,
    } = options;

    const tabs: ModuleTab[] = [];
    let searchFilter = '';

    const createTabButton = (moduleName: string, viewMode: ViewMode): HTMLElement => {
        const btn = document.createElement('button');
        btn.className = 'area-tab';
        btn.type = 'button';
        btn.role = 'tab';
        btn.setAttribute('data-mode', viewMode);
        btn.setAttribute('aria-selected', 'false');
        btn.setAttribute('tabindex', '-1');
        btn.textContent = moduleName;

        btn.addEventListener('click', () => onTabClick(viewMode));
        btn.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onTabClick(viewMode);
            }
        });

        return btn;
    };

    const createAreaElement = (moduleName: string): HTMLElement => {
        const section = document.createElement('section');
        section.className = 'vocabulary-area module-tab-area';
        section.setAttribute('data-module', moduleName);
        section.setAttribute('role', 'tabpanel');
        section.setAttribute('tabindex', '0');
        section.style.display = 'none';

        const header = document.createElement('div');
        header.className = 'vocabulary-header';
        const searchWrapper = document.createElement('div');
        searchWrapper.className = 'search-wrapper';
        const searchInput = document.createElement('input');
        searchInput.type = 'text';
        searchInput.className = 'vocabulary-search-input module-search-input';
        searchInput.placeholder = 'Search words...';
        searchInput.setAttribute('aria-label', `Search ${moduleName} words`);
        const clearBtn = document.createElement('button');
        clearBtn.type = 'button';
        clearBtn.className = 'inline-clear-btn vocabulary-search-clear-btn';
        clearBtn.setAttribute('aria-label', 'Clear search');
        clearBtn.textContent = '×';
        searchWrapper.appendChild(searchInput);
        searchWrapper.appendChild(clearBtn);
        header.appendChild(searchWrapper);

        searchInput.addEventListener('input', () => onSearchInput(searchInput.value));
        clearBtn.addEventListener('click', () => {
            searchInput.value = '';
            onSearchInput('');
        });

        const wordInfoDisplay = document.createElement('span');
        wordInfoDisplay.className = 'word-info-display module-word-info';

        const wordsArea = document.createElement('div');
        wordsArea.className = 'core-words-area';
        wordsArea.appendChild(wordInfoDisplay);

        const wordsDisplay = document.createElement('div');
        wordsDisplay.className = 'words-display module-words-display';
        wordsArea.appendChild(wordsDisplay);
        setupBackgroundClickHandlers(wordsDisplay, onBackgroundClick, onBackgroundDoubleClick);

        const container = document.createElement('div');
        container.className = 'vocabulary-container';
        container.appendChild(wordsArea);

        section.appendChild(header);
        section.appendChild(container);

        return section;
    };

    const renderModuleWords = (tab: ModuleTab): void => {
        if (!window.ajisaiInterpreter) return;

        const wordsDisplay = tab.areaEl.querySelector('.module-words-display');
        const wordInfo = tab.areaEl.querySelector('.module-word-info');
        if (!wordsDisplay || !wordInfo) return;

        wordsDisplay.innerHTML = '';

        try {
            const moduleWords: Array<[string, string | null]> =
                window.ajisaiInterpreter.get_module_words_info(tab.moduleName);

            const sorted = [...moduleWords].sort((a, b) => sortWordName(a[0], b[0]));
            const matched = sorted.filter(wd => matchesFilter(wd[0], searchFilter));
            const prefix = `${tab.moduleName}::`;

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
                wordsDisplay.appendChild(createNoResultsMessage());
            }
        } catch (error) {
            console.error(`Failed to render module words for ${tab.moduleName}:`, error);
        }
    };

    const findTab = (moduleName: string): ModuleTab | undefined =>
        tabs.find(t => t.moduleName === moduleName);

    const syncSearchInputValues = (): void => {
        for (const tab of tabs) {
            const input = tab.areaEl.querySelector('.module-search-input') as HTMLInputElement | null;
            if (input && input.value !== searchFilter) {
                input.value = searchFilter;
            }
        }
    };

    const syncModuleTabs = (): ViewMode[] => {
        if (!window.ajisaiInterpreter) return [];

        const newViewModes: ViewMode[] = [];

        try {
            const importedModules: string[] = window.ajisaiInterpreter.get_imported_modules();
            const importedSet = new Set(importedModules);

            for (let i = tabs.length - 1; i >= 0; i--) {
                const tab = tabs[i]!;
                if (!importedSet.has(tab.moduleName)) {
                    tab.tabBtn.remove();
                    tab.areaEl.remove();
                    tabs.splice(i, 1);
                }
            }

            for (const moduleName of importedModules) {
                if (!findTab(moduleName)) {
                    const viewMode: ViewMode = `module:${moduleName}`;
                    const tabBtn = createTabButton(moduleName, viewMode);
                    const areaEl = createAreaElement(moduleName);

                    tabGroupEl.appendChild(tabBtn);
                    areaContainerEl.appendChild(areaEl);

                    const tab: ModuleTab = { moduleName, viewMode, tabBtn, areaEl };
                    tabs.push(tab);
                    newViewModes.push(viewMode);
                }
            }

            for (const tab of tabs) {
                renderModuleWords(tab);
            }
        } catch (error) {
            console.error('Failed to sync module tabs:', error);
        }

        return newViewModes;
    };

    const clearModuleTabs = (): void => {
        for (const tab of tabs) {
            tab.tabBtn.remove();
            tab.areaEl.remove();
        }
        tabs.length = 0;
    };

    const getModuleArea = (viewMode: string): HTMLElement | null => {
        const tab = tabs.find(t => t.viewMode === viewMode);
        return tab?.areaEl ?? null;
    };

    const getTabs = (): ModuleTab[] => tabs;

    const setSearchFilter = (filter: string): void => {
        searchFilter = filter.trim();
        syncSearchInputValues();
        for (const tab of tabs) {
            renderModuleWords(tab);
        }
    };

    return {
        syncModuleTabs,
        clearModuleTabs,
        getModuleArea,
        getTabs,
        setSearchFilter,
    };
};
