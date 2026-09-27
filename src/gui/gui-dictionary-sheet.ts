export const switchDictionarySheet = (containerEl: HTMLElement, sheetId: string): void => {
    containerEl.querySelectorAll<HTMLElement>('.dictionary-sheet').forEach(sheet => {
        sheet.hidden = sheet.id !== `dictionary-sheet-${sheetId}`;
    });
};
