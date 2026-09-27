// Custom control for choosing the dictionary sheet (Core or User).
//
// It replaces a native <select> because the app styles the closed control and
// the open panel itself. An inert native select still paints the closed
// control (the browser-provided text, height and chevron); the overlaid button
// owns the popup.

export type SelectorEntryKind = 'core' | 'user';

export interface SelectorEntry {
    readonly sheetId: string;
    readonly label: string;
    readonly kind: SelectorEntryKind;
}

export interface DictionarySheetSelectorOptions {
    /** Called when the user picks a sheet; not when `select` is called. */
    readonly onChange?: (sheetId: string) => void;
}

export interface DictionarySheetSelector {
    readonly setEntries: (entries: SelectorEntry[]) => void;
    readonly select: (sheetId: string) => void;
    readonly current: () => string;
}

const SHEET_SELECTOR_PANEL_ID = 'sheet-selector-panel';

export const createDictionarySheetSelector = (
    rootEl: HTMLElement,
    options: DictionarySheetSelectorOptions = {}
): DictionarySheetSelector => {
    let entries: SelectorEntry[] = [];
    // The Core sheet is the one shown in the initial markup, so the trigger
    // label and the visible sheet agree before the first setEntries().
    let currentValue = 'core';

    rootEl.classList.add('sheet-selector');
    rootEl.replaceChildren();

    const nativeTrigger = document.createElement('select');
    nativeTrigger.className = 'sheet-selector-native-trigger';
    nativeTrigger.setAttribute('aria-hidden', 'true');
    nativeTrigger.tabIndex = -1;

    const trigger = document.createElement('button');
    trigger.type = 'button';
    trigger.className = 'sheet-selector-trigger';
    trigger.setAttribute('aria-haspopup', 'listbox');
    trigger.setAttribute('aria-expanded', 'false');
    // The browser toggles the popover, which gets the open-trigger /
    // light-dismiss interplay right.
    trigger.setAttribute('popovertarget', SHEET_SELECTOR_PANEL_ID);

    const panel = document.createElement('div');
    panel.className = 'sheet-selector-panel';
    panel.id = SHEET_SELECTOR_PANEL_ID;
    panel.setAttribute('role', 'listbox');
    // Native popover: top-layer placement plus light-dismiss on outside click
    // and Escape.
    panel.popover = 'auto';

    // The popover lives in the top layer, so it is anchored to the closed
    // control with fixed coordinates taken just before it opens.
    panel.addEventListener('beforetoggle', (e) => {
        if ((e as ToggleEvent).newState !== 'open') return;
        const rect = rootEl.getBoundingClientRect();
        panel.style.left = `${rect.left}px`;
        panel.style.top = `${rect.bottom + 2}px`;
        panel.style.width = `${rect.width}px`;
    });
    panel.addEventListener('toggle', (e) => {
        trigger.setAttribute('aria-expanded', String((e as ToggleEvent).newState === 'open'));
    });

    rootEl.append(nativeTrigger, trigger, panel);

    const labelFor = (sheetId: string): string =>
        entries.find(e => e.sheetId === sheetId)?.label ?? sheetId;

    const renderNativeTrigger = (): void => {
        nativeTrigger.replaceChildren(...entries.map(entry => {
            const optionEl = document.createElement('option');
            optionEl.value = entry.sheetId;
            optionEl.textContent = entry.label;
            return optionEl;
        }));
        nativeTrigger.value = currentValue;
        trigger.textContent = labelFor(currentValue);
    };

    const renderPanel = (): void => {
        panel.replaceChildren(...entries.map(entry => {
            const selected = entry.sheetId === currentValue;
            const optionEl = document.createElement('button');
            optionEl.type = 'button';
            optionEl.setAttribute('role', 'option');
            optionEl.className = 'sheet-selector-option';
            optionEl.classList.toggle('is-selected', selected);
            optionEl.setAttribute('aria-selected', String(selected));
            optionEl.textContent = entry.label;
            optionEl.addEventListener('click', () => {
                select(entry.sheetId);
                if (panel.matches(':popover-open')) panel.hidePopover();
                options.onChange?.(entry.sheetId);
            });
            return optionEl;
        }));
    };

    const select = (sheetId: string): void => {
        currentValue = sheetId;
        renderNativeTrigger();
        renderPanel();
    };

    const setEntries = (next: SelectorEntry[]): void => {
        entries = next;
        if (!entries.some(e => e.sheetId === currentValue)) {
            currentValue = entries[0]?.sheetId ?? '';
        }
        renderNativeTrigger();
        renderPanel();
    };

    return {
        setEntries,
        select,
        current: () => currentValue
    };
};
