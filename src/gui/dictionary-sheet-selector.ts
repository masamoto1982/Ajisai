// Custom replacement for the native `<select id="dictionary-sheet-select">`.
//
// A native select cannot long-press individual options nor style un-imported
// modules as greyed-out candidates, both of which the module activation UI
// needs. This component pre-lists every dictionary (Core, User, and all
// importable modules), renders inactive modules faded, navigates on a short
// tap, and toggles a module's import on a long-press.
//
// To stay drop-in compatible with the call sites that used the native select,
// the component installs a `value` accessor on the root element and dispatches
// a `change` event when the selection changes via user interaction (matching
// native behavior: programmatic `value =` assignment does not fire `change`).

const LONG_PRESS_MS = 500;
const LONG_PRESS_MOVE_TOLERANCE_PX = 10;

export type SelectorEntryKind = 'core' | 'user' | 'module';

export interface SelectorEntry {
    readonly sheetId: string;
    readonly label: string;
    readonly kind: SelectorEntryKind;
    readonly moduleName?: string;
    readonly active?: boolean;
}

export interface DictionarySheetSelectorOptions {
    readonly onToggleModule: (moduleName: string, currentlyActive: boolean) => void;
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
    rootEl: HTMLElement,
    options: DictionarySheetSelectorOptions
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

    // Long-press detection (touch + mouse), mirrored from word buttons: a held
    // press past the threshold toggles the module and suppresses the click so
    // the long-press never doubles as a navigation tap.
    const attachLongPress = (el: HTMLElement, onLongPress: () => void): void => {
        let timer: ReturnType<typeof setTimeout> | null = null;
        let fired = false;
        let startX = 0;
        let startY = 0;
        const cancel = (): void => {
            if (timer) { clearTimeout(timer); timer = null; }
        };
        el.addEventListener('pointerdown', (e: PointerEvent) => {
            if (e.button !== 0) return;
            fired = false;
            startX = e.clientX;
            startY = e.clientY;
            cancel();
            timer = setTimeout(() => { fired = true; timer = null; onLongPress(); }, LONG_PRESS_MS);
        });
        el.addEventListener('pointermove', (e: PointerEvent) => {
            if (!timer) return;
            if (Math.abs(e.clientX - startX) > LONG_PRESS_MOVE_TOLERANCE_PX
                || Math.abs(e.clientY - startY) > LONG_PRESS_MOVE_TOLERANCE_PX) {
                cancel();
            }
        });
        el.addEventListener('pointerup', cancel);
        el.addEventListener('pointerleave', cancel);
        el.addEventListener('pointercancel', cancel);
        el.addEventListener('click', (e: MouseEvent) => {
            if (fired) {
                fired = false;
                e.preventDefault();
                e.stopImmediatePropagation();
            }
        });
    };

    const renderPanel = (): void => {
        panel.innerHTML = '';
        const fragment = document.createDocumentFragment();
        for (const entry of entries) {
            const optionEl = document.createElement('button');
            optionEl.type = 'button';
            optionEl.setAttribute('role', 'option');
            const isActive = entry.kind !== 'module' || entry.active !== false;
            optionEl.className = [
                'sheet-selector-option',
                entry.sheetId === currentValue ? 'is-selected' : '',
                entry.kind === 'module' && !isActive ? 'is-inactive' : ''
            ].filter(Boolean).join(' ');
            optionEl.textContent = entry.label;
            optionEl.setAttribute('aria-selected', String(entry.sheetId === currentValue));

            if (entry.kind === 'module' && entry.moduleName) {
                const moduleName = entry.moduleName;
                optionEl.title = `${entry.label}\nLong-press to ${isActive ? 'unimport' : 'import'} this module.`;
                attachLongPress(optionEl, () => {
                    closePanel();
                    options.onToggleModule(moduleName, isActive);
                });
            }

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
