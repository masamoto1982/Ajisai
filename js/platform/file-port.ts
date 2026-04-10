export interface PickTextFileResult {
    readonly path?: string;
    readonly name?: string;
    readonly text: string;
}

export interface FilePort {
    pickTextFile(options?: {
        accept?: string;
        title?: string;
    }): Promise<PickTextFileResult | null>;
    saveTextFile(options: {
        suggestedName: string;
        text: string;
        title?: string;
    }): Promise<boolean>;
}
