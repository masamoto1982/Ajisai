// js/gui/editor.ts (TypeScriptエラー修正版)

export class Editor {
    private element!: HTMLTextAreaElement;
    private autoLabelEnabled = true;
    private usedLabels = new Set<string>(); // 衝突回避用
    private labelRegistry = new Map<string, number>(); // ラベル → 行番号
    private reverseRegistry = new Map<number, string>(); // 行番号 → ラベル
    
    // Base62文字セット
    private readonly BASE62_CHARS = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';

    init(element: HTMLTextAreaElement): void {
        this.element = element;
        this.setupEventListeners();
        this.setupLinkHandling();
        
        if (this.element.value.trim() === '') {
            this.element.value = this.generateLabel() + ': ';
        }
        this.updateLabelRegistry();
    }

    private generateLabel(): string {
        let attempts = 0;
        const maxAttempts = 100; // 安全装置
        
        while (attempts < maxAttempts) {
            const label = this.generateRandomLabel();
            if (!this.usedLabels.has(label)) {
                this.usedLabels.add(label);
                return label;
            }
            attempts++;
        }
        
        // フォールバック（理論上到達不可能）
        console.warn('Label generation failed, using fallback');
        return this.generateFallbackLabel();
    }

    private generateRandomLabel(): string {
        // 高品質ランダム生成
        const now = Date.now();
        const random1 = Math.random() * 0xFFFFFF;
        const random2 = Math.random() * 0xFFFFFF;
        
        // 時刻 + 2つのランダム要素を組み合わせ
        const seed = (now * random1 + random2) % (62 * 62 * 62 * 62);
        
        return this.toBase62(Math.floor(seed), 4);
    }

    private generateFallbackLabel(): string {
        // 連続番号ベースのフォールバック
        let counter = this.usedLabels.size;
        while (this.usedLabels.has(this.toBase62(counter, 4))) {
            counter++;
        }
        const label = this.toBase62(counter, 4);
        this.usedLabels.add(label);
        return label;
    }

    private toBase62(num: number, minLength: number = 4): string {
        if (num === 0) return this.BASE62_CHARS[0].repeat(minLength);
        
        let result = '';
        while (num > 0) {
            result = this.BASE62_CHARS[num % 62] + result;
            num = Math.floor(num / 62);
        }
        
        return result.padStart(minLength, this.BASE62_CHARS[0]);
    }

    private setupEventListeners(): void {
        this.element.addEventListener('keydown', (e) => this.handleKeyDown(e));
        this.element.addEventListener('paste', (e) => this.handlePaste(e));
        this.element.addEventListener('input', () => this.updateLabelRegistry());
    }

    private setupLinkHandling(): void {
        this.element.addEventListener('click', (e) => this.handleLinkClick(e));
        this.element.addEventListener('mousemove', (e) => this.handleMouseMove(e));
        this.element.style.position = 'relative';
    }

    private updateLabelRegistry(): void {
    this.labelRegistry.clear();
    this.reverseRegistry.clear();
    this.usedLabels.clear(); // 既存ラベルを再収集
    
    const lines = this.element.value.split('\n');
    lines.forEach((line, index) => {
        // Base62ラベルパターン（英数字4文字 + コロン）
        const match = line.match(/^([0-9A-Za-z]{4}):\s/);
        if (match && match.length > 1) {
            const label = match[1]!; // 66行目修正: Non-null assertion
            this.labelRegistry.set(label, index);
            this.reverseRegistry.set(index, label);
            this.usedLabels.add(label); // 使用済みとして記録
        }
    });
}

    private insertNewLineWithLabel(): void {
        const cursorPos = this.element.selectionStart;
        const textBefore = this.element.value.substring(0, cursorPos);
        const textAfter = this.element.value.substring(cursorPos);
        
        const label = this.generateLabel();
        const newLine = `\n${label}: `;
        
        this.element.value = textBefore + newLine + textAfter;
        this.element.selectionStart = this.element.selectionEnd = cursorPos + newLine.length;
        this.updateLabelRegistry();
    }

    private handlePaste(event: ClipboardEvent): void {
        if (!this.autoLabelEnabled) return;
        
        event.preventDefault();
        const pastedText = event.clipboardData?.getData('text') || '';
        const processedText = this.processMultilineText(pastedText);
        this.insertTextAtCursor(processedText);
    }

    private processMultilineText(text: string): string {
        const lines = text.split('\n');
        return lines.map(line => {
            if (line.trim() === '') return '';
            
            // 既にBase62ラベルが付いているかチェック
            if (this.hasLabel(line)) {
                return line;
            } else {
                return `${this.generateLabel()}: ${line}`;
            }
        }).join('\n');
    }

    private hasLabel(line: string): boolean {
        // 4文字英数字 + コロン + スペースのパターン
        return /^[0-9A-Za-z]{4}:\s/.test(line.trim());
    }

    private insertTextAtCursor(text: string): void {
        const cursorPos = this.element.selectionStart;
        const textBefore = this.element.value.substring(0, cursorPos);
        const textAfter = this.element.value.substring(cursorPos);
        
        this.element.value = textBefore + text + textAfter;
        this.element.selectionStart = this.element.selectionEnd = cursorPos + text.length;
    }

    // 普通のクリックでジャンプ
    private handleLinkClick(event: MouseEvent): void {
        const clickInfo = this.getClickInfo(event);
        
        if (clickInfo && clickInfo.isLeapLabel && clickInfo.label) { // 162行目修正: clickInfo.labelのundefinedチェック追加
            this.jumpToLabel(clickInfo.label);
            event.preventDefault();
            event.stopPropagation();
        }
    }

    private getClickInfo(event: MouseEvent): { isLeapLabel: boolean; label?: string } | null {
        const rect = this.element.getBoundingClientRect();
        const x = event.clientX - rect.left + this.element.scrollLeft;
        const y = event.clientY - rect.top + this.element.scrollTop;
        
        // フォント情報から位置を計算
        const computedStyle = window.getComputedStyle(this.element);
        const lineHeight = parseFloat(computedStyle.lineHeight);
        const fontSize = parseFloat(computedStyle.fontSize);
        const charWidth = fontSize * 0.6; // モノスペースフォントの概算
        
        const lineIndex = Math.floor(y / lineHeight);
        const charIndex = Math.floor(x / charWidth);
        
        const lines = this.element.value.split('\n');
        if (lineIndex >= lines.length) return null;
        
        const line = lines[lineIndex]; // 188行目修正: line変数の定義
        if (!line) return null; // 188行目修正: undefinedチェック追加
        
        // LEAP文のラベル部分かチェック
        const leapMatch = line.match(/"([0-9A-Za-z]{4})"\s+LEAP/); // 190行目修正: lineのundefinedチェック済み
        if (leapMatch && leapMatch[1]) { // 191行目修正: leapMatch[1]のundefinedチェック追加
            const labelStart = line.indexOf(`"${leapMatch[1]}"`);
            const labelEnd = labelStart + leapMatch[1].length + 2; // クォート含む
            
            if (charIndex >= labelStart && charIndex <= labelEnd) {
                return { isLeapLabel: true, label: leapMatch[1] };
            }
        }
        
        return null;
    }

    // マウス移動でカーソル変更
    private handleMouseMove(event: MouseEvent): void {
        const clickInfo = this.getClickInfo(event);
        
        if (clickInfo && clickInfo.isLeapLabel && clickInfo.label) { // clickInfo.labelのundefinedチェック追加
            this.element.style.cursor = 'pointer';
            this.element.title = `${clickInfo.label} へジャンプ`;
        } else {
            this.element.style.cursor = 'text';
            this.element.title = '';
        }
    }

    private jumpToLabel(label: string): void {
        const targetLine = this.labelRegistry.get(label);
        if (targetLine !== undefined) {
            this.jumpToLine(targetLine);
            this.highlightLine(targetLine);
        } else {
            this.showLabelNotFoundEffect();
        }
    }

    private jumpToLine(lineNumber: number): void {
        const lines = this.element.value.split('\n');
        if (lineNumber < lines.length) {
            const lineStart = lines.slice(0, lineNumber).join('\n').length + (lineNumber > 0 ? 1 : 0);
            this.element.focus();
            this.element.setSelectionRange(lineStart, lineStart);
            this.scrollToLine(lineNumber);
        }
    }

    private scrollToLine(lineNumber: number): void {
        const computedStyle = window.getComputedStyle(this.element);
        const lineHeight = parseFloat(computedStyle.lineHeight);
        const targetY = lineNumber * lineHeight;
        
        this.element.scrollTo({
            top: targetY - this.element.clientHeight / 2,
            behavior: 'smooth'
        });
    }

    private highlightLine(lineNumber: number): void {
        const lines = this.element.value.split('\n');
        const lineStart = lines.slice(0, lineNumber).join('\n').length + (lineNumber > 0 ? 1 : 0);
        const targetLine = lines[lineNumber]; // 248行目修正: targetLine変数の定義
        if (!targetLine) return; // 248行目修正: undefinedチェック追加
        const lineEnd = lineStart + targetLine.length;
        
        setTimeout(() => {
            this.element.setSelectionRange(lineStart, lineEnd);
        }, 100);
        
        setTimeout(() => {
            this.element.setSelectionRange(lineStart, lineStart);
        }, 1000);
    }

    private showLabelNotFoundEffect(): void {
        const originalBg = this.element.style.backgroundColor;
        this.element.style.backgroundColor = '#ffebee';
        setTimeout(() => {
            this.element.style.backgroundColor = originalBg;
        }, 300);
    }

    // 外部インターフェース
    getValue(): string {
        return this.element.value.trim();
    }

    // デバッグ用：ラベル統計
    getLabelStats(): { 
        totalUsed: number; 
        totalPossible: number; 
        usageRate: string;
        samples: string[] 
    } {
        const totalPossible = 62 * 62 * 62 * 62; // Base62の4文字
        const samples = Array.from(this.usedLabels).slice(0, 10); // 最初の10個
        
        return {
            totalUsed: this.usedLabels.size,
            totalPossible,
            usageRate: ((this.usedLabels.size / totalPossible) * 100).toFixed(6) + '%',
            samples
        };
    }

    // 既存メソッド
    handleKeyDown(event: KeyboardEvent): void {
        if (event.key === 'Enter' && this.autoLabelEnabled) {
            event.preventDefault();
            this.insertNewLineWithLabel();
        }
    }

    clear(): void {
        this.usedLabels.clear();
        this.labelRegistry.clear();
        this.reverseRegistry.clear();
        const initialContent = `${this.generateLabel()}: `;
        this.element.value = initialContent;
        this.element.focus();
        this.updateLabelRegistry();
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
