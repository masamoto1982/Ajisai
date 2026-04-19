const dynamicImport = (specifier: string): Promise<any> =>
    (0, eval)(`import(${JSON.stringify(specifier)})`);

import type { FileIO, OpenResult, SaveResult } from '../platform-adapter';

export class TauriFileIO implements FileIO {
    async saveJson(defaultName: string, data: unknown): Promise<SaveResult> {
        const [{ save }, { writeTextFile }] = await Promise.all([
            dynamicImport('@tauri-apps/plugin-dialog'),
            dynamicImport('@tauri-apps/plugin-fs')
        ]);

        const path = await save({
            defaultPath: defaultName,
            filters: [{ name: 'JSON', extensions: ['json'] }]
        });

        if (!path) {
            throw new Error('Save cancelled');
        }

        await writeTextFile(path, JSON.stringify(data, null, 2));
        return { filename: defaultName };
    }

    async openJsonFile(): Promise<OpenResult | null> {
        const [{ open }, { readTextFile }] = await Promise.all([
            dynamicImport('@tauri-apps/plugin-dialog'),
            dynamicImport('@tauri-apps/plugin-fs')
        ]);

        const selected = await open({
            multiple: false,
            filters: [{ name: 'JSON', extensions: ['json'] }]
        });

        if (!selected || Array.isArray(selected)) {
            return null;
        }

        const text = await readTextFile(selected);
        const filename = selected.split(/[\\/]/).pop() ?? 'import.json';
        return { filename, text };
    }
}
