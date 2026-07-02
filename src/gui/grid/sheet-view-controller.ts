// Sheet view controller (redesign plan Phase 2): glues the UI-free engine
// (src/sheet/) to the grid UI and the interpreter. One table = one user
// dictionary (§1.1); Phase 2 hosts the single default table TABLE1.
//
// Responsibilities: apply cell edits to the engine, mirror the resulting
// word definitions into the interpreter through the forced host API,
// evaluate the recalculation plan serially on the worker path (main thread
// stays free; Escape/abort still works), and paint the results. Word
// bodies persist through the existing user-word persistence; grid-specific
// document state (raw texts, layout) is Phase 4 — on reload, cell raw text
// is reconstructed from the word bodies.

import {
    DEFAULT_GRID_LIMITS,
    type SheetGridLimits,
} from '../../sheet/cell-address';
import { reconstructCellText } from '../../sheet/formula-preprocessor';
import { SheetEngine, type RecalcPlan } from '../../sheet/sheet-engine';
import { SheetEvaluator, type CellEvaluationState } from '../../sheet/sheet-evaluator';
import { createInterpreterSnapshot } from '../../workers/interpreter-snapshot';
import { WORKER_MANAGER } from '../../workers/execution-worker-manager';
import { collectUserWords } from '../interpreter-execution-utils';
import type { AjisaiInterpreter } from '../../wasm-interpreter-types';
import { renderCellDisplay } from './cell-renderer';
import { createGridView, type GridView } from './grid-view';
import { parseCellRef, formatCellRef, isWithinLimits } from '../../sheet/cell-address';

export const DEFAULT_TABLE_NAME = 'TABLE1';

export interface SheetViewCallbacks {
    saveState(): Promise<void>;
    showError(error: Error | string): void;
}

export interface SheetViewController {
    /** Build the grid into `container`, restore existing cells, recalc. */
    init(container: HTMLElement): Promise<void>;
    /**
     * Re-evaluate every cell. The view switcher calls this when the shared
     * vocabulary changed while the sheet was hidden (a word was redefined
     * in the Script view), so word-dependent cells never show stale values.
     */
    refreshAllCells(): Promise<void>;
    focus(): void;
}

export function createSheetViewController(
    interpreter: AjisaiInterpreter,
    callbacks: SheetViewCallbacks,
    limits: SheetGridLimits = DEFAULT_GRID_LIMITS,
): SheetViewController {
    const engine = new SheetEngine(DEFAULT_TABLE_NAME, limits);
    /** Definition failures reported by the interpreter, keyed by fq word. */
    const defineFailures = new Map<string, string>();
    let grid: GridView | null = null;
    let nameBox: HTMLInputElement | null = null;
    let formulaInput: HTMLInputElement | null = null;
    /** Address the formula bar currently reflects. */
    let currentAddress = 'A1';
    /** The formula bar holds uncommitted user input. */
    let formulaDirty = false;

    const evaluator = new SheetEvaluator(async (fqName) => {
        // Empty stack + shared vocabulary (plan §2.1), forced greedy mode:
        // hedged racing per cell would double the worker load for no UX
        // gain, and serial inboxes must not be drained by cell recalc.
        const snapshot = createInterpreterSnapshot({
            stack: [],
            userWords: collectUserWords(interpreter),
            importedModules: interpreter.collect_imported_modules(),
            executionMode: 'greedy',
        });
        return WORKER_MANAGER.execute(fqName, snapshot);
    });

    const localAddress = (fqName: string): string | null => {
        const prefix = `${DEFAULT_TABLE_NAME}@`;
        return fqName.startsWith(prefix) ? fqName.slice(prefix.length) : null;
    };

    const resolveHostError = (fqName: string): string | null => {
        const failure = defineFailures.get(fqName);
        if (failure) return failure;
        const address = localAddress(fqName);
        if (address === null) return null;
        return engine.getCell(address)?.error ?? null;
    };

    const refreshFormulaBar = (address: string): void => {
        currentAddress = address;
        formulaDirty = false;
        if (nameBox) nameBox.value = address;
        if (formulaInput && document.activeElement !== formulaInput) {
            formulaInput.value = engine.getCell(address)?.rawText ?? '';
        }
    };

    const paintStates = (states: Map<string, CellEvaluationState>): void => {
        if (!grid) return;
        for (const [fqName, state] of states) {
            const address = localAddress(fqName);
            if (address === null) continue;
            let display = renderCellDisplay(state);
            // A text literal cell displays its own text: the host authored
            // the literal, so its text role is known here even though the
            // current protocol drops interpretation roles across word-call
            // boundaries (the Editor shows word-returned strings as byte
            // Vectors for the same reason — no heuristic decoding is done
            // for computed values).
            const content = engine.getCell(address)?.content;
            if (state.kind === 'value' && content?.kind === 'text') {
                display = { text: content.text, kind: 'text', detail: null };
            }
            grid.updateCell(address, display);
        }
    };

    const runPlan = async (plan: RecalcPlan): Promise<void> => {
        try {
            const changed = await evaluator.applyPlan(plan, { resolveHostError });
            paintStates(changed);
        } catch (error) {
            callbacks.showError(error as Error);
        }
    };

    const commitEdit = async (address: string, rawText: string): Promise<void> => {
        let update;
        try {
            update = engine.setCell(address, rawText);
        } catch (error) {
            callbacks.showError(error as Error);
            return;
        }

        if (update.remove !== null) {
            try {
                interpreter.remove_word(update.remove);
            } catch (error) {
                console.warn('[Sheet] Failed to remove cell word:', error);
            }
            evaluator.invalidate(update.remove);
            defineFailures.delete(update.remove);
            grid?.updateCell(address, renderCellDisplay(null));
        }

        if (update.define !== null) {
            const fqName = `${update.define.dictionary}@${update.define.wordName}`;
            try {
                interpreter.define_word_forced(
                    update.define.dictionary,
                    update.define.wordName,
                    update.define.bodySource,
                );
                defineFailures.delete(fqName);
            } catch (error) {
                defineFailures.set(fqName, String(error));
            }
        }

        refreshFormulaBar(address);
        await runPlan(update.plan);
        try {
            await callbacks.saveState();
        } catch (error) {
            console.warn('[Sheet] Failed to persist state:', error);
        }
    };

    const restoreCellsFromDictionary = (): void => {
        for (const [dictionary, name] of interpreter.collect_user_words_info()) {
            if (dictionary !== DEFAULT_TABLE_NAME) continue;
            const coord = parseCellRef(name);
            if (!coord || !isWithinLimits(coord, limits)) continue;
            const body = interpreter.lookup_word_definition(`${dictionary}@${name}`);
            if (!body) continue;
            const rawText = reconstructCellText(body);
            try {
                engine.setCell(name, rawText);
                grid?.revealAddress(name);
            } catch (error) {
                console.warn(`[Sheet] Failed to restore cell ${name}:`, error);
            }
        }
    };

    /** Move the selection one row down after a formula-bar commit. */
    const selectNextRowAfter = (address: string): void => {
        const coord = parseCellRef(address);
        if (!coord || !grid) return;
        const next = { col: coord.col, row: Math.min(limits.rows - 1, coord.row + 1) };
        grid.selectAddress(formatCellRef(next));
    };

    const commitFormulaBar = (moveDown: boolean): void => {
        if (!grid || !formulaInput) return;
        const address = currentAddress;
        if (!formulaDirty) {
            grid.focus();
            return;
        }
        formulaDirty = false;
        grid.clearPreview(address);
        void commitEdit(address, formulaInput.value);
        if (moveDown) selectNextRowAfter(address);
        grid.focus();
    };

    const cancelFormulaBar = (): void => {
        if (!grid || !formulaInput) return;
        formulaDirty = false;
        grid.clearPreview(currentAddress);
        formulaInput.value = engine.getCell(currentAddress)?.rawText ?? '';
        grid.focus();
    };

    // Google Sheets-style formula bar: a name box (selected address; typing
    // an address jumps to it), the fx affordance, and the editable formula
    // field that mirrors and edits the selected cell.
    const buildFormulaBar = (): HTMLElement => {
        const bar = document.createElement('div');
        bar.className = 'sheet-formula-bar';

        nameBox = document.createElement('input');
        nameBox.type = 'text';
        nameBox.className = 'sheet-name-box';
        nameBox.setAttribute('aria-label', 'セル位置（入力でジャンプ）');
        nameBox.spellcheck = false;
        nameBox.addEventListener('focus', () => nameBox?.select());
        nameBox.addEventListener('keydown', (event: KeyboardEvent) => {
            if (event.key === 'Enter') {
                event.preventDefault();
                const coord = parseCellRef(nameBox?.value.trim() ?? '');
                if (coord && isWithinLimits(coord, limits) && grid) {
                    grid.selectAddress(formatCellRef(coord));
                    grid.focus();
                } else if (nameBox) {
                    nameBox.value = currentAddress;
                }
            } else if (event.key === 'Escape') {
                event.preventDefault();
                if (nameBox) nameBox.value = currentAddress;
                grid?.focus();
            }
        });

        const fxLabel = document.createElement('span');
        fxLabel.className = 'sheet-fx-label';
        fxLabel.textContent = 'fx';
        fxLabel.setAttribute('aria-hidden', 'true');

        formulaInput = document.createElement('input');
        formulaInput.type = 'text';
        formulaInput.className = 'sheet-formula-input';
        formulaInput.setAttribute('aria-label', '選択セルの数式または値');
        formulaInput.spellcheck = false;
        formulaInput.addEventListener('input', () => {
            if (!formulaInput || !grid) return;
            formulaDirty = true;
            grid.previewCell(currentAddress, formulaInput.value);
        });
        formulaInput.addEventListener('keydown', (event: KeyboardEvent) => {
            if (event.key === 'Enter') {
                event.preventDefault();
                commitFormulaBar(true);
            } else if (event.key === 'Escape') {
                event.preventDefault();
                cancelFormulaBar();
            }
        });
        formulaInput.addEventListener('blur', () => {
            // Clicking elsewhere commits, exactly like the in-cell editor.
            if (formulaDirty) commitFormulaBar(false);
        });

        bar.append(nameBox, fxLabel, formulaInput);
        return bar;
    };

    const init = async (container: HTMLElement): Promise<void> => {
        grid = createGridView(limits, {
            onSelect: (address) => refreshFormulaBar(address),
            onCommitEdit: (address, rawText) => {
                void commitEdit(address, rawText);
            },
            resolveRawText: (address) => engine.getCell(address)?.rawText ?? '',
            onEditInput: (address, rawText) => {
                // Mirror in-cell typing into the formula bar (Google Sheets
                // keeps both surfaces live).
                if (formulaInput && document.activeElement !== formulaInput) {
                    formulaInput.value = rawText;
                }
                currentAddress = address;
            },
        });

        const canvas = document.createElement('div');
        canvas.className = 'sheet-canvas';
        const tableCard = document.createElement('div');
        tableCard.className = 'sheet-table-card';
        const tableName = document.createElement('div');
        tableName.className = 'sheet-table-name';
        tableName.textContent = DEFAULT_TABLE_NAME;
        tableCard.append(tableName, grid.element);
        canvas.appendChild(tableCard);

        container.textContent = '';
        container.append(buildFormulaBar(), canvas);

        restoreCellsFromDictionary();
        refreshFormulaBar(grid.extractSelectedAddress() ?? 'A1');
        await runPlan(engine.fullRecalcPlan());
    };

    return {
        init,
        refreshAllCells: () => runPlan(engine.fullRecalcPlan()),
        focus: () => grid?.focus(),
    };
}
