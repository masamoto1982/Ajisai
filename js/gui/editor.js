export class Editor {
    constructor() {
        this.element = null;
    }

    init(element) {
        this.element = element;
    }

    getValue() {
        return this.element.value.trim();
    }

    setValue(value) {
        this.element.value = value;
    }

    clear() {
        this.element.value = '';
    }

    insertWord(word) {
        const start = this.element.selectionStart;
        const end = this.element.selectionEnd;
        const text = this.element.value;
        
        // カーソル位置に挿入
        this.element.value = text.substring(0, start) + word + text.substring(end);
        
        // カーソル位置を更新
        const newPos = start + word.length;
        this.element.selectionStart = newPos;
        this.element.selectionEnd = newPos;
        
        // フォーカスを維持
        this.element.focus();
    }

    focus() {
        this.element.focus();
    }
}
