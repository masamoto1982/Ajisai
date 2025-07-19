export class MobileHandler {
    constructor() {
        this.elements = {};
    }

    init(elements) {
        this.elements = elements;
        this.updateView('input');
    }

    isMobile() {
        return window.innerWidth <= 768;
    }

    updateView(mode) {
        if (!this.isMobile()) {
            // デスクトップモードでは全て表示
            Object.values(this.elements).forEach(el => {
                if (el) el.style.display = '';
            });
            return;
        }
        
        // モバイルモード
        if (mode === 'input') {
            this.elements.inputArea.style.display = 'block';
            this.elements.outputArea.style.display = 'none';
            this.elements.memoryArea.style.display = 'none';
            this.elements.dictionaryArea.style.display = 'block';
        } else {
            this.elements.inputArea.style.display = 'none';
            this.elements.outputArea.style.display = 'block';
            this.elements.memoryArea.style.display = 'block';
            this.elements.dictionaryArea.style.display = 'none';
        }
    }
}
