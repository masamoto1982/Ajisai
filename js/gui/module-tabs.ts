// js/gui/module-tabs.ts

import type { ViewMode } from './mobile';

export interface ModuleTab {
    readonly moduleName: string;   // 'MUSIC', 'JSON', 'IO' etc.
    readonly viewMode: ViewMode;   // 'module:MUSIC' etc.
    readonly tabBtn: HTMLElement;
    readonly areaEl: HTMLElement;
}

export interface ModuleTabManager {
    readonly syncModuleTabs: () => void;
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
    readonly dictionaryTabBtn: HTMLElement;
}

const sortWordName = (a: string, b: string): number => {
    const aIsAlpha = /^[A-Za-z]/.test(a);
    const bIsAlpha = /^[A-Za-z]/.test(b);
    if (!aIsAlpha && bIsAlpha) return -1;
    if (aIsAlpha && !bIsAlpha) return 1;
    return a.localeCompare(b);
};

const matchesFilter = (wordName: string, filter: string): boolean => {
    if (!filter) return true;
    return wordName.toLowerCase().includes(filter.toLowerCase());
};

export const createModuleTabManager = (
    options: ModuleTabManagerOptions
): ModuleTabManager => {
    const {
        tabGroupEl, areaContainerEl, onWordClick,
        onBackgroundClick, onBackgroundDoubleClick,
        onTabClick, onSearchInput
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

    const isBackgroundClick = (e: MouseEvent): boolean => {
        const target = e.target as HTMLElement;
        return !target.closest('.word-button');
    };

    const setupBackgroundClickHandlers = (wordsDisplay: HTMLElement): void => {
        let clickTimer: ReturnType<typeof setTimeout> | null = null;

        wordsDisplay.addEventListener('click', (e) => {
            if (isBackgroundClick(e as MouseEvent)) {
                if (clickTimer) clearTimeout(clickTimer);
                clickTimer = setTimeout(() => {
                    onBackgroundClick();
                    clickTimer = null;
                }, 200);
            }
        });

        wordsDisplay.addEventListener('dblclick', (e) => {
            if (isBackgroundClick(e as MouseEvent)) {
                if (clickTimer) {
                    clearTimeout(clickTimer);
                    clickTimer = null;
                }
                onBackgroundDoubleClick();
            }
        });
    };

    const createAreaElement = (moduleName: string): HTMLElement => {
        const section = document.createElement('section');
        section.className = 'dictionary-area module-tab-area';
        section.setAttribute('data-module', moduleName);
        section.setAttribute('role', 'tabpanel');
        section.setAttribute('tabindex', '0');
        section.style.display = 'none';

        // Search bar (mirrors the dictionary's search bar)
        const header = document.createElement('div');
        header.className = 'dictionary-header';
        const searchWrapper = document.createElement('div');
        searchWrapper.className = 'search-wrapper';
        const searchInput = document.createElement('input');
        searchInput.type = 'text';
        searchInput.className = 'module-search-input';
        searchInput.placeholder = 'Search words...';
        searchInput.setAttribute('aria-label', `Search ${moduleName} words`);
        const clearBtn = document.createElement('button');
        clearBtn.type = 'button';
        clearBtn.className = 'inline-clear-btn';
        clearBtn.setAttribute('aria-label', 'Clear search');
        clearBtn.textContent = '\u00d7';
        searchWrapper.appendChild(searchInput);
        searchWrapper.appendChild(clearBtn);
        header.appendChild(searchWrapper);

        // Wire search events
        searchInput.addEventListener('input', () => {
            onSearchInput(searchInput.value);
        });
        clearBtn.addEventListener('click', () => {
            searchInput.value = '';
            onSearchInput('');
        });

        const wordInfoDisplay = document.createElement('span');
        wordInfoDisplay.className = 'word-info-display module-word-info';

        const wordsHeader = document.createElement('div');
        wordsHeader.className = 'words-header';
        const h3 = document.createElement('h3');
        h3.textContent = `${moduleName} Words`;
        wordsHeader.appendChild(h3);
        wordsHeader.appendChild(wordInfoDisplay);

        const wordsArea = document.createElement('div');
        wordsArea.className = 'builtin-words-area';
        wordsArea.appendChild(wordsHeader);

        const wordsDisplay = document.createElement('div');
        wordsDisplay.className = 'words-display module-words-display';
        wordsArea.appendChild(wordsDisplay);

        // Background click/dblclick handlers
        setupBackgroundClickHandlers(wordsDisplay);

        const container = document.createElement('div');
        container.className = 'dictionary-container';
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

                const button = document.createElement('button');
                button.textContent = shortName;
                button.className = 'word-button module';
                button.title = description;
                button.addEventListener('click', () => onWordClick(name));
                button.addEventListener('mouseenter', () => {
                    (wordInfo as HTMLElement).textContent = description;
                });
                button.addEventListener('mouseleave', () => {
                    (wordInfo as HTMLElement).textContent = '';
                });

                wordsDisplay.appendChild(button);
            });

            if (searchFilter && matched.length === 0) {
                const message = document.createElement('div');
                message.className = 'no-results-message';
                message.textContent = 'No matching words found';
                wordsDisplay.appendChild(message);
            }
        } catch (error) {
            console.error(`Failed to render module words for ${tab.moduleName}:`, error);
        }
    };

    const findTab = (moduleName: string): ModuleTab | undefined =>
        tabs.find(t => t.moduleName === moduleName);

    /** Sync search input values across all module tab areas */
    const syncSearchInputValues = (): void => {
        for (const tab of tabs) {
            const input = tab.areaEl.querySelector('.module-search-input') as HTMLInputElement | null;
            if (input && input.value !== searchFilter) {
                input.value = searchFilter;
            }
        }
    };

    const syncModuleTabs = (): void => {
        if (!window.ajisaiInterpreter) return;

        try {
            const importedModules: string[] = window.ajisaiInterpreter.get_imported_modules();
            const importedSet = new Set(importedModules);

            // Remove tabs for modules no longer imported
            for (let i = tabs.length - 1; i >= 0; i--) {
                const tab = tabs[i]!;
                if (!importedSet.has(tab.moduleName)) {
                    tab.tabBtn.remove();
                    tab.areaEl.remove();
                    tabs.splice(i, 1);
                }
            }

            // Add tabs for newly imported modules
            for (const moduleName of importedModules) {
                if (!findTab(moduleName)) {
                    const viewMode: ViewMode = `module:${moduleName}`;
                    const tabBtn = createTabButton(moduleName, viewMode);
                    const areaEl = createAreaElement(moduleName);

                    tabGroupEl.appendChild(tabBtn);
                    areaContainerEl.appendChild(areaEl);

                    const tab: ModuleTab = { moduleName, viewMode, tabBtn, areaEl };
                    tabs.push(tab);
                }
            }

            // Re-render words for all module tabs
            for (const tab of tabs) {
                renderModuleWords(tab);
            }
        } catch (error) {
            console.error('Failed to sync module tabs:', error);
        }
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
        setSearchFilter
    };
};
