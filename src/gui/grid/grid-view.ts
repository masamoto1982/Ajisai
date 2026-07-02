// Sheet view table rendering (redesign plan §5: grid-view). Numbers-style:
// one finite table sitting on a light canvas, sized to its content, with
// row/column add affordances — not an infinite grid. Selection, keyboard
// navigation, and the in-cell editor live here; recalculation and word
// bookkeeping stay in the controller.

import { columnIndexToLetters, formatCellRef, parseCellRef } from '../../sheet/cell-address';
import type { SheetGridLimits } from '../../sheet/cell-address';
import { createCellEditor, type EditCommitDirection } from './cell-editor';
import type { CellDisplay } from './cell-renderer';

export interface GridViewCallbacks {
    /** A cell was selected (canonical unqualified address, e.g. 'A1'). */
    onSelect(address: string): void;
    /** An edit was committed; the controller re-renders affected cells. */
    onCommitEdit(address: string, rawText: string): void;
    /** Raw text (as typed) to prefill when editing an existing cell. */
    resolveRawText(address: string): string;
}

export interface GridView {
    readonly element: HTMLElement;
    updateCell(address: string, display: CellDisplay): void;
    /** Grow the visible table so `address` is shown (restore path). */
    revealAddress(address: string): void;
    extractSelectedAddress(): string | null;
    focus(): void;
}

/** Numbers-like starting size: small and purposeful, growable. */
const INITIAL_ROWS = 12;
const INITIAL_COLS = 6;
const ROW_GROWTH = 6;
const COL_GROWTH = 2;

export function createGridView(
    limits: SheetGridLimits,
    callbacks: GridViewCallbacks,
): GridView {
    let visibleRows = Math.min(INITIAL_ROWS, limits.rows);
    let visibleCols = Math.min(INITIAL_COLS, limits.cols);
    let selected: { col: number; row: number } = { col: 0, row: 0 };
    const displays = new Map<string, CellDisplay>();

    const element = document.createElement('div');
    element.className = 'sheet-table-frame';

    const table = document.createElement('table');
    table.className = 'sheet-table';
    table.setAttribute('role', 'grid');
    element.appendChild(table);

    const controls = document.createElement('div');
    controls.className = 'sheet-table-controls';
    const addRowBtn = document.createElement('button');
    addRowBtn.type = 'button';
    addRowBtn.className = 'sheet-add-btn sheet-add-row';
    addRowBtn.textContent = '+ 行';
    const addColBtn = document.createElement('button');
    addColBtn.type = 'button';
    addColBtn.className = 'sheet-add-btn sheet-add-col';
    addColBtn.textContent = '+ 列';
    controls.append(addRowBtn, addColBtn);
    element.appendChild(controls);

    const editor = createCellEditor({
        onCommit: (rawText: string, direction: EditCommitDirection) => {
            const address = formatCellRef(selected);
            callbacks.onCommitEdit(address, rawText);
            if (direction === 'down') moveSelection(0, 1);
            if (direction === 'right') moveSelection(1, 0);
            table.focus();
        },
        onCancel: () => {
            table.focus();
        },
    });

    const lookupCellElement = (address: string): HTMLTableCellElement | null =>
        table.querySelector(`td[data-address="${address}"]`);

    const applyDisplay = (cell: HTMLTableCellElement, display: CellDisplay): void => {
        cell.textContent = display.text;
        cell.dataset.kind = display.kind;
        if (display.detail) {
            cell.title = display.detail;
        } else {
            cell.removeAttribute('title');
        }
    };

    const renderTable = (): void => {
        table.textContent = '';
        table.tabIndex = 0;

        const thead = document.createElement('thead');
        const headRow = document.createElement('tr');
        const corner = document.createElement('th');
        corner.className = 'sheet-corner';
        corner.setAttribute('aria-label', 'TABLE1');
        headRow.appendChild(corner);
        for (let col = 0; col < visibleCols; col++) {
            const th = document.createElement('th');
            th.scope = 'col';
            th.textContent = columnIndexToLetters(col);
            headRow.appendChild(th);
        }
        thead.appendChild(headRow);
        table.appendChild(thead);

        const tbody = document.createElement('tbody');
        for (let row = 0; row < visibleRows; row++) {
            const tr = document.createElement('tr');
            const th = document.createElement('th');
            th.scope = 'row';
            th.textContent = String(row + 1);
            tr.appendChild(th);
            for (let col = 0; col < visibleCols; col++) {
                const address = formatCellRef({ col, row });
                const td = document.createElement('td');
                td.dataset.address = address;
                td.setAttribute('role', 'gridcell');
                const display = displays.get(address);
                if (display) applyDisplay(td, display);
                tr.appendChild(td);
            }
            tbody.appendChild(tr);
        }
        table.appendChild(tbody);
        refreshSelection();
    };

    const refreshSelection = (): void => {
        for (const marked of table.querySelectorAll('.sheet-cell-selected')) {
            marked.classList.remove('sheet-cell-selected');
        }
        const address = formatCellRef(selected);
        const cell = lookupCellElement(address);
        if (cell) {
            cell.classList.add('sheet-cell-selected');
            cell.scrollIntoView({ block: 'nearest', inline: 'nearest' });
        }
        callbacks.onSelect(address);
    };

    const moveSelection = (deltaCol: number, deltaRow: number): void => {
        const col = Math.max(0, Math.min(limits.cols - 1, selected.col + deltaCol));
        const row = Math.max(0, Math.min(limits.rows - 1, selected.row + deltaRow));
        if (col >= visibleCols || row >= visibleRows) {
            // Numbers-style: arrow keys never silently grow the table; the
            // add buttons do. Clamp to the visible edge instead.
            selected = {
                col: Math.min(col, visibleCols - 1),
                row: Math.min(row, visibleRows - 1),
            };
        } else {
            selected = { col, row };
        }
        refreshSelection();
    };

    const openEditor = (initialText: string | null): void => {
        const address = formatCellRef(selected);
        const cell = lookupCellElement(address);
        if (!cell) return;
        editor.open(cell, initialText ?? callbacks.resolveRawText(address));
    };

    table.addEventListener('mousedown', (event: MouseEvent) => {
        const target = (event.target as HTMLElement).closest('td[data-address]');
        if (!target) return;
        if (editor.isOpen()) editor.commitIfOpen();
        const coord = parseCellRef((target as HTMLTableCellElement).dataset.address as string);
        if (!coord) return;
        selected = coord;
        refreshSelection();
        table.focus();
        event.preventDefault();
    });

    table.addEventListener('dblclick', (event: MouseEvent) => {
        const target = (event.target as HTMLElement).closest('td[data-address]');
        if (!target) return;
        openEditor(null);
        event.preventDefault();
    });

    table.addEventListener('keydown', (event: KeyboardEvent) => {
        if (editor.isOpen()) return;
        switch (event.key) {
            case 'ArrowUp':
                moveSelection(0, -1);
                event.preventDefault();
                return;
            case 'ArrowDown':
                moveSelection(0, 1);
                event.preventDefault();
                return;
            case 'ArrowLeft':
                moveSelection(-1, 0);
                event.preventDefault();
                return;
            case 'ArrowRight':
                moveSelection(1, 0);
                event.preventDefault();
                return;
            case 'Tab':
                moveSelection(event.shiftKey ? -1 : 1, 0);
                event.preventDefault();
                return;
            case 'Enter':
            case 'F2':
                openEditor(null);
                event.preventDefault();
                return;
            case 'Delete':
            case 'Backspace':
                callbacks.onCommitEdit(formatCellRef(selected), '');
                event.preventDefault();
                return;
            default:
                break;
        }
        // Typing starts a fresh edit, replacing the cell content
        // (spreadsheet convention).
        if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
            openEditor(event.key);
            event.preventDefault();
        }
    });

    addRowBtn.addEventListener('click', () => {
        visibleRows = Math.min(limits.rows, visibleRows + ROW_GROWTH);
        renderTable();
    });
    addColBtn.addEventListener('click', () => {
        visibleCols = Math.min(limits.cols, visibleCols + COL_GROWTH);
        renderTable();
    });

    renderTable();

    return {
        element,
        updateCell: (address: string, display: CellDisplay): void => {
            displays.set(address, display);
            const cell = lookupCellElement(address);
            if (cell) applyDisplay(cell, display);
        },
        revealAddress: (address: string): void => {
            const coord = parseCellRef(address);
            if (!coord) return;
            let grown = false;
            while (coord.row >= visibleRows && visibleRows < limits.rows) {
                visibleRows = Math.min(limits.rows, visibleRows + ROW_GROWTH);
                grown = true;
            }
            while (coord.col >= visibleCols && visibleCols < limits.cols) {
                visibleCols = Math.min(limits.cols, visibleCols + COL_GROWTH);
                grown = true;
            }
            if (grown) renderTable();
        },
        extractSelectedAddress: () => formatCellRef(selected),
        focus: () => table.focus(),
    };
}
