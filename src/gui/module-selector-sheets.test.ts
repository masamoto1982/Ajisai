import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, test } from 'vitest';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../..');

const readRepoFile = (path: string): string =>
    readFileSync(resolve(repoRoot, path), 'utf8');

describe('module selector sheet simplicity', () => {
    test('does not render the removed module unimport hint in DOM source or CSS', () => {
        const moduleSelectorSource = readRepoFile('src/gui/module-selector-sheets.ts');
        const componentStyles = readRepoFile('src/styles/components.css');
        const removedHintClass = ['module', 'unimport', 'hint'].join('-');
        const removedHintCopy = ['Right-click module words', 'to Unimport'].join(' ');

        expect(moduleSelectorSource).not.toContain(removedHintClass);
        expect(moduleSelectorSource).not.toContain(removedHintCopy);
        expect(componentStyles).not.toContain(removedHintClass);
    });
});
