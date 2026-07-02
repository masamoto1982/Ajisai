// Sheet view in-cell editor (redesign plan §5: cell-editor). Owns the
// lifecycle of the single <input> that overlays the cell being edited —
// the cell *is* the text editor, which is the first of the two spreadsheet
// strengths the plan is built on (§0).

export type EditCommitDirection = 'down' | 'right' | 'stay';

export interface CellEditorCallbacks {
    /** Commit the edited text; `direction` tells the grid where to move. */
    onCommit(rawText: string, direction: EditCommitDirection): void;
    onCancel(): void;
}

export interface CellEditor {
    /** Open the editor inside `cell` with `initialText`, caret at end. */
    open(cell: HTMLElement, initialText: string): void;
    readonly isOpen: () => boolean;
    /** Commit programmatically (e.g. when the grid loses focus). */
    commitIfOpen(): void;
}

export function createCellEditor(callbacks: CellEditorCallbacks): CellEditor {
    let input: HTMLInputElement | null = null;
    let host: HTMLElement | null = null;
    let settled = false;

    const close = (): void => {
        if (input && host) {
            input.remove();
            host.classList.remove('sheet-cell-editing');
        }
        input = null;
        host = null;
    };

    const settle = (action: () => void): void => {
        if (settled) return;
        settled = true;
        action();
        close();
    };

    const open = (cell: HTMLElement, initialText: string): void => {
        commitIfOpen();
        settled = false;
        host = cell;
        input = document.createElement('input');
        input.type = 'text';
        input.className = 'sheet-cell-input';
        input.value = initialText;
        input.setAttribute('aria-label', 'セルの編集');

        input.addEventListener('keydown', (event: KeyboardEvent) => {
            // The grid's own navigation must not fire while editing.
            event.stopPropagation();
            const current = input;
            if (!current) return;
            if (event.key === 'Enter') {
                event.preventDefault();
                settle(() => callbacks.onCommit(current.value, 'down'));
            } else if (event.key === 'Tab') {
                event.preventDefault();
                settle(() => callbacks.onCommit(current.value, 'right'));
            } else if (event.key === 'Escape') {
                event.preventDefault();
                settle(() => callbacks.onCancel());
            }
        });
        input.addEventListener('blur', () => {
            const current = input;
            if (!current) return;
            settle(() => callbacks.onCommit(current.value, 'stay'));
        });

        cell.classList.add('sheet-cell-editing');
        cell.appendChild(input);
        input.focus();
        input.setSelectionRange(initialText.length, initialText.length);
    };

    const commitIfOpen = (): void => {
        const current = input;
        if (!current) return;
        settle(() => callbacks.onCommit(current.value, 'stay'));
    };

    return {
        open,
        isOpen: () => input !== null,
        commitIfOpen,
    };
}
