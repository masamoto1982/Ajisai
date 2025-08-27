// js/gui/mobile.ts

interface MobileElements {
    inputArea: HTMLElement;
    outputArea: HTMLElement;
    bookshelfArea: HTMLElement;  // workspaceArea → bookshelfArea
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
            this.elements.bookshelfArea.style.display = 'none';  // workspaceArea → bookshelfArea
            this.elements.dictionaryArea.style.display = 'block';
        } else { // 'execution' mode
            this.elements.inputArea.style.display = 'none';
            this.elements.outputArea.style.display = 'block';
            this.elements.bookshelfArea.style.display = 'block';  // workspaceArea → bookshelfArea
            this.elements.dictionaryArea.style.display = 'none';
        }
    }
}
