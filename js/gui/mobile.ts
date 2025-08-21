// js/gui/mobile.ts

interface MobileElements {
    inputArea: HTMLElement;
    outputArea: HTMLElement;
    workspaceArea: HTMLElement;  // stackArea → workspaceArea
    dictionaryArea: HTMLElement;
}

export class MobileHandler {
    private elements!: MobileElements;

    init(elements: MobileElements): void {
        this.elements = elements;
    }

    isMobile(): boolean {
        return window.innerWidth <= 768;
    }

    updateView(mode: 'input' | 'execution'): void {
        if (!this.isMobile()) {
            // デスクトップモードでは全て表示
            Object.values(this.elements).forEach(el => {
                if (el && el.style) el.style.display = '';
            });
            return;
        }
        
        // モバイルモード
        if (mode === 'input') {
            this.elements.inputArea.style.display = 'block';
            this.elements.outputArea.style.display = 'none';
            this.elements.workspaceArea.style.display = 'none';  // stackArea → workspaceArea
            this.elements.dictionaryArea.style.display = 'block';
        } else { // 'execution' mode
            this.elements.inputArea.style.display = 'none';
            this.elements.outputArea.style.display = 'block';
            this.elements.workspaceArea.style.display = 'block';  // stackArea → workspaceArea
            this.elements.dictionaryArea.style.display = 'none';
        }
    }
}
