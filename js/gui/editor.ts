// js/gui/editor.ts (ラベル機能完全削除版 + モバイル対応)

export class Editor {
    private element!: HTMLTextAreaElement;
    private gui: any; // GUIインスタンスへの参照

    init(element: HTMLTextAreaElement, gui?: any): void {
        this.element = element;
        this.gui = gui;
        this.setupEventListeners();
        
        if (this.element.value.trim() === '') {
            this.element.value = '';
        }
    }

    private setupEventListeners(): void {
        this.element.addEventListener('keydown', (e) => this.handleKeyDown(e));
        
        // フォーカス時は入力モードに切り替え（モバイル用）
        this.element.addEventListener('focus', () => {
            if (this.gui && this.gui.mobile) {
                this.gui.mobile.updateView('input');
            }
        });
    }

    handleKeyDown(_event: KeyboardEvent): void {
        // 特別な処理なし - 普通のテキストエディタとして動作
        // パラメータに _ を付けて未使用であることを明示
    }

    getValue(): string {
        return this.element.value.trim();
    }

    setValue(value: string): void {
        this.element.value = value;
        // 値をセットしたら入力モードに切り替え（モバイル用）
        if (this.gui && this.gui.mobile) {
            this.gui.mobile.updateView('input');
        }
    }

    clear(switchView: boolean = true): void {
        this.element.value = '';
        this.element.focus();
        // クリア後は入力モードに戻す（モバイル用）
        if (switchView && this.gui && this.gui.mobile) {
            this.gui.mobile.updateView('input');
        }
    }

    insertWord(word: string): void {
        const start = this.element.selectionStart;
        const end = this.element.selectionEnd;
        const text = this.element.value;
        
        this.element.value = text.substring(0, start) + word + text.substring(end);
        
        const newPos = start + word.length;
        this.element.selectionStart = newPos;
        this.element.selectionEnd = newPos;
        
        this.element.focus();
        // ワード挿入時は入力モードに切り替え（モバイル用）
        if (this.gui && this.gui.mobile) {
            this.gui.mobile.updateView('input');
        }
    }

    focus(): void {
        this.element.focus();
        // フォーカス時は入力モードに切り替え（モバイル用）
        if (this.gui && this.gui.mobile) {
            this.gui.mobile.updateView('input');
        }
    }
}
