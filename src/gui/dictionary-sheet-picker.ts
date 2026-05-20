export interface DictionarySheetPickerOptions {
    readonly selectEl: HTMLSelectElement;
    readonly onSelectSheet: (sheetId: string) => void;
    readonly onImportModule: (moduleName: string) => void;
    readonly onUnimportModule: (moduleName: string) => void;
}

export interface DictionarySheetPicker {
    readonly refresh: () => void;
    readonly syncSelection: () => void;
}

type ModuleAffordance = 'import' | 'unimport' | null;

const moduleAffordanceForOption = (option: HTMLOptionElement): ModuleAffordance => {
    switch (option.dataset.moduleState) {
        case 'available': return 'import';
        case 'imported': return 'unimport';
        default: return null;
    }
};

export const createDictionarySheetPicker = (
    options: DictionarySheetPickerOptions
): DictionarySheetPicker => {
    const { selectEl, onSelectSheet, onImportModule, onUnimportModule } = options;

    const wrapper = document.createElement('div');
    wrapper.className = 'dictionary-sheet-picker';

    const trigger = document.createElement('button');
    trigger.type = 'button';
    trigger.className = 'dictionary-sheet-picker-trigger';
    trigger.setAttribute('aria-haspopup', 'listbox');
    trigger.setAttribute('aria-expanded', 'false');
    trigger.setAttribute('aria-label', 'Select dictionary');
    wrapper.appendChild(trigger);

    const list = document.createElement('div');
    list.hidden = true;
    list.className = 'dictionary-sheet-picker-list';
    list.setAttribute('role', 'listbox');
    wrapper.appendChild(list);

    selectEl.classList.add('dictionary-sheet-native-select');
    selectEl.after(wrapper);

    const hideList = (): void => {
        list.hidden = true;
        trigger.setAttribute('aria-expanded', 'false');
    };

    const showList = (): void => {
        list.hidden = false;
        trigger.setAttribute('aria-expanded', 'true');
    };

    const toggleList = (): void => {
        if (list.hidden) {
            showList();
        } else {
            hideList();
        }
    };

    const syncSelection = (): void => {
        const selected = selectEl.selectedOptions[0] ?? selectEl.options[0];
        const label = selected?.textContent?.trim() || 'Select dictionary';
        const state = selected?.dataset.moduleState ?? 'base';
        trigger.textContent = label;
        trigger.dataset.moduleState = state;

        for (const item of list.querySelectorAll<HTMLElement>('.dictionary-sheet-picker-item')) {
            const isSelected = item.dataset.sheetId === selectEl.value;
            item.classList.toggle('is-selected', isSelected);
            item.setAttribute('aria-selected', isSelected ? 'true' : 'false');
        }
    };

    const selectSheet = (sheetId: string): void => {
        if (selectEl.value === sheetId) {
            syncSelection();
            onSelectSheet(sheetId);
            return;
        }
        selectEl.value = sheetId;
        selectEl.dispatchEvent(new Event('change', { bubbles: true }));
        syncSelection();
    };

    const buildActionButton = (
        moduleName: string,
        affordance: Exclude<ModuleAffordance, null>
    ): HTMLButtonElement => {
        const button = document.createElement('button');
        button.type = 'button';
        const isImport = affordance === 'import';
        button.textContent = isImport ? '+' : '−';
        button.className = `dictionary-sheet-picker-item-action ${isImport ? 'is-import' : 'is-unimport'}`;
        const label = isImport
            ? `Import this module (${moduleName})`
            : `Unimport this module (${moduleName})`;
        button.setAttribute('aria-label', label);
        button.addEventListener('click', (event) => {
            event.stopPropagation();
            hideList();
            if (isImport) {
                onImportModule(moduleName);
            } else {
                onUnimportModule(moduleName);
            }
        });
        return button;
    };

    const refresh = (): void => {
        list.innerHTML = '';
        for (const option of Array.from(selectEl.options)) {
            const item = document.createElement('div');
            item.className = 'dictionary-sheet-picker-item';
            item.dataset.sheetId = option.value;
            item.dataset.moduleState = option.dataset.moduleState ?? 'base';
            item.setAttribute('role', 'option');
            item.tabIndex = 0;

            const label = document.createElement('span');
            label.className = 'dictionary-sheet-picker-item-label';
            label.textContent = option.textContent;
            item.appendChild(label);

            const state = option.dataset.moduleState;
            if (state === 'available') {
                item.classList.add('is-available');
            } else if (state === 'imported') {
                item.classList.add('is-imported');
            } else {
                item.classList.add('is-base');
            }

            const affordance = moduleAffordanceForOption(option);
            const moduleName = option.dataset.moduleName;
            if (affordance && moduleName) {
                item.appendChild(buildActionButton(moduleName, affordance));
            }

            const activate = (): void => {
                selectSheet(option.value);
                hideList();
            };
            item.addEventListener('click', activate);
            item.addEventListener('keydown', (event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    activate();
                }
            });
            list.appendChild(item);
        }
        syncSelection();
    };

    trigger.addEventListener('click', (event) => {
        event.stopPropagation();
        toggleList();
    });

    selectEl.addEventListener('change', () => {
        syncSelection();
    });

    document.addEventListener('click', (event) => {
        if (!wrapper.contains(event.target as Node)) hideList();
    });
    window.addEventListener('blur', () => {
        hideList();
    });

    refresh();

    return { refresh, syncSelection };
};
