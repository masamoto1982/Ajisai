// js/gui/editor.ts

export class Editor {
    private element!: HTMLTextAreaElement;
    private autoLabelEnabled = true;

    init(element: HTMLTextAreaElement): void {
        this.element = element;
        this.setupEventListeners();
        
        // 初期行にもラベルを付与
        if (this.element.value.trim() === '') {
            this.element.value = this.generateTimestampLabel() + ': ';
        }
    }

    private setupEventListeners(): void {
        this.element.addEventListener('keydown', (e) => this.handleKeyDown(e));
        this.element.addEventListener('paste', (e) => this.handlePaste(e));
    }

    private handleKeyDown(event: KeyboardEvent): void {
        if (event.key === 'Enter' && this.autoLabelEnabled) {
            event.preventDefault();
            this.insertNewLineWithLabel();
        }
    }

    private handlePaste(event: ClipboardEvent): void {
        if (!this.autoLabelEnabled) return;
        
        event.preventDefault();
        const pastedText = event.clipboardData?.getData('text') || '';
        const processedText = this.processMultilineText(pastedText);
        this.insertTextAtCursor(processedText);
    }

    private generateTimestampLabel(): string {
        const now = new Date();
        const year = now.getFullYear();
        const month = (now.getMonth() + 1).toString().padStart(2, '0');
        const day = now.getDate().toString().padStart(2, '0');
        const hour = now.getHours().toString().padStart(2, '0');
        const minute = now.getMinutes().toString().padStart(2, '0');
        const second = now.getSeconds().toString().padStart(2, '0');
        
        return `${year}${month}${day}${hour}${minute}${second}`;
    }

    private insertNewLineWithLabel(): void {
        const cursorPos = this.element.selectionStart;
        const textBefore = this.element.value.substring(0, cursorPos);
        const textAfter = this.element.value.substring(cursorPos);
        
        const timestamp = this.generateTimestampLabel();
        const newLine = `\n${timestamp}: `;
        
        this.element.value = textBefore + newLine + textAfter;
        this.element.selectionStart = this.element.selectionEnd = cursorPos + newLine.length;
    }

    private processMultilineText(text: string): string {
        const lines = text.split('\n');
        return lines.map(line => {
            if (line.trim() === '') return '';
            
            // 既にタイムスタンプが付いているかチェック
            if (this.hasTimestamp(line)) {
                return line;
            } else {
                return `${this.generateTimestampLabel()}: ${line}`;
            }
        }).join('\n');
    }

    private hasTimestamp(line: string): boolean {
        // 12桁数字 + コロン + スペースのパターン
        return /^\d{12}:\s/.test(line.trim());
    }

    private insertTextAtCursor(text: string): void {
        const cursorPos = this.element.selectionStart;
        const textBefore = this.element.value.substring(0, cursorPos);
        const textAfter = this.element.value.substring(cursorPos);
        
        this.element.value = textBefore + text + textAfter;
        this.element.selectionStart = this.element.selectionEnd = cursorPos + text.length;
    }

    getValue(): string {
        return this.element.value.trim();
    }

    setValue(value: string): void {
        if (this.autoLabelEnabled && value && !this.hasTimestamp(value)) {
            // 設定値にタイムスタンプがない場合は追加
            const lines = value.split('\n');
            const processedLines = lines.map(line => {
                if (line.trim() === '') return '';
                return this.hasTimestamp(line) ? line : `${this.generateTimestampLabel()}: ${line}`;
            });
            this.element.value = processedLines.join('\n');
        } else {
            this.element.value = value;
        }
    }

    clear(): void {
        this.element.value = this.autoLabelEnabled ? `${this.generateTimestampLabel()}: ` : '';
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

    // ラベル機能の切り替え（将来的なオプション）
    toggleAutoLabel(): void {
        this.autoLabelEnabled = !this.autoLabelEnabled;
    }

    // 検索支援機能
    highlightTimestamp(timestamp: string): void {
        const text = this.element.value;
        const index = text.indexOf(`${timestamp}:`);
        if (index !== -1) {
            this.element.focus();
            this.element.setSelectionRange(index, index + timestamp.length + 1);
            this.element.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
    }

    // 相対ジャンプ支援：現在位置から相対的な行のタイムスタンプを取得
    getRelativeTimestamp(offset: number): string | null {
        const cursorPos = this.element.selectionStart;
        const textBefore = this.element.value.substring(0, cursorPos);
        const lines = textBefore.split('\n');
        const currentLine = lines.length - 1;
        
        const allLines = this.element.value.split('\n');
        const targetLine = currentLine + offset;
        
        if (targetLine >= 0 && targetLine < allLines.length) {
            const match = allLines[targetLine].match(/^(\d{12}):/);
            return match ? match[1] : null;
        }
        
        return null;
    }
}
