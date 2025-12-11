// js/gui/editor.ts (ラベル機能完全削除版 + モバイル対応 + ハイライト機能)

export class Editor {
    private element!: HTMLTextAreaElement;
    private gui: any; // GUIインスタンスへの参照
    private onContentChangeCallback?: (content: string) => void;

    init(element: HTMLTextAreaElement, gui?: any): void {
        this.element = element;
        this.gui = gui;
        this.setupEventListeners();

        if (this.element.value.trim() === '') {
            this.element.value = '';
        }
    }

    private setupEventListeners(): void {
        // フォーカス時は入力モードに切り替え（モバイル用）
        this.element.addEventListener('focus', () => {
            if (this.gui && this.gui.mobile) {
                this.gui.mobile.updateView('input');
            }
        });

        // 入力内容の変更を監視
        this.element.addEventListener('input', () => {
            if (this.onContentChangeCallback) {
                this.onContentChangeCallback(this.element.value);
            }
        });
    }

    // コンテンツ変更時のコールバックを設定
    setOnContentChange(callback: (content: string) => void): void {
        this.onContentChangeCallback = callback;
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

    // 入力支援テキストを挿入（カーソルを最も内側の [ ] の間に配置）
    insertText(text: string): void {
        const start = this.element.selectionStart;
        const end = this.element.selectionEnd;
        const currentText = this.element.value;

        this.element.value = currentText.substring(0, start) + text + currentText.substring(end);

        // カーソルを最も内側の [ ] の間に配置
        // "[ [ [ ] ] ]" の場合、中央の空白位置
        const innerBracketPos = text.lastIndexOf('[ ]');
        if (innerBracketPos !== -1) {
            const cursorPos = start + innerBracketPos + 2; // "[ " の後
            this.element.selectionStart = cursorPos;
            this.element.selectionEnd = cursorPos;
        } else {
            const newPos = start + text.length;
            this.element.selectionStart = newPos;
            this.element.selectionEnd = newPos;
        }

        this.element.focus();
        // 入力支援時は入力モードに切り替え（モバイル用）
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
