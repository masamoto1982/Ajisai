import { open, save } from '@tauri-apps/plugin-dialog';
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
import type { FilePort, PickTextFileResult } from '../file-port';

const resolveNameFromPath = (path: string): string => path.split(/[\\/]/).pop() ?? path;

export const TAURI_FILE_PORT: FilePort = {
    async pickTextFile(options): Promise<PickTextFileResult | null> {
        const selected = await open({
            title: options?.title ?? 'Select file',
            multiple: false,
            filters: [{ name: 'JSON', extensions: ['json'] }]
        });

        if (!selected || Array.isArray(selected)) {
            return null;
        }

        const text = await readTextFile(selected);
        return {
            path: selected,
            name: resolveNameFromPath(selected),
            text
        };
    },

    async saveTextFile(options): Promise<boolean> {
        const selected = await save({
            title: options.title ?? 'Save file',
            defaultPath: options.suggestedName,
            filters: [{ name: 'JSON', extensions: ['json'] }]
        });

        if (!selected) {
            return false;
        }

        await writeTextFile(selected, options.text);
        return true;
    }
};
