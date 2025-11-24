// js/gui/mobile.ts

interface MobileElements {
    inputArea: HTMLElement;
    outputArea: HTMLElement;
    stackArea: HTMLElement;
    dictionaryArea: HTMLElement;
}

export class MobileHandler {
    private elements!: MobileElements;
    private currentMode: 'input' | 'execution' = 'input';
    private touchStartX: number = 0;
    private touchStartY: number = 0;
    private touchEndX: number = 0;
    private touchEndY: number = 0;
    private readonly SWIPE_THRESHOLD = 50; // 最小スワイプ距離（px）

    init(elements: MobileElements): void {
        this.elements = elements;
        this.setupSwipeGestures();
    }

    isMobile(): boolean {
        return window.innerWidth <= 768;
    }

    getCurrentMode(): 'input' | 'execution' {
        return this.currentMode;
    }

    updateView(mode: 'input' | 'execution'): void {
        if (!this.isMobile()) {
            // デスクトップモードでは全て表示
            Object.values(this.elements).forEach(el => {
                if (el && el.style) el.style.display = '';
            });
            return;
        }

        // 現在のモードを保存
        this.currentMode = mode;

        // モバイルモード
        if (mode === 'input') {
            this.elements.inputArea.style.display = 'block';
            this.elements.outputArea.style.display = 'none';
            this.elements.stackArea.style.display = 'none';
            this.elements.dictionaryArea.style.display = 'block';
        } else { // 'execution' mode
            this.elements.inputArea.style.display = 'none';
            this.elements.outputArea.style.display = 'block';
            this.elements.stackArea.style.display = 'block';
            this.elements.dictionaryArea.style.display = 'none';
        }
    }

    private setupSwipeGestures(): void {
        if (!this.isMobile()) return;

        const container = document.body;

        container.addEventListener('touchstart', (e: TouchEvent) => {
            const touch = e.changedTouches[0];
            if (touch) {
                this.touchStartX = touch.screenX;
                this.touchStartY = touch.screenY;
            }
        }, { passive: true });

        container.addEventListener('touchend', (e: TouchEvent) => {
            const touch = e.changedTouches[0];
            if (touch) {
                this.touchEndX = touch.screenX;
                this.touchEndY = touch.screenY;
                this.handleSwipeGesture();
            }
        }, { passive: true });
    }

    private handleSwipeGesture(): void {
        const deltaX = this.touchEndX - this.touchStartX;
        const deltaY = this.touchEndY - this.touchStartY;

        // 横方向のスワイプが縦方向より大きい場合のみ処理
        if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > this.SWIPE_THRESHOLD) {
            // 左右どちらのスワイプでも、現在のモードと反対のモードに切り替え
            const newMode = this.currentMode === 'input' ? 'execution' : 'input';
            this.updateView(newMode);
        }
    }
}
