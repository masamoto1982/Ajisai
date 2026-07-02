// Sheet view recalculation engine tests (plan §7: トポソート・循環検出・
// dirty 閉包 unit tests). The engine is pure host logic: no DOM, no WASM.

import { describe, expect, test } from 'vitest';
import { SheetEngine } from './sheet-engine';

describe('cell edits become word definitions (plan §1: セル＝ワード)', () => {
    test('a number cell defines a 1-element Vector body in the sheet dictionary', () => {
        const engine = new SheetEngine();
        const update = engine.setCell('A1', '42');
        expect(update.define).toEqual({
            dictionary: 'SHEET',
            wordName: 'A1',
            bodySource: '[ 42 ]',
        });
        expect(update.remove).toBeNull();
        expect(update.plan).toEqual({ order: ['SHEET@A1'], cyclic: [], blocked: [] });
    });

    test('a text cell defines a string literal body', () => {
        const engine = new SheetEngine();
        const update = engine.setCell('B2', 'hello');
        expect(update.define?.bodySource).toBe("'hello'");
    });

    test('a formula cell defines the preprocessed body and records references', () => {
        const engine = new SheetEngine();
        const update = engine.setCell('C1', '= A1 B1 +');
        expect(update.define?.bodySource).toBe(' SHEET@A1 SHEET@B1 +');
        expect(update.record?.references).toEqual(['SHEET@A1', 'SHEET@B1']);
    });

    test('cell addresses are normalized: a1 and A1 are the same cell', () => {
        const engine = new SheetEngine();
        engine.setCell('a1', '1');
        const record = engine.getCell('A1');
        expect(record?.rawText).toBe('1');
        expect(engine.cellAddresses()).toEqual(['A1']);
    });

    test('an invalid or out-of-grid address is rejected', () => {
        const engine = new SheetEngine();
        expect(() => engine.setCell('SUM', '1')).toThrow(RangeError);
        expect(() => engine.setCell('A1001', '1')).toThrow(RangeError);
    });

    test('an unrepresentable text cell reports an error and defines nothing', () => {
        const engine = new SheetEngine();
        const update = engine.setCell('A1', "quote' end");
        expect(update.define).toBeNull();
        expect(update.record?.error).toMatch(/cannot be represented/);
    });

    test('clearing a cell removes its word and dirties its readers', () => {
        const engine = new SheetEngine();
        engine.setCell('A1', '1');
        engine.setCell('B1', '= A1');
        const update = engine.setCell('A1', '');
        expect(update.record).toBeNull();
        expect(update.remove).toBe('SHEET@A1');
        expect(update.plan.order).toEqual(['SHEET@B1']);
        expect(engine.getCell('A1')).toBeNull();
    });

    test('clearing an already-empty cell removes nothing', () => {
        const engine = new SheetEngine();
        expect(engine.setCell('A1', '   ').remove).toBeNull();
    });
});

describe('dirty closure (plan §2.4: collect_transitive_dependents 相当)', () => {
    test('editing a cell dirties its transitive readers in dependency order', () => {
        const engine = new SheetEngine();
        engine.setCell('A1', '1');
        engine.setCell('B1', '= A1 [ 2 ] *');
        engine.setCell('C1', '= B1 [ 3 ] +');
        engine.setCell('D1', '= C1');

        const update = engine.setCell('A1', '10');
        expect(update.plan.order).toEqual(['SHEET@A1', 'SHEET@B1', 'SHEET@C1', 'SHEET@D1']);
    });

    test('unrelated cells stay out of the dirty set (最小再計算)', () => {
        const engine = new SheetEngine();
        engine.setCell('A1', '1');
        engine.setCell('B1', '= A1');
        engine.setCell('Z9', '= Z8');

        const update = engine.setCell('A1', '2');
        expect(update.plan.order).toEqual(['SHEET@A1', 'SHEET@B1']);
    });

    test('a reference to a still-empty cell is a live dependency', () => {
        const engine = new SheetEngine();
        engine.setCell('B1', '= A1');
        // A1 was empty when B1 was defined; defining it must dirty B1.
        const update = engine.setCell('A1', '5');
        expect(update.plan.order).toEqual(['SHEET@A1', 'SHEET@B1']);
    });

    test('a diamond dependency evaluates each cell once, upstream first', () => {
        const engine = new SheetEngine();
        engine.setCell('A1', '1');
        engine.setCell('B1', '= A1');
        engine.setCell('B2', '= A1');
        engine.setCell('C1', '= B1 B2 +');

        const { plan } = engine.setCell('A1', '2');
        expect(plan.order).toEqual(['SHEET@A1', 'SHEET@B1', 'SHEET@B2', 'SHEET@C1']);
        expect(plan.cyclic).toEqual([]);
    });

    test('a range reference dirties on any member edit', () => {
        const engine = new SheetEngine();
        engine.setCell('C1', '= A1:A3 [ 0 ] { + } FOLD');
        const update = engine.setCell('A2', '7');
        expect(update.plan.order).toEqual(['SHEET@A2', 'SHEET@C1']);
    });

    test('recalcPlan accepts a foreign word (Editor ワード再定義 → 依存セル)', () => {
        const engine = new SheetEngine();
        engine.setCell('A1', '= B1 TAX');
        engine.setCell('A2', '= A1');

        const plan = engine.recalcPlan(['SHEET@B1']);
        expect(plan.order).toEqual(['SHEET@A1', 'SHEET@A2']);
    });

    test('rewriting a formula releases its old dependencies', () => {
        const engine = new SheetEngine();
        engine.setCell('A1', '1');
        engine.setCell('B1', '= A1');
        engine.setCell('B1', '= C1');

        const update = engine.setCell('A1', '2');
        expect(update.plan.order).toEqual(['SHEET@A1']);
    });
});

describe('cycle detection (plan §2.4: セル間参照の循環はホストが拒否)', () => {
    test('a direct two-cell cycle is reported, not ordered', () => {
        const engine = new SheetEngine();
        engine.setCell('A1', '= B1');
        const update = engine.setCell('B1', '= A1');

        expect(update.plan.order).toEqual([]);
        expect(update.plan.cyclic).toEqual(['SHEET@A1', 'SHEET@B1']);
        expect(update.plan.blocked).toEqual([]);
    });

    test('a self-reference is a cycle', () => {
        const engine = new SheetEngine();
        const update = engine.setCell('A1', '= A1 [ 1 ] +');
        expect(update.plan.cyclic).toEqual(['SHEET@A1']);
    });

    test('cells behind a cycle are blocked, not cyclic', () => {
        const engine = new SheetEngine();
        engine.setCell('A1', '= B1');
        engine.setCell('C1', '= A1');
        const update = engine.setCell('B1', '= A1');

        expect(update.plan.cyclic).toEqual(['SHEET@A1', 'SHEET@B1']);
        expect(update.plan.blocked).toEqual(['SHEET@C1']);
        expect(update.plan.order).toEqual([]);
    });

    test('cells upstream of the cycle still evaluate', () => {
        const engine = new SheetEngine();
        engine.setCell('B1', '= A1 C1 +');
        engine.setCell('C1', '= B1');
        const update = engine.setCell('A1', '1');

        expect(update.plan.order).toEqual(['SHEET@A1']);
        expect(update.plan.cyclic).toEqual(['SHEET@B1', 'SHEET@C1']);
    });

    test('breaking a cycle restores normal recalculation', () => {
        const engine = new SheetEngine();
        engine.setCell('A1', '= B1');
        engine.setCell('B1', '= A1');
        const update = engine.setCell('B1', '5');

        expect(update.plan.cyclic).toEqual([]);
        expect(update.plan.order).toEqual(['SHEET@B1', 'SHEET@A1']);
    });
});

describe('fullRecalcPlan (initial load / restore)', () => {
    test('orders every cell with dependencies first', () => {
        const engine = new SheetEngine();
        engine.setCell('C1', '= B1');
        engine.setCell('B1', '= A1');
        engine.setCell('A1', '1');

        const plan = engine.fullRecalcPlan();
        expect(plan.order).toEqual(['SHEET@A1', 'SHEET@B1', 'SHEET@C1']);
    });

    test('independent cells come out in deterministic (sorted) order', () => {
        const engine = new SheetEngine();
        engine.setCell('B2', '2');
        engine.setCell('A1', '1');
        engine.setCell('C3', '3');

        expect(engine.fullRecalcPlan().order).toEqual(['SHEET@A1', 'SHEET@B2', 'SHEET@C3']);
    });
});

describe('multi-sheet boundaries (plan §3.1: 辞書修飾で越境参照)', () => {
    test('cross-sheet references are tracked but only local cells enter the plan', () => {
        const engine = new SheetEngine('SHEET2');
        engine.setCell('A1', '= SHEET@Z9 [ 1 ] +');

        const plan = engine.recalcPlan(['SHEET@Z9']);
        expect(plan.order).toEqual(['SHEET2@A1']);
    });

    test('bare refs qualify against the engine sheet name', () => {
        const engine = new SheetEngine('SHEET2');
        const update = engine.setCell('B1', '= A1');
        expect(update.define?.bodySource).toBe(' SHEET2@A1');
        expect(update.record?.references).toEqual(['SHEET2@A1']);
    });
});
