export const switchDictionarySheet = (containerEl: HTMLElement, sheetId: string): void => {
    const allSheets = containerEl.querySelectorAll('.dictionary-sheet');
    allSheets.forEach(sheet => {
        (sheet as HTMLElement).hidden = true;
        sheet.classList.remove('active');
    });

    const target = document.getElementById(`dictionary-sheet-${sheetId}`);
    if (target) {
        target.hidden = false;
        target.classList.add('active');
    }

    const userOnlyIds = ['user-dictionary-select', 'export-btn', 'import-btn'];
    const showUserControls = sheetId === 'user';
    for (const id of userOnlyIds) {
        const el = document.getElementById(id);
        if (el) (el as HTMLElement).hidden = !showUserControls;
    }
};
