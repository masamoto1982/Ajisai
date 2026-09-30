// Step mode's hold on the editor's selection: it selects the piece about to
// run, and collapses that selection when it ends. Run and Reset also end step
// mode, and used to collapse the selection whether or not step mode had drawn
// one — which moved the caret to the start of the text on every Run, so a Run
// that failed left its text in place but the caret at the top of it.

import { describe, expect, test } from 'vitest';
import { createStepExecutor } from './step-executor';

const setup = (source: string, completes: boolean = true) => {
    const highlights: Array<[number, number]> = [];
    const executed: string[] = [];
    const info: string[] = [];
    const executor = createStepExecutor({
        extractEditorValue: () => source,
        showInfo: (text) => { info.push(text); },
        highlightSourceRange: (start, end) => { highlights.push([start, end]); },
        executeSource: async (code) => { executed.push(code); return completes; }
    });
    return { executor, highlights, executed, info };
};

describe('createStepExecutor', () => {
    test('reset outside step mode leaves the selection alone', () => {
        const { executor, highlights } = setup('1 2 ADD');
        executor.reset();
        executor.abort();
        expect(highlights).toEqual([]);
        expect(executor.isActive()).toBe(false);
    });

    test('selects each piece before running it and collapses the selection at the end', async () => {
        const { executor, highlights, executed } = setup('1 [ 2 ] ADD');
        await executor.executeStep();
        expect(executor.isActive()).toBe(true);
        await executor.executeStep();
        await executor.executeStep();
        expect(executed).toEqual(['1', '[ 2 ]', 'ADD']);
        expect(highlights).toEqual([[0, 1], [2, 7], [8, 11], [0, 0]]);
        expect(executor.isActive()).toBe(false);
    });

    test('a Run during step mode ends it and collapses its selection', async () => {
        const { executor, highlights } = setup('1 2');
        await executor.executeStep();
        executor.reset();
        expect(highlights).toEqual([[0, 1], [0, 0]]);
        expect(executor.isActive()).toBe(false);
    });

    test('a step that fails ends step mode and collapses its selection', async () => {
        const { executor, highlights } = setup('1 DIV', false);
        await executor.executeStep();
        expect(highlights).toEqual([[0, 1], [0, 0]]);
        expect(executor.isActive()).toBe(false);
    });
});
