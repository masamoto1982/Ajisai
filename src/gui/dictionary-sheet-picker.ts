type ContextMenuAction = {
    readonly label: string;
    readonly onClick: () => void;
};

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

const createContextMenuElement = (): HTMLDivElement => {
    const menu = document.createElement('div');
    menu.hidden = true;
    menu.className = 'context-menu module-context-menu dictionary-picker-context-menu';
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
        button.addEventListener('click', (clickEvent) => {
            clickEvent.stopPropagation();
            menu.hidden = true;
            action.onClick();
        });
        menu.appendChild(button);
    }
    menu.hidden = false;
    menu.style.left = `${event.clientX}px`;
    menu.style.top = `${event.clientY}px`;
};

const moduleActionsForOption = (
    option: HTMLOptionElement,
    onImportModule: (moduleName: string) => void,
    onUnimportModule: (moduleName: string) => void
): readonly ContextMenuAction[] => {
    const moduleName = option.dataset.moduleName;
    const moduleState = option.dataset.moduleState;
    if (!moduleName || !moduleState) return [];

    if (moduleState === 'available') {
        return [{
            label: `Import this module (${moduleName})`,
            onClick: () => onImportModule(moduleName),
        }];
    }

    if (moduleState === 'imported') {
        return [{
            label: `Unimport this module (${moduleName})`,
            onClick: () => onUnimportModule(moduleName),
        }];
    }

    return [];
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

    const contextMenu = createContextMenuElement();

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

    const hideContextMenu = (): void => {
        contextMenu.hidden = true;
    };

    const syncSelection = (): void => {
        const selected = selectEl.selectedOptions[0] ?? selectEl.options[0];
        const label = selected?.textContent?.trim() || 'Select dictionary';
        const state = selected?.dataset.moduleState ?? 'base';
        trigger.textContent = label;
        trigger.dataset.moduleState = state;
        trigger.title = selected?.title || label;

        for (const item of list.querySelectorAll<HTMLButtonElement>('.dictionary-sheet-picker-item')) {
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

    const openModuleContextMenu = (event: MouseEvent, option: HTMLOptionElement | undefined): void => {
        if (!option) return;
        const actions = moduleActionsForOption(option, onImportModule, onUnimportModule);
        if (actions.length === 0) return;
        event.preventDefault();
        event.stopPropagation();
        hideList();
        renderContextMenu(contextMenu, event, actions);
    };

    const refresh = (): void => {
        list.innerHTML = '';
        for (const option of Array.from(selectEl.options)) {
            const item = document.createElement('button');
            item.type = 'button';
            item.className = 'dictionary-sheet-picker-item';
            item.dataset.sheetId = option.value;
            item.dataset.moduleState = option.dataset.moduleState ?? 'base';
            item.textContent = option.textContent;
            item.title = option.title || option.textContent || '';
            item.setAttribute('role', 'option');

            const state = option.dataset.moduleState;
            if (state === 'available') {
                item.classList.add('is-available');
            } else if (state === 'imported') {
                item.classList.add('is-imported');
            } else {
                item.classList.add('is-base');
            }

            item.addEventListener('click', () => {
                selectSheet(option.value);
                hideList();
            });
            item.addEventListener('contextmenu', (event) => openModuleContextMenu(event, option));
            list.appendChild(item);
        }
        syncSelection();
    };

    trigger.addEventListener('click', (event) => {
        event.stopPropagation();
        hideContextMenu();
        toggleList();
    });

    trigger.addEventListener('contextmenu', (event) => {
        const selected = selectEl.selectedOptions[0];
        openModuleContextMenu(event, selected);
    });

    selectEl.addEventListener('change', () => {
        syncSelection();
    });

    document.addEventListener('click', (event) => {
        if (!wrapper.contains(event.target as Node)) hideList();
        hideContextMenu();
    });
    window.addEventListener('blur', () => {
        hideList();
        hideContextMenu();
    });

    refresh();

    return { refresh, syncSelection };
};
