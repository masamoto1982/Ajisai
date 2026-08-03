import { describe, expect, test } from 'vitest';
import { readFileSync } from 'node:fs';

const readJson = (path: string): Record<string, unknown> =>
    JSON.parse(readFileSync(path, 'utf8')) as Record<string, unknown>;

describe('GUI Phase 0 freeze contract', () => {
    const freeze = readJson('spec/freeze/gui-production-files.json');
    const html = readFileSync('index.html', 'utf8');
    const layoutSource = readFileSync('src/gui/gui-layout-state.ts', 'utf8');

    test('pins the production roots and four observable surfaces', () => {
        expect(freeze.roots).toEqual(['index.html', 'src/gui', 'src/styles']);
        expect(freeze.surfaces).toEqual(['Input', 'Output', 'Stack', 'Dictionary']);
        expect(freeze.desktopColumns).toBe(2);
        expect(freeze.mobileVisibleSurfaces).toBe(1);
    });

    test('current DOM retains all four accessible panels', () => {
        for (const surface of freeze.surfaces as string[]) {
            expect(html).toContain(`aria-label="${surface}"`);
        }
        expect(html).toContain('id="dictionary-sheet-select"');
        expect(html).toContain('id="copy-output-btn"');
    });

    test('current keyboard labels remain byte-for-byte visible to users', () => {
        for (const shortcut of freeze.shortcuts as string[]) {
            expect(layoutSource).toContain(`'${shortcut}'`);
        }
    });

    test('the generated word inventory stays at 80 surfaces', () => {
        // The vocabulary is a ceiling, not a fixed inventory: shrinking is free,
        // growing is a deliberate specification change. Modules are gone, so
        // every Word is a flat Core Word.
        const manifest = readJson('docs/word-manifest.json');
        expect(manifest.counts).toEqual({
            canonicalWords: 57,
            semanticKernelWords: 35,
            standardWords: 22,
            corewords: 57,
            aliases: 15,
            surface_forms: 8,
            total: 80,
        });
    });
});
