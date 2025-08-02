// js/gui/editor.ts

export class Editor {
    private element!: HTMLTextAreaElement;

    init(element: HTMLTextAreaElement): void {
        this.element = element;
    }

    getValue(): string {
        return this.element.value.trim();
    }

    setValue(value: string): void {
        this.element.value = value;
    }

    clear(): void {
        this.element.value = '';
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
