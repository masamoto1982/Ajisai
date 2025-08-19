// js/gui/editor.ts (ラベル機能完全削除版)

export class Editor {
    private element!: HTMLTextAreaElement;

    init(element: HTMLTextAreaElement): void {
        this.element = element;
        this.setupEventListeners();
        
        if (this.element.value.trim() === '') {
            this.element.value = '';
        }
    }

    private setupEventListeners(): void {
        this.element.addEventListener('keydown', (e) => this.handleKeyDown(e));
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
    }

    clear(): void {
        this.element.value = '';
        this.element.focus();
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
    }

    focus(): void {
        this.element.focus();
    }
}
