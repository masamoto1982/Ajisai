import { describe, expect, test } from 'vitest';
import { readFileSync } from 'node:fs';

const readJson = (path: string): Record<string, unknown> =>
    JSON.parse(readFileSync(path, 'utf8')) as Record<string, unknown>;

describe('HostProtocolV1 freeze contract', () => {
    const schema = readJson('spec/host-protocol-v1.schema.json');
    const typeSource = readFileSync('src/wasm-interpreter-types.ts', 'utf8');

    test('declares the additive-only V1 compatibility policy', () => {
        expect(schema['x-ajisai-version']).toBe(1);
        expect(schema['x-ajisai-compatibility']).toEqual({
            allowed: ['add optional fields'],
            forbidden: [
                'remove fields',
                'rename fields',
                'change field meaning',
                'reorder tuple elements',
                'change tuple shape',
            ],
            breakingChangesRequire: 'HostProtocolV2',
        });
    });

    test('lists every method exposed by the TypeScript host boundary', () => {
        const methods = schema['x-ajisai-methods'] as string[];
        for (const method of methods) {
            expect(typeSource, `${method} is absent from AjisaiInterpreter`).toMatch(
                new RegExp(`\\b${method}\\??\\s*\\(`),
            );
        }
        expect(methods).toContain('execute');
        expect(methods).toContain('execute_step');
        expect(methods).toContain('snapshot_stack');
        expect(methods).toContain('restore_stack_snapshot');
        expect(methods).toContain('collect_import_state');
        expect(methods).toContain('lookup_word_definition');
    });

    test('pins tuple lengths and their element order', () => {
        const defs = schema.$defs as Record<string, Record<string, unknown>>;
        for (const name of ['importStateEntry', 'userWordsInfo', 'coreWordsInfo', 'moduleCatalog']) {
            expect(JSON.stringify(defs[name])).toContain('prefixItems');
            expect(JSON.stringify(defs[name])).toContain('"items":false');
        }
    });

    test('golden payload retains NIL diagnosis, stack, dictionary, and import observations', () => {
        const golden = readJson('spec/freeze/host-protocol-v1.golden.json');
        const stack = golden.stack as Array<Record<string, unknown>>;
        const semantics = stack[0]!.semantics as Record<string, unknown>;
        const absence = semantics.absence as Record<string, unknown>;

        expect(golden.status).toBe('OK');
        expect(absence).toMatchObject({
            reason: 'divisionByZero',
            origin: 'divisionByZero',
            recoverability: 'recoverable',
        });
        expect(absence.diagnosis).toBeDefined();
        expect(golden.userWords).toEqual([
            { dictionary: 'USER', name: 'DOUBLE', definition: '{ 2 MUL }' },
        ]);
        expect(golden.importedModules).toEqual(['math']);
    });
});

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

    test('the generated word inventory stays at 93 surfaces', () => {
        // The vocabulary is a ceiling, not a fixed inventory: shrinking is free,
        // growing is a deliberate specification change. Modules are gone, so
        // every Word is a flat Core Word.
        const manifest = readJson('docs/word-manifest.json');
        expect(manifest.counts).toEqual({
            corewords: 69,
            aliases: 16,
            surface_forms: 8,
            total: 93,
        });
    });
});
