// The Sheet view of the User dictionary (spec/gui-semantics.md, Sheet): a grid
// of cells, A1 to whatever is used, where each User Word stands in a cell —
// its value if it takes nothing and leaves one, its stack effect otherwise —
// and the formula bar above the grid shows the selected cell's name, its
// body as the formula and its type. The sheet keeps no dictionary of its own:
// a cell is the dictionary's Word, editing a formula runs `DEF`, an empty cell
// filled in is a new `DEF`, renaming is `DEF` then `DEL`. Where each Word
// stands is the one thing the sheet holds, and it is a view (user-sheet-layout).

import type { AjisaiInterpreter, ExecuteResult, SheetCellInfo, SheetCellValue, SheetTable, UserWordInfo } from '../wasm-interpreter-types';
import { isFailure, toError } from './interpreter-execution-utils';
import {
    formatAddress,
    gridExtent,
    parseAddress,
    placeWords,
    type SheetLayout
} from './user-sheet-layout';
import { deleteUserWord } from './vocabulary-state-controller';

export interface UserSheetCallbacks {
    readonly onWordClick: (word: string) => void;
    readonly onDictionaryChanged: () => Promise<void>;
    /** A cell moved: the layout is saved with the session. */
    readonly onLayoutChanged: () => void;
    readonly showInfo: (text: string, append: boolean) => void;
    readonly showError: (error: Error) => void;
    // Every call into the interpreter takes its turn with runs.
    readonly runExclusive: <T>(task: () => Promise<T>) => Promise<T>;
}

export interface UserSheet {
    readonly render: (words: UserWordInfo[]) => void;
    readonly layout: () => SheetLayout;
    readonly restoreLayout: (layout: SheetLayout) => void;
}

/** The source a sheet edit runs: the Word's `#:contract` line, then its DEF. */
export const sheetDefinitionSource = (
    name: string,
    formula: string,
    contract: string | null
): string => {
    const definition = `[ ${formula.trim()} ] '${name}' DEF`;
    return contract ? `#:contract ${name} ${contract}\n${definition}` : definition;
};

/** A name a sheet cell may take: one token, which a quoted String can hold. */
export const isSheetName = (name: string): boolean => /^[^\s'[\]#]+$/.test(name);

// Run a sheet edit on the session interpreter. A refused DEF puts its operands
// back on the stack; an edit of the sheet is not a stack edit, so the stack is
// restored as it was, as `deleteUserWord` does for a refused DEL.
export const runSheetEdit = async (
    interpreter: AjisaiInterpreter,
    source: string
): Promise<ExecuteResult> => {
    const stackBefore = interpreter.snapshot_stack();
    const result = await interpreter.execute(source);
    if (isFailure(result)) interpreter.restore_stack_snapshot(stackBefore);
    return result;
};

/**
 * Rename a Word: define the new name with the same body and declaration,
 * then delete the old one. A Word other Words call cannot be deleted, so it
 * cannot be renamed either; the new name is taken back and the refusal is
 * the answer, with the stack and dictionary as they were.
 */
export const renameSheetWord = async (
    interpreter: AjisaiInterpreter,
    from: string,
    to: string,
    formula: string,
    contract: string | null
): Promise<ExecuteResult> => {
    const defined = await runSheetEdit(interpreter, sheetDefinitionSource(to, formula, contract));
    if (isFailure(defined)) return defined;
    const deleted = await deleteUserWord(interpreter, from);
    if (isFailure(deleted)) await deleteUserWord(interpreter, to);
    return deleted;
};

const describeEffect = (inputs: number | null, outputs: number | null): string =>
    inputs === null || outputs === null
        ? 'variable stack effect'
        : `takes ${inputs}, leaves ${outputs}`;

/** A table's shape in a few words, for a cell too narrow to hold it. */
export const describeTable = (table: SheetTable): string =>
    `${table.columns.join(', ')} · ${table.rows} row${table.rows === 1 ? '' : 's'}`;

const tableElement = (table: SheetTable): HTMLElement => {
    const holder = document.createElement('div');
    holder.className = 'sheet-table-view';
    const grid = document.createElement('table');
    const head = document.createElement('tr');
    for (const column of table.columns) {
        const th = document.createElement('th');
        th.scope = 'col';
        th.textContent = column;
        head.append(th);
    }
    const thead = document.createElement('thead');
    thead.append(head);
    const tbody = document.createElement('tbody');
    for (const row of table.cells) {
        const tr = document.createElement('tr');
        for (const text of row) {
            const td = document.createElement('td');
            td.textContent = text;
            tr.append(td);
        }
        tbody.append(tr);
    }
    grid.append(thead, tbody);
    holder.append(grid);
    if (table.cells.length < table.rows) {
        const more = document.createElement('p');
        more.className = 'sheet-table-more';
        more.textContent = `${table.cells.length} of ${table.rows} rows shown`;
        holder.append(more);
    }
    return holder;
};

const textInput = (className: string, label: string): HTMLInputElement => {
    const input = document.createElement('input');
    input.type = 'text';
    input.className = className;
    input.spellcheck = false;
    input.autocomplete = 'off';
    input.setAttribute('aria-label', label);
    return input;
};

const element = <K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className: string,
    text = ''
): HTMLElementTagNameMap[K] => {
    const node = document.createElement(tag);
    node.className = className;
    node.textContent = text;
    return node;
};

const MOVES: Readonly<Record<string, readonly [number, number]>> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1]
};

export const createUserSheet = (
    interpreter: AjisaiInterpreter,
    container: HTMLElement,
    callbacks: UserSheetCallbacks
): UserSheet => {
    const { onWordClick, onDictionaryChanged, onLayoutChanged, showInfo, showError, runExclusive } = callbacks;
    // A cell's value under its content identity, which changes whenever the
    // Word or anything it calls is redefined: an unchanged cell is not run
    // again when the sheet is redrawn.
    let values = new Map<string, Promise<SheetCellValue>>();
    let generation = 0;
    let layout: Record<string, string> = {};
    let words: UserWordInfo[] = [];
    let selected = 'A1';
    // The Word in each cell, by address, as the last render placed them.
    let byAddress = new Map<string, string>();
    let extent = { columns: 6, rows: 12 };

    // The formula bar: the name box, the formula, and what the cell holds.
    const bar = element('div', 'sheet-bar');
    const nameInput = textInput('sheet-name-box', 'Cell name');
    const formulaInput = textInput('sheet-formula', 'Formula');
    const details = element('div', 'sheet-details');
    // The selected cell's value laid out as a table, when it is one.
    const tableView = element('div', 'sheet-table-slot');
    bar.append(nameInput, formulaInput, details, tableView);
    const gridHolder = element('div', 'sheet-grid-holder');
    container.replaceChildren(bar, gridHolder);

    const wordAt = (address: string): string | undefined => byAddress.get(address);
    const infoOf = (name: string | undefined): SheetCellInfo | null =>
        name === undefined ? null : interpreter.describe_sheet_cell(name);

    const report = (result: ExecuteResult, fallback: string): boolean => {
        if (!isFailure(result)) return true;
        showInfo(result.message || fallback, false);
        return false;
    };

    const edit = async (task: () => Promise<ExecuteResult>, fallback: string): Promise<boolean> => {
        try {
            const ok = report(await runExclusive(task), fallback);
            if (ok) await onDictionaryChanged();
            return ok;
        } catch (error) {
            showError(toError(error));
            return false;
        }
    };

    const checkName = (name: string): boolean => {
        if (isSheetName(name)) return true;
        showInfo(`'${name}' cannot name a cell: a name is one word with no quote, bracket or #.`, false);
        return false;
    };

    const define = (name: string, formula: string, contract: string | null): Promise<boolean> => {
        if (!checkName(name)) return Promise.resolve(false);
        if (!formula.trim()) {
            showInfo(`The formula of '${name}' is empty.`, false);
            return Promise.resolve(false);
        }
        return edit(
            () => runSheetEdit(interpreter, sheetDefinitionSource(name, formula, contract)),
            `Could not define '${name}'.`
        );
    };

    const place = (name: string, address: string): void => {
        layout = { ...layout, [name]: address };
        onLayoutChanged();
    };

    // Whether the cell now holds what the formula bar says.
    const commitFormula = async (): Promise<boolean> => {
        const address = selected;
        const name = wordAt(address);
        const formula = formulaInput.value;
        if (name !== undefined) {
            const info = infoOf(name);
            if (!info || formula.trim() === info.formula.trim()) return true;
            const ok = await define(name, formula, interpreter.lookup_word_description(name));
            if (!ok) formulaInput.value = info.formula;
            return ok;
        }
        if (!formula.trim()) return true;
        const wanted = (nameInput.value.trim() || address).toUpperCase();
        const elsewhere = layout[wanted];
        if (infoOf(wanted) && elsewhere !== undefined && elsewhere !== address) {
            showInfo(`'${wanted}' is already the cell at ${elsewhere}; edit it there or name this cell otherwise.`, false);
            return false;
        }
        // Placed first, so the redraw the DEF causes draws it here.
        const before = layout;
        layout = { ...layout, [wanted]: address };
        const ok = await define(wanted, formula, null);
        if (ok) onLayoutChanged();
        else layout = before;
        return ok;
    };

    const commitName = async (): Promise<void> => {
        const address = selected;
        const from = wordAt(address);
        const to = nameInput.value.trim().toUpperCase();
        if (from === undefined) {
            if (formulaInput.value.trim()) await commitFormula();
            return;
        }
        if (!to || to === from) {
            nameInput.value = from;
            return;
        }
        const info = infoOf(from);
        if (!info || !checkName(to)) return refreshBar();
        if (infoOf(to)) {
            showInfo(`'${to}' already names a Word.`, false);
            return refreshBar();
        }
        const contract = interpreter.lookup_word_description(from);
        const before = layout;
        const rest = { ...layout };
        delete rest[from];
        layout = { ...rest, [to]: address };
        const ok = await edit(
            () => renameSheetWord(interpreter, from, to, info.formula, contract),
            `Could not rename '${from}'.`
        );
        if (ok) onLayoutChanged();
        else {
            layout = before;
            refreshBar();
        }
    };

    const deleteSelected = async (): Promise<void> => {
        const name = wordAt(selected);
        if (name === undefined) return;
        if (await edit(() => deleteUserWord(interpreter, name), `Could not delete '${name}'.`)) {
            showInfo(`Word '${name}' deleted`, true);
        }
    };

    const showValue = (
        target: HTMLElement,
        name: string,
        info: SheetCellInfo,
        onValue?: (value: SheetCellValue) => void,
        tableAsShape = false
    ): void => {
        if (!(info.inputs === 0 && info.outputs === 1)) {
            target.textContent = describeEffect(info.inputs, info.outputs);
            target.classList.add('is-muted');
            return;
        }
        target.textContent = '…';
        const key = info.identity ?? `name:${name}`;
        let pending = values.get(key);
        if (!pending) {
            pending = runExclusive(() => interpreter.evaluate_sheet_cell(name));
            values.set(key, pending);
        }
        const current = generation;
        void pending.then((result) => {
            if (current !== generation) return;
            target.classList.toggle('is-error', result.error !== undefined);
            target.textContent = tableAsShape && result.table
                ? describeTable(result.table)
                : result.error ?? result.value;
            onValue?.(result);
        });
    };

    // The formula bar for the selected cell. A field being typed into is
    // left alone, so a redraw does not take the text from under the cursor.
    const refreshBar = (): void => {
        const name = wordAt(selected);
        const info = infoOf(name);
        const typing = document.activeElement;
        nameInput.placeholder = selected;
        if (typing !== nameInput) nameInput.value = name ?? '';
        if (typing !== formulaInput) formulaInput.value = info?.formula ?? '';
        formulaInput.placeholder = name === undefined
            ? 'formula, e.g. [ 120 80 300 ] or PRICE 3 MUL'
            : '';
        details.replaceChildren();
        tableView.replaceChildren();
        if (name === undefined || !info) {
            details.append(element('span', 'sheet-address', selected));
            return;
        }
        const insert = element('button', 'sheet-insert', `${name} → Input`);
        insert.type = 'button';
        insert.title = 'Write this word into the Input';
        insert.addEventListener('click', () => onWordClick(name));
        const value = element('span', 'sheet-detail-value');
        const type = element('span', 'sheet-type', info.signature ?? '');
        details.append(element('span', 'sheet-address', selected), value, type, insert);
        const shownFor = selected;
        showValue(value, name, info, (result) => {
            if (selected !== shownFor) return;
            if (result.table) {
                value.textContent = describeTable(result.table);
                tableView.replaceChildren(tableElement(result.table));
            }
            if (result.error !== undefined || !result.suggestedType || info.signature) return;
            const suggest = element('button', 'sheet-type-suggestion', `${result.suggestedType}?`);
            suggest.type = 'button';
            suggest.title = `Declare ${name} as -> ${result.suggestedType} (adds a #:contract line)`;
            suggest.addEventListener('click', () => {
                void define(name, info.formula, `-> ${result.suggestedType}`);
            });
            type.replaceChildren(suggest);
        });
    };

    const cellElement = (address: string): HTMLElement | null =>
        gridHolder.querySelector<HTMLElement>(`[data-address="${address}"]`);

    const select = (address: string, focus = true): void => {
        cellElement(selected)?.classList.remove('is-selected');
        cellElement(selected)?.setAttribute('tabindex', '-1');
        selected = address;
        const target = cellElement(address);
        target?.classList.add('is-selected');
        target?.setAttribute('tabindex', '0');
        if (focus) target?.focus();
        refreshBar();
    };

    const move = (dx: number, dy: number): void => {
        const at = parseAddress(selected) ?? { column: 0, row: 0 };
        const column = Math.min(Math.max(at.column + dx, 0), extent.columns - 1);
        const row = Math.min(Math.max(at.row + dy, 0), extent.rows - 1);
        select(formatAddress({ column, row }));
    };

    const onCellKey = (event: KeyboardEvent): void => {
        const step = MOVES[event.key];
        if (step) {
            event.preventDefault();
            move(step[0], step[1]);
        } else if (event.key === 'Enter' || event.key === 'F2') {
            event.preventDefault();
            formulaInput.focus();
        } else if (event.key === 'Delete') {
            event.preventDefault();
            void deleteSelected();
        } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
            // Typing into a selected cell starts its formula, as a spreadsheet does.
            event.preventDefault();
            formulaInput.value = event.key;
            formulaInput.focus();
        }
    };

    const gridCell = (address: string): HTMLTableCellElement => {
        const td = element('td', 'sheet-cell');
        td.dataset.address = address;
        td.tabIndex = address === selected ? 0 : -1;
        td.classList.toggle('is-selected', address === selected);
        const name = wordAt(address);
        const info = infoOf(name);
        // While a formula is being written, a click on a cell writes that
        // cell's name into it, and the formula keeps the focus.
        td.addEventListener('mousedown', (event) => {
            const other = wordAt(address);
            if (document.activeElement === formulaInput && other !== undefined && address !== selected) {
                event.preventDefault();
                const at = formulaInput.selectionStart ?? formulaInput.value.length;
                const end = formulaInput.selectionEnd ?? at;
                const before = formulaInput.value.slice(0, at);
                const pad = before === '' || before.endsWith(' ') ? '' : ' ';
                formulaInput.setRangeText(`${pad}${other} `, at, end, 'end');
            }
        });
        td.addEventListener('click', () => { if (document.activeElement !== formulaInput) select(address); });
        td.addEventListener('dblclick', () => formulaInput.focus());
        td.addEventListener('keydown', onCellKey);
        td.addEventListener('dragover', (event) => {
            if (wordAt(address) === undefined) event.preventDefault();
        });
        td.addEventListener('drop', (event) => {
            event.preventDefault();
            const moved = event.dataTransfer?.getData('text/plain');
            if (!moved || layout[moved] === undefined || wordAt(address) !== undefined) return;
            place(moved, address);
            selected = address;
            render(words);
        });
        if (name === undefined || !info) return td;
        td.classList.add('is-filled');
        td.draggable = true;
        td.addEventListener('dragstart', (event) => event.dataTransfer?.setData('text/plain', name));
        td.title = `${name}: ${info.formula}`;
        if (name !== address) td.append(element('span', 'sheet-cell-name', name));
        const value = element('span', 'sheet-cell-value');
        td.append(value);
        showValue(value, name, info, undefined, true);
        return td;
    };

    const render = (next: UserWordInfo[]): void => {
        words = next;
        generation += 1;
        const kept = values;
        layout = placeWords(next.map(([name]) => name), layout);
        byAddress = new Map(Object.entries(layout).map(([name, address]) => [address, name]));
        extent = gridExtent(layout);
        if (!parseAddress(selected)) selected = 'A1';

        const table = element('table', 'sheet-grid');
        const head = document.createElement('tr');
        head.append(element('th', 'sheet-corner'));
        for (let column = 0; column < extent.columns; column += 1) {
            const th = element('th', 'sheet-column', formatAddress({ column, row: 0 }).replace(/\d+$/, ''));
            th.scope = 'col';
            head.append(th);
        }
        const thead = document.createElement('thead');
        thead.append(head);
        const tbody = document.createElement('tbody');
        for (let row = 0; row < extent.rows; row += 1) {
            const tr = document.createElement('tr');
            const th = element('th', 'sheet-row', String(row + 1));
            th.scope = 'row';
            tr.append(th);
            for (let column = 0; column < extent.columns; column += 1) {
                tr.append(gridCell(formatAddress({ column, row })));
            }
            tbody.append(tr);
        }
        table.append(thead, tbody);
        // A redraw under a selected cell keeps the keyboard on the grid.
        const hadFocus = gridHolder.contains(document.activeElement);
        gridHolder.replaceChildren(table);
        if (hadFocus) cellElement(selected)?.focus();
        // Only values a cell still asks for are kept.
        const used = new Set(next.map(([name]) => infoOf(name)?.identity ?? `name:${name}`));
        values = new Map([...kept].filter(([key]) => used.has(key)));
        refreshBar();
    };

    // Escape puts back what the cell holds and returns to the grid; the text
    // is put back first, so leaving the field commits nothing.
    const refreshBarFrom = (field: HTMLInputElement): void => {
        const name = wordAt(selected);
        nameInput.value = name ?? '';
        formulaInput.value = infoOf(name)?.formula ?? '';
        field.blur();
        refreshBar();
        cellElement(selected)?.focus();
    };

    nameInput.addEventListener('keydown', (event) => {
        if (event.key === 'Enter') {
            event.preventDefault();
            void commitName();
        } else if (event.key === 'Escape') {
            refreshBarFrom(nameInput);
        }
    });
    nameInput.addEventListener('change', () => { void commitName(); });
    formulaInput.addEventListener('keydown', (event) => {
        if (event.key === 'Enter') {
            event.preventDefault();
            void commitFormula().then((ok) => { if (ok) cellElement(selected)?.focus(); });
        } else if (event.key === 'Escape') {
            refreshBarFrom(formulaInput);
        }
    });
    formulaInput.addEventListener('change', () => { void commitFormula(); });

    return {
        render,
        layout: () => layout,
        restoreLayout: (saved) => {
            layout = { ...saved };
        }
    };
};
