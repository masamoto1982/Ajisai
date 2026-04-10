import type { FilePort, PickTextFileResult } from '../file-port';

const readFileAsText = (file: File): Promise<string> =>
    new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (event) => {
            const result = event.target?.result;
            if (typeof result === 'string') {
                resolve(result);
            } else {
                reject(new Error('Failed to read file'));
            }
        };
        reader.onerror = () => reject(new Error('Failed to read file'));
        reader.readAsText(file);
    });

const pickFile = (accept: string): Promise<File | null> =>
    new Promise((resolve) => {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = accept;
        input.onchange = (e) => resolve((e.target as HTMLInputElement).files?.[0] ?? null);
        input.click();
    });

export const WEB_FILE_PORT: FilePort = {
    async pickTextFile(options): Promise<PickTextFileResult | null> {
        const file = await pickFile(options?.accept ?? '.json,text/plain,application/json');
        if (!file) return null;
        const text = await readFileAsText(file);
        return { name: file.name, text };
    },

    async saveTextFile(options): Promise<boolean> {
        const blob = new Blob([options.text], { type: 'application/json' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = options.suggestedName;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        return true;
    }
};
