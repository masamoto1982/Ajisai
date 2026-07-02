// Sheet view recalculation engine (docs/dev/ajisai-spreadsheet-app-redesign-plan.md §2.4).
//
// UI- and WASM-free: this module owns the cell table, the cell-to-cell
// dependency graph, dirty-set closure, topological recalculation order, and
// cycle detection. Evaluation itself (running a cell word on the
// interpreter) is a later phase; the engine only decides *what* to define
// and *in which order* to re-evaluate.
//
// Why the engine keeps its own dependency graph when the interpreter
// already maintains a reverse-dependency index: the interpreter records a
// dependency only when the referenced word resolves at definition time, but
// a spreadsheet formula routinely references a still-empty cell. The
// engine's graph is built from the formula *text* (preprocessor output), so
// `=B1` depends on SHEET@B1 even while B1 is undefined, and defining B1
// later dirties A1 as expected. The interpreter's index remains the source
// of truth for Editor-view words referencing cells and vice versa.

import {
    DEFAULT_GRID_LIMITS,
    SheetGridLimits,
    formatCellRef,
    isWithinLimits,
    parseCellRef,
} from './cell-address';
import {
    CellContent,
    classifyCellText,
    formatTextCellLiteral,
    preprocessFormula,
} from './formula-preprocessor';

export interface CellRecord {
    /** Canonical unqualified address on this sheet, e.g. 'A1'. */
    address: string;
    /** The text as typed by the user (formula text keeps its `=`). */
    rawText: string;
    content: CellContent;
    /**
     * Ajisai word body for `SHEET@address`, or null when the cell cannot be
     * defined (preprocessing error).
     */
    bodySource: string | null;
    /** Fully-qualified cell words this cell reads. */
    references: string[];
    /** Host-side preprocessing error, if any. */
    error: string | null;
}

/** One word (re)definition the host must apply to the interpreter. */
export interface CellWordDefinition {
    dictionary: string;
    wordName: string;
    bodySource: string;
}

/**
 * Recalculation plan for a change. `order` lists evaluable dirty cells in
 * dependency order (upstream first). Cells on a reference cycle are
 * rejected as 循環参照 (plan §2.4: the host refuses spreadsheet-style, even
 * though word recursion is legal in the language); cells that merely read a
 * cyclic cell are `blocked` — not evaluable, but not the cycle's fault.
 */
export interface RecalcPlan {
    /** Fully-qualified cell words in evaluation order. */
    order: string[];
    /** Fully-qualified cells participating in a reference cycle. */
    cyclic: string[];
    /** Fully-qualified cells depending (transitively) on a cyclic cell. */
    blocked: string[];
}

/** Result of a single cell edit. */
export interface SheetUpdate {
    /** The new record, or null when the edit cleared the cell. */
    record: CellRecord | null;
    /** Definition to apply via the forced host API, if any. */
    define: CellWordDefinition | null;
    /** Fully-qualified word to remove from the interpreter, if any. */
    remove: string | null;
    /** Recalculation plan for the edited cell and its dependents. */
    plan: RecalcPlan;
}

export class SheetEngine {
    readonly sheetName: string;
    readonly limits: SheetGridLimits;

    /** Cell records keyed by fully-qualified name (`SHEET@A1`). */
    private readonly cells = new Map<string, CellRecord>();
    /** cell → cells it reads. Keys are always known cells. */
    private readonly readsFrom = new Map<string, Set<string>>();
    /**
     * word → known cells that read it. Keys may be foreign (another sheet's
     * cells, still-empty cells, Editor words); values are always known cells,
     * so dirty-closure traversal stays inside this engine.
     */
    private readonly readBy = new Map<string, Set<string>>();

    constructor(sheetName = 'SHEET', limits: SheetGridLimits = DEFAULT_GRID_LIMITS) {
        this.sheetName = sheetName.toUpperCase();
        this.limits = limits;
    }

    /** Canonical fully-qualified word name for an address on this sheet. */
    qualifyAddress(address: string): string {
        return `${this.sheetName}@${this.normalizeAddress(address)}`;
    }

    private normalizeAddress(address: string): string {
        const coord = parseCellRef(address);
        if (!coord || !isWithinLimits(coord, this.limits)) {
            throw new RangeError(`not a cell address on this sheet: ${address}`);
        }
        return formatCellRef(coord);
    }

    getCell(address: string): CellRecord | null {
        return this.cells.get(this.qualifyAddress(address)) ?? null;
    }

    /** Addresses of all non-empty cells, for persistence and full recalc. */
    cellAddresses(): string[] {
        return [...this.cells.values()].map((record) => record.address);
    }

    /**
     * Apply one cell edit: classify the text, build the word body, update
     * the dependency graph, and return what the host must do (define or
     * remove a word, then re-evaluate `plan.order`).
     */
    setCell(address: string, rawText: string): SheetUpdate {
        const normalized = this.normalizeAddress(address);
        const fqName = `${this.sheetName}@${normalized}`;
        const content = classifyCellText(rawText);

        if (content.kind === 'empty') {
            const existed = this.cells.delete(fqName);
            this.replaceReads(fqName, []);
            return {
                record: null,
                define: null,
                remove: existed ? fqName : null,
                plan: this.recalcPlan([fqName], { includeChanged: false }),
            };
        }

        const record = this.buildRecord(normalized, rawText, content);
        this.cells.set(fqName, record);
        this.replaceReads(fqName, record.references);

        return {
            record,
            define:
                record.bodySource !== null
                    ? {
                          dictionary: this.sheetName,
                          wordName: normalized,
                          bodySource: record.bodySource,
                      }
                    : null,
            remove: null,
            plan: this.recalcPlan([fqName]),
        };
    }

    private buildRecord(address: string, rawText: string, content: CellContent): CellRecord {
        switch (content.kind) {
            case 'number':
                // Scalars are 1-element Vectors by convention (plan §2.1).
                return {
                    address,
                    rawText,
                    content,
                    bodySource: `[ ${content.literal} ]`,
                    references: [],
                    error: null,
                };
            case 'text': {
                const formatted = formatTextCellLiteral(content.text);
                return {
                    address,
                    rawText,
                    content,
                    bodySource: formatted.literal,
                    references: [],
                    error: formatted.error,
                };
            }
            case 'formula': {
                const preprocessed = preprocessFormula(content.formulaSource, {
                    sheetName: this.sheetName,
                    limits: this.limits,
                });
                return {
                    address,
                    rawText,
                    content,
                    bodySource: preprocessed.error === null ? preprocessed.source : null,
                    references: preprocessed.references,
                    error: preprocessed.error,
                };
            }
            case 'empty':
                throw new Error('empty content has no record');
        }
    }

    private replaceReads(fqName: string, references: string[]): void {
        const previous = this.readsFrom.get(fqName);
        if (previous) {
            for (const target of previous) {
                const readers = this.readBy.get(target);
                if (readers) {
                    readers.delete(fqName);
                    if (readers.size === 0) {
                        this.readBy.delete(target);
                    }
                }
            }
        }
        if (references.length === 0) {
            this.readsFrom.delete(fqName);
            return;
        }
        const reads = new Set(references);
        this.readsFrom.set(fqName, reads);
        for (const target of reads) {
            let readers = this.readBy.get(target);
            if (!readers) {
                readers = new Set();
                this.readBy.set(target, readers);
            }
            readers.add(fqName);
        }
    }

    /**
     * Dirty closure + evaluation order for a set of changed words. The
     * changed words may be foreign (an Editor word or another sheet's cell
     * that this sheet reads); only this engine's cells enter the plan.
     * Mirrors `Interpreter::collect_transitive_dependents` (BFS over the
     * reverse index), then orders the dirty subgraph with Kahn's algorithm;
     * Tarjan SCCs split the unorderable remainder into cycle members and
     * cells blocked behind a cycle.
     */
    recalcPlan(
        changedWords: string[],
        { includeChanged = true }: { includeChanged?: boolean } = {},
    ): RecalcPlan {
        const dirty = new Set<string>();
        const queue: string[] = [];
        for (const word of changedWords) {
            if (includeChanged && this.cells.has(word)) {
                dirty.add(word);
            }
            queue.push(word);
        }
        while (queue.length > 0) {
            const current = queue.pop() as string;
            for (const reader of this.readBy.get(current) ?? []) {
                if (!dirty.has(reader)) {
                    dirty.add(reader);
                    queue.push(reader);
                }
            }
        }
        return this.orderDirtySet(dirty);
    }

    /** Plan re-evaluating every cell on the sheet (initial load, restore). */
    fullRecalcPlan(): RecalcPlan {
        return this.orderDirtySet(new Set(this.cells.keys()));
    }

    private orderDirtySet(dirty: Set<string>): RecalcPlan {
        // Kahn's algorithm over the dirty subgraph. Dependencies outside the
        // dirty set are clean (or foreign) and count as already satisfied.
        const inDegree = new Map<string, number>();
        for (const cell of dirty) {
            let degree = 0;
            for (const target of this.readsFrom.get(cell) ?? []) {
                if (dirty.has(target) && target !== cell) {
                    degree++;
                } else if (target === cell) {
                    degree++; // self-reference: never satisfiable
                }
            }
            inDegree.set(cell, degree);
        }

        const ready: string[] = [];
        for (const [cell, degree] of inDegree) {
            if (degree === 0) {
                ready.push(cell);
            }
        }
        // Deterministic order among independent cells.
        ready.sort();

        const order: string[] = [];
        while (ready.length > 0) {
            const cell = ready.shift() as string;
            order.push(cell);
            const nextReady: string[] = [];
            for (const reader of this.readBy.get(cell) ?? []) {
                const degree = inDegree.get(reader);
                if (degree !== undefined) {
                    inDegree.set(reader, degree - 1);
                    if (degree - 1 === 0) {
                        nextReady.push(reader);
                    }
                }
            }
            nextReady.sort();
            for (const cell2 of nextReady) {
                insertSorted(ready, cell2);
            }
        }

        const ordered = new Set(order);
        const leftover = [...dirty].filter((cell) => !ordered.has(cell));
        if (leftover.length === 0) {
            return { order, cyclic: [], blocked: [] };
        }

        const cyclicSet = this.findCycleMembers(new Set(leftover));
        const cyclic = leftover.filter((cell) => cyclicSet.has(cell)).sort();
        const blocked = leftover.filter((cell) => !cyclicSet.has(cell)).sort();
        return { order, cyclic, blocked };
    }

    /**
     * Tarjan strongly-connected components over the leftover subgraph.
     * Members of an SCC of size > 1, or with a self-edge, are true cycle
     * participants; the rest of the leftover merely sits downstream.
     */
    private findCycleMembers(nodes: Set<string>): Set<string> {
        const indexOf = new Map<string, number>();
        const lowLink = new Map<string, number>();
        const onStack = new Set<string>();
        const stack: string[] = [];
        const cyclic = new Set<string>();
        let nextIndex = 0;

        const edgesFrom = (node: string): string[] =>
            [...(this.readsFrom.get(node) ?? [])].filter((target) => nodes.has(target));

        const strongConnect = (root: string): void => {
            // Iterative Tarjan: each frame is [node, iterator position].
            const frames: Array<{ node: string; targets: string[]; next: number }> = [];
            const openFrame = (node: string): void => {
                indexOf.set(node, nextIndex);
                lowLink.set(node, nextIndex);
                nextIndex++;
                stack.push(node);
                onStack.add(node);
                frames.push({ node, targets: edgesFrom(node), next: 0 });
            };
            openFrame(root);

            while (frames.length > 0) {
                const frame = frames[frames.length - 1] as {
                    node: string;
                    targets: string[];
                    next: number;
                };
                if (frame.next < frame.targets.length) {
                    const target = frame.targets[frame.next] as string;
                    frame.next++;
                    if (!indexOf.has(target)) {
                        openFrame(target);
                    } else if (onStack.has(target)) {
                        lowLink.set(
                            frame.node,
                            Math.min(
                                lowLink.get(frame.node) as number,
                                indexOf.get(target) as number,
                            ),
                        );
                    }
                    continue;
                }

                frames.pop();
                const parent = frames[frames.length - 1];
                if (parent) {
                    lowLink.set(
                        parent.node,
                        Math.min(
                            lowLink.get(parent.node) as number,
                            lowLink.get(frame.node) as number,
                        ),
                    );
                }
                if (lowLink.get(frame.node) === indexOf.get(frame.node)) {
                    const component: string[] = [];
                    let member: string;
                    do {
                        member = stack.pop() as string;
                        onStack.delete(member);
                        component.push(member);
                    } while (member !== frame.node);
                    const selfEdge =
                        component.length === 1 &&
                        (this.readsFrom.get(frame.node)?.has(frame.node) ?? false);
                    if (component.length > 1 || selfEdge) {
                        for (const cell of component) {
                            cyclic.add(cell);
                        }
                    }
                }
            }
        };

        for (const node of nodes) {
            if (!indexOf.has(node)) {
                strongConnect(node);
            }
        }
        return cyclic;
    }
}

/** Insert into a sorted array keeping it sorted (small ready-lists only). */
function insertSorted(sorted: string[], value: string): void {
    let low = 0;
    let high = sorted.length;
    while (low < high) {
        const mid = (low + high) >> 1;
        if ((sorted[mid] as string) < value) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    sorted.splice(low, 0, value);
}
