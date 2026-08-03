// Custom replacement for the native `<select id="dictionary-sheet-select">`.
//
// The dictionary has two tiers, so the control lists exactly two sheets: Core
// and User. It replaces the native select because the app styles the closed
// control and the open panel itself.
//
// To stay drop-in compatible with the call sites that used the native select,
// the component installs a `value` accessor on the root element and dispatches
// a `change` event when the selection changes via user interaction (matching
// native behavior: programmatic `value =` assignment does not fire `change`).

export type SelectorEntryKind = 'core' | 'user';

export interface SelectorEntry {
    readonly sheetId: string;
    readonly label: string;
    readonly kind: SelectorEntryKind;
}

export interface DictionarySheetSelectElement extends HTMLElement {
    value: string;
}

export interface DictionarySheetSelector {
    readonly setEntries: (entries: SelectorEntry[]) => void;
    readonly setValue: (sheetId: string) => void;
    readonly getValue: () => string;
}

export const createDictionarySheetSelector = (
    rootEl: HTMLElement
): DictionarySheetSelector => {
    let entries: SelectorEntry[] = [];
    // Defaults to the Core sheet, which is the sheet marked active in the
    // initial markup, so the trigger label and the visible sheet agree before
    // the first setEntries() populates the real list.
    let currentValue = 'core';

    rootEl.dataset.value = currentValue;
    rootEl.classList.add('sheet-selector');
    rootEl.innerHTML = '';

    // Keep the closed control visually identical to the app's native selects:
    // the overlaid button owns the custom popup behavior, while this inert
    // native select paints the browser-provided text, height, and chevron.
    const nativeTrigger = document.createElement('select');
    nativeTrigger.className = 'sheet-selector-native-trigger';
    nativeTrigger.setAttribute('aria-hidden', 'true');
    nativeTrigger.tabIndex = -1;

    const SHEET_SELECTOR_PANEL_ID = 'sheet-selector-panel';

    const trigger = document.createElement('button');
    trigger.type = 'button';
    trigger.className = 'sheet-selector-trigger';
    trigger.setAttribute('aria-haspopup', 'listbox');
    trigger.setAttribute('aria-expanded', 'false');
    // Let the browser toggle the popover natively (handles the
    // open-trigger/light-dismiss interplay that a manual click handler gets wrong).
    trigger.setAttribute('popovertarget', SHEET_SELECTOR_PANEL_ID);

    const panel = document.createElement('div');
    panel.className = 'sheet-selector-panel';
    panel.id = SHEET_SELECTOR_PANEL_ID;
    panel.setAttribute('role', 'listbox');
    // Native popover: top-layer placement plus light-dismiss on outside click /
    // Escape, replacing the former document-click and window-blur listeners.
    panel.popover = 'auto';

    // The popover lives in the top layer, so anchor it to the closed control with
    // fixed coordinates taken just before it opens (no flash), and mirror its width.
    const positionPanel = (): void => {
        const rect = rootEl.getBoundingClientRect();
        panel.style.left = `${rect.left}px`;
        panel.style.top = `${rect.bottom + 2}px`;
        panel.style.width = `${rect.width}px`;
    };
    panel.addEventListener('beforetoggle', (e) => {
        if ((e as Event & { newState: string }).newState === 'open') positionPanel();
    });
    panel.addEventListener('toggle', (e) => {
        const isOpen = (e as Event & { newState: string }).newState === 'open';
        trigger.setAttribute('aria-expanded', String(isOpen));
    });

    rootEl.appendChild(nativeTrigger);
    rootEl.appendChild(trigger);
    rootEl.appendChild(panel);

    const labelFor = (sheetId: string): string =>
        entries.find(e => e.sheetId === sheetId)?.label ?? sheetId;

    const syncTrigger = (): void => {
        const label = currentValue ? labelFor(currentValue) : 'Select dictionary';
        trigger.textContent = label;
        nativeTrigger.value = currentValue;
    };

    const syncNativeTriggerOptions = (): void => {
        nativeTrigger.innerHTML = '';
        const fragment = document.createDocumentFragment();
        for (const entry of entries) {
            const optionEl = document.createElement('option');
            optionEl.value = entry.sheetId;
            optionEl.textContent = entry.label;
            fragment.appendChild(optionEl);
        }
        nativeTrigger.appendChild(fragment);
        nativeTrigger.value = currentValue;
    };

    const closePanel = (): void => {
        if (panel.matches(':popover-open')) panel.hidePopover();
    };

    const selectSheet = (sheetId: string): void => {
        currentValue = sheetId;
        rootEl.dataset.value = sheetId;
        syncNativeTriggerOptions();
        syncTrigger();
        renderPanel();
    };

    const renderPanel = (): void => {
        panel.innerHTML = '';
        const fragment = document.createDocumentFragment();
        for (const entry of entries) {
            const optionEl = document.createElement('button');
            optionEl.type = 'button';
            optionEl.setAttribute('role', 'option');
            optionEl.className = [
                'sheet-selector-option',
                entry.sheetId === currentValue ? 'is-selected' : ''
            ].filter(Boolean).join(' ');
            optionEl.textContent = entry.label;
            optionEl.setAttribute('aria-selected', String(entry.sheetId === currentValue));

            optionEl.addEventListener('click', () => {
                selectSheet(entry.sheetId);
                closePanel();
                // Mirror native <select>: a user-driven selection fires `change`
                // (programmatic `value =` assignment does not).
                rootEl.dispatchEvent(new Event('change'));
            });

            fragment.appendChild(optionEl);
        }
        panel.appendChild(fragment);
    };

    Object.defineProperty(rootEl, 'value', {
        configurable: true,
        get: () => currentValue,
        set: (sheetId: string) => { selectSheet(sheetId); }
    });

    const setEntries = (next: SelectorEntry[]): void => {
        entries = next;
        if (currentValue && !entries.some(e => e.sheetId === currentValue)) {
            currentValue = entries[0]?.sheetId ?? '';
            rootEl.dataset.value = currentValue;
        }
        syncNativeTriggerOptions();
        syncTrigger();
        renderPanel();
    };

    return {
        setEntries,
        setValue: (sheetId: string) => selectSheet(sheetId),
        getValue: () => currentValue
    };
};
