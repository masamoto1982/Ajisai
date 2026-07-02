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
import { parseCellRef, isWithinLimits } from '../../sheet/cell-address';

export const DEFAULT_TABLE_NAME = 'TABLE1';

export interface SheetViewCallbacks {
    saveState(): Promise<void>;
    showError(error: Error | string): void;
}

export interface SheetViewController {
    /** Build the grid into `container`, restore existing cells, recalc. */
    init(container: HTMLElement): Promise<void>;
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
    let statusAddress: HTMLElement | null = null;
    let statusRaw: HTMLElement | null = null;

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

    const refreshStatus = (address: string): void => {
        if (statusAddress) statusAddress.textContent = address;
        if (statusRaw) statusRaw.textContent = engine.getCell(address)?.rawText ?? '';
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

        refreshStatus(address);
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

    const init = async (container: HTMLElement): Promise<void> => {
        grid = createGridView(limits, {
            onSelect: (address) => refreshStatus(address),
            onCommitEdit: (address, rawText) => {
                void commitEdit(address, rawText);
            },
            resolveRawText: (address) => engine.getCell(address)?.rawText ?? '',
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

        const statusbar = document.createElement('div');
        statusbar.className = 'sheet-statusbar';
        statusAddress = document.createElement('span');
        statusAddress.className = 'sheet-status-address';
        statusRaw = document.createElement('code');
        statusRaw.className = 'sheet-status-raw';
        statusbar.append(statusAddress, statusRaw);

        container.textContent = '';
        container.append(statusbar, canvas);

        restoreCellsFromDictionary();
        refreshStatus(grid.extractSelectedAddress() ?? 'A1');
        await runPlan(engine.fullRecalcPlan());
    };

    return {
        init,
        focus: () => grid?.focus(),
    };
}
