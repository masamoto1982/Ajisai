// js/gui/mobile.ts

interface MobileElements {
    inputArea: HTMLElement;
    outputArea: HTMLElement;
    stackArea: HTMLElement;
    dictionaryArea: HTMLElement;
}

export class MobileHandler {
    updateView(mode: 'input' | 'execution'): void {
        if (!this.isMobile()) {
            Object.values(this.elements).forEach(el => {
                if (el && el.style) el.style.display = '';
            });
            return;
        }
        
        if (mode === 'input') {
            this.elements.inputArea.style.display = 'block';
            this.elements.outputArea.style.display = 'none';
            this.elements.stackArea.style.display = 'none';      // memoryArea → stackArea
            this.elements.dictionaryArea.style.display = 'block';
        } else {
            this.elements.inputArea.style.display = 'none';
            this.elements.outputArea.style.display = 'block';
            this.elements.stackArea.style.display = 'block';     // memoryArea → stackArea
            this.elements.dictionaryArea.style.display = 'none';
        }
    }
}
