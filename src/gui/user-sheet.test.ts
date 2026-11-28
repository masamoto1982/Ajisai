// The Sheet view of the User dictionary, against the real interpreter: a
// cell's value is read apart from the session, and an edit is a DEF.

import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import * as wasm from '../wasm/generated/ajisai_core.js';
import type { AjisaiInterpreter } from '../wasm-interpreter-types';
import { describeTable, isSheetName, renameSheetWord, runSheetEdit, sheetDefinitionSource } from './user-sheet';

wasm.initSync({ module: readFileSync(new URL('../wasm/generated/ajisai_core_bg.wasm', import.meta.url)) });

const createInterpreter = (): AjisaiInterpreter => new wasm.AjisaiInterpreter() as unknown as AjisaiInterpreter;

it('writes a sheet edit as the DEF it is, with its #:contract line', () => {
    expect(sheetDefinitionSource('PRICE', ' [ 120 80 ] ', null)).toBe("[ [ 120 80 ] ] 'PRICE' DEF");
    expect(sheetDefinitionSource('TOTAL', 'PRICE 3 MUL', '-> [int]'))
        .toBe("#:contract TOTAL -> [int]\n[ PRICE 3 MUL ] 'TOTAL' DEF");
    expect(isSheetName('TOTAL')).toBe(true);
    expect(isSheetName("IT'S")).toBe(false);
    expect(isSheetName('TWO WORDS')).toBe(false);
});

it('reads a cell apart from the session, and an edited cell is read again', async () => {
    const interpreter = createInterpreter();
    await interpreter.execute('7');
    const stackBefore = interpreter.collect_stack();
    expect((await runSheetEdit(interpreter, sheetDefinitionSource('PRICE', '[ 120 80 300 ]', null))).status).toBe('OK');
    expect((await runSheetEdit(interpreter, sheetDefinitionSource('TOTAL', 'PRICE 3 MUL', null))).status).toBe('OK');

    const total = interpreter.describe_sheet_cell('TOTAL');
    expect(total).toMatchObject({ formula: 'PRICE 3 MUL', inputs: 0, outputs: 1, signature: null });
    expect(await interpreter.evaluate_sheet_cell('TOTAL'))
        .toEqual({ value: '[ 360/1 240/1 900/1 ]', suggestedType: '[int]' });

    // The cell TOTAL reads is edited in place, and TOTAL's identity follows.
    expect((await runSheetEdit(interpreter, sheetDefinitionSource('PRICE', '[ 100 ]', null))).status).toBe('OK');
    expect(interpreter.describe_sheet_cell('TOTAL')?.identity).not.toBe(total?.identity);
    expect((await interpreter.evaluate_sheet_cell('TOTAL')).value).toBe('[ 300/1 ]');

    // Accepting the suggested type declares it. A typed body may call an
    // untyped Word, whose body is walked with the types it is given, so
    // TOTAL is typed before PRICE is; one that does not fit is refused.
    expect((await runSheetEdit(interpreter, sheetDefinitionSource('TOTAL', 'PRICE 3 MUL', '-> [str]'))).errorInfo?.category)
        .toBe('contractViolation');
    expect((await runSheetEdit(interpreter, sheetDefinitionSource('TOTAL', 'PRICE 3 MUL', '-> [int]'))).status).toBe('OK');
    expect((await runSheetEdit(interpreter, sheetDefinitionSource('PRICE', '[ 100 ]', '-> [int]'))).status).toBe('OK');
    expect(interpreter.describe_sheet_cell('TOTAL')?.signature).toBe('-> [int]');

    expect(interpreter.collect_stack()).toEqual(stackBefore);
});

it('leaves the stack as it was when a sheet edit is refused', async () => {
    const interpreter = createInterpreter();
    await interpreter.execute("[ 1 ] 'A' DEF [ A ] 'B' DEF 5");
    const stackBefore = interpreter.collect_stack();
    const result = await runSheetEdit(interpreter, sheetDefinitionSource('A', '1 2', null));
    expect(result.errorInfo?.category).toBe('definitionConflict');
    expect(interpreter.collect_stack()).toEqual(stackBefore);
});

it('renames a cell as DEF then DEL, and leaves a called one as it was', async () => {
    const interpreter = createInterpreter();
    await interpreter.execute("#:contract A1 -> int\n[ 120 ] 'A1' DEF 5");
    const stackBefore = interpreter.collect_stack();
    const contract = interpreter.lookup_word_description('A1');
    expect((await renameSheetWord(interpreter, 'A1', 'PRICE', '120', contract)).status).toBe('OK');
    expect(interpreter.describe_sheet_cell('A1')).toBeNull();
    expect(interpreter.describe_sheet_cell('PRICE')).toMatchObject({ formula: '120', signature: '-> int' });

    await interpreter.execute("[ PRICE 3 MUL ] 'TOTAL' DEF");
    const refused = await renameSheetWord(interpreter, 'PRICE', 'COST', '120', '-> int');
    expect(refused.errorInfo?.category).toBe('definitionConflict');
    expect(interpreter.describe_sheet_cell('COST')).toBeNull();
    expect(interpreter.describe_sheet_cell('PRICE')).not.toBeNull();
    expect(interpreter.collect_stack()).toEqual(stackBefore);
});

it('lays a Record of columns out as a table and suggests its Record type', async () => {
    const interpreter = createInterpreter();
    await interpreter.execute("[ [ 'item' 'price' ] [ [ 'tea' 'cake' ] [ 120 80 ] ] RECORD ] 'MENU' DEF");
    const menu = await interpreter.evaluate_sheet_cell('MENU');
    expect(menu.suggestedType).toBe('{item:[str] price:[int]}');
    expect(menu.table).toEqual({ columns: ['item', 'price'], cells: [["'tea'", '120/1'], ["'cake'", '80/1']], rows: 2 });
    expect(describeTable(menu.table!)).toBe('item, price · 2 rows');
    expect((await runSheetEdit(interpreter, sheetDefinitionSource(
        'MENU', "[ 'item' 'price' ] [ [ 'tea' 'cake' ] [ 120 80 ] ] RECORD", '-> {item:[str] price:[int]}'
    ))).status).toBe('OK');
    expect(interpreter.describe_sheet_cell('MENU')?.signature).toBe('-> {item:[str] price:[int]}');
});
