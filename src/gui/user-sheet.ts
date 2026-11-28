// The Sheet view of the User dictionary (spec/gui-semantics.md, Sheet): each
// User Word is a row — its name, its body as the formula, and, for a Word that
// takes nothing and leaves one value, that value — like the named cells of a
// spreadsheet. The sheet keeps no state of its own: a row is the dictionary's
// Word, editing a formula runs `DEF`, and a new row is a new `DEF`.

import type { AjisaiInterpreter, ExecuteResult, SheetCellValue, UserWordInfo } from '../wasm-interpreter-types';
import { isFailure, toError } from './interpreter-execution-utils';

export interface UserSheetCallbacks {
    readonly onWordClick: (word: string) => void;
    readonly onDictionaryChanged: () => Promise<void>;
    readonly showInfo: (text: string, append: boolean) => void;
    readonly showError: (error: Error) => void;
    // Every call into the interpreter takes its turn with runs.
    readonly runExclusive: <T>(task: () => Promise<T>) => Promise<T>;
}

export interface UserSheet {
    readonly render: (words: UserWordInfo[]) => void;
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

/** A name a sheet row may take: one token, which a quoted String can hold. */
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

const cell = (tag: 'td' | 'th', className: string, text = ''): HTMLTableCellElement => {
    const element = document.createElement(tag);
    element.className = className;
    element.textContent = text;
    return element;
};

const formulaInput = (value: string, label: string): HTMLInputElement => {
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'sheet-formula';
    input.value = value;
    input.spellcheck = false;
    input.autocomplete = 'off';
    input.setAttribute('aria-label', label);
    return input;
};

const describeEffect = (inputs: number | null, outputs: number | null): string =>
    inputs === null || outputs === null
        ? 'variable stack effect'
        : `takes ${inputs}, leaves ${outputs}`;

export const createUserSheet = (
    interpreter: AjisaiInterpreter,
    container: HTMLElement,
    callbacks: UserSheetCallbacks
): UserSheet => {
    const { onWordClick, onDictionaryChanged, showInfo, showError, runExclusive } = callbacks;
    // A cell's value under its content identity, which changes whenever the
    // Word or anything it calls is redefined: an unchanged cell is not run
    // again when the sheet is redrawn.
    let values = new Map<string, Promise<SheetCellValue>>();
    let generation = 0;

    const define = async (name: string, formula: string, contract: string | null): Promise<boolean> => {
        if (!isSheetName(name)) {
            showInfo(`'${name}' cannot name a cell: a name is one word with no quote, bracket or #.`, false);
            return false;
        }
        if (!formula.trim()) {
            showInfo(`The formula of '${name}' is empty.`, false);
            return false;
        }
        try {
            const result = await runExclusive(() =>
                runSheetEdit(interpreter, sheetDefinitionSource(name, formula, contract))
            );
            if (isFailure(result)) {
                showInfo(result.message || `Could not define '${name}'.`, false);
                return false;
            }
            await onDictionaryChanged();
            return true;
        } catch (error) {
            showError(toError(error));
            return false;
        }
    };

    const valueOf = (name: string, identity: string | null): Promise<SheetCellValue> => {
        const key = identity ?? `name:${name}`;
        let pending = values.get(key);
        if (!pending) {
            pending = runExclusive(() => interpreter.evaluate_sheet_cell(name));
            values.set(key, pending);
        }
        return pending;
    };

    const wordRow = (name: string, kept: Map<string, Promise<SheetCellValue>>, current: number): HTMLTableRowElement | null => {
        const info = interpreter.describe_sheet_cell(name);
        if (!info) return null;
        const row = document.createElement('tr');

        const nameCell = cell('th', 'sheet-name');
        nameCell.scope = 'row';
        const nameButton = document.createElement('button');
        nameButton.type = 'button';
        nameButton.className = 'sheet-name-button';
        nameButton.textContent = name;
        nameButton.title = 'Write this word into the Input';
        nameButton.addEventListener('click', () => onWordClick(name));
        nameCell.appendChild(nameButton);

        const formulaCell = cell('td', 'sheet-formula-cell');
        const input = formulaInput(info.formula, `Formula of ${name}`);
        const commit = (): void => {
            if (input.value.trim() === info.formula.trim()) return;
            const contract = interpreter.lookup_word_description(name);
            void define(name, input.value, contract).then((ok) => {
                if (!ok) input.value = info.formula;
            });
        };
        input.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
                event.preventDefault();
                commit();
            } else if (event.key === 'Escape') {
                input.value = info.formula;
                input.blur();
            }
        });
        input.addEventListener('change', commit);
        formulaCell.appendChild(input);

        const valueCell = cell('td', 'sheet-value');
        const typeCell = cell('td', 'sheet-type', info.signature ?? '');

        if (info.inputs === 0 && info.outputs === 1) {
            valueCell.textContent = '…';
            const key = info.identity ?? `name:${name}`;
            const pending = valueOf(name, info.identity);
            kept.set(key, pending);
            void pending.then((result) => {
                if (current !== generation) return;
                if (result.error !== undefined) {
                    valueCell.textContent = result.error;
                    valueCell.classList.add('is-error');
                    return;
                }
                valueCell.textContent = result.value;
                if (result.suggestedType && !info.signature) {
                    const suggest = document.createElement('button');
                    suggest.type = 'button';
                    suggest.className = 'sheet-type-suggestion';
                    suggest.textContent = `${result.suggestedType}?`;
                    suggest.title = `Declare ${name} as -> ${result.suggestedType} (adds a #:contract line)`;
                    suggest.addEventListener('click', () => {
                        void define(name, info.formula, `-> ${result.suggestedType}`);
                    });
                    typeCell.replaceChildren(suggest);
                }
            });
        } else {
            valueCell.textContent = describeEffect(info.inputs, info.outputs);
            valueCell.classList.add('is-muted');
        }

        row.append(nameCell, formulaCell, valueCell, typeCell);
        return row;
    };

    const newRow = (): HTMLTableRowElement => {
        const row = document.createElement('tr');
        row.className = 'sheet-new-row';
        const nameCell = cell('th', 'sheet-name');
        nameCell.scope = 'row';
        const nameInput = formulaInput('', 'New cell name');
        nameInput.classList.add('sheet-new-name');
        nameInput.placeholder = 'NAME';
        nameCell.appendChild(nameInput);
        const formulaCell = cell('td', 'sheet-formula-cell');
        const input = formulaInput('', 'New cell formula');
        input.placeholder = 'formula, e.g. [ 120 80 300 ] or PRICE 3 MUL';
        formulaCell.appendChild(input);
        const add = (): void => {
            const name = nameInput.value.trim();
            if (!name || !input.value.trim()) return;
            void define(name, input.value, null);
        };
        for (const field of [nameInput, input]) {
            field.addEventListener('keydown', (event) => {
                if (event.key === 'Enter') {
                    event.preventDefault();
                    add();
                }
            });
        }
        const actionCell = cell('td', 'sheet-value');
        const addButton = document.createElement('button');
        addButton.type = 'button';
        addButton.className = 'btn';
        addButton.textContent = 'Add';
        addButton.addEventListener('click', add);
        actionCell.appendChild(addButton);
        row.append(nameCell, formulaCell, actionCell, cell('td', 'sheet-type'));
        return row;
    };

    const render = (words: UserWordInfo[]): void => {
        generation += 1;
        const current = generation;
        const kept = new Map<string, Promise<SheetCellValue>>();
        const table = document.createElement('table');
        table.className = 'sheet-table';
        const head = document.createElement('tr');
        for (const title of ['Name', 'Formula', 'Value', 'Type']) {
            const th = cell('th', title === 'Name' ? 'sheet-heading sheet-name' : 'sheet-heading', title);
            th.scope = 'col';
            head.appendChild(th);
        }
        const thead = document.createElement('thead');
        thead.appendChild(head);
        const tbody = document.createElement('tbody');
        for (const [name] of words) {
            const row = wordRow(name, kept, current);
            if (row) tbody.appendChild(row);
        }
        tbody.appendChild(newRow());
        table.append(thead, tbody);
        container.replaceChildren(table);
        values = kept;
    };

    return { render };
};
