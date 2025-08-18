// js/gui/editor.ts

export class Editor {
    private element!: HTMLTextAreaElement;
    private autoLabelEnabled = true;
    private labelsVisible = true;
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
            if (match) {
                const label = match[1];
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
        const newLine = this.labelsVisible ? `\n${label}: ` : '\n';
        
        this.element.value = textBefore + newLine + textAfter;
        this.element.selectionStart = this.element.selectionEnd = cursorPos + newLine.length;
        this.updateLabelRegistry();
    }

    // 普通のクリックでジャンプ
    private handleLinkClick(event: MouseEvent): void {
        const clickInfo = this.getClickInfo(event);
        
        if (clickInfo && clickInfo.isLeapLabel) {
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
        
        const line = lines[lineIndex];
        
        // LEAP文のラベル部分かチェック
        const leapMatch = line.match(/"([0-9A-Za-z]{4})"\s+LEAP/);
        if (leapMatch) {
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
        
        if (clickInfo && clickInfo.isLeapLabel) {
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
        const lineEnd = lineStart + lines[lineNumber].length;
        
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

    // ラベル表示切り替え
    toggleLabelVisibility(): void {
        this.labelsVisible = !this.labelsVisible;
        this.refreshDisplay();
    }

    private refreshDisplay(): void {
        const cursorPos = this.element.selectionStart;
        const lines = this.element.value.split('\n');
        
        const processedLines = lines.map(line => {
            const match = line.match(/^([0-9A-Za-z]{4}):\s(.*)$/);
            if (match) {
                const [, label, code] = match;
                return this.labelsVisible ? line : code;
            }
            return line;
        });
        
        this.element.value = processedLines.join('\n');
        this.element.selectionStart = this.element.selectionEnd = cursorPos;
    }

    // 外部インターフェース
    getValue(): string {
        if (!this.labelsVisible) {
            return this.getFullLabeledValue();
        }
        return this.element.value.trim();
    }

    private getFullLabeledValue(): string {
        const lines = this.element.value.split('\n');
        return lines.map((line, index) => {
            const label = this.reverseRegistry.get(index);
            if (label && !line.includes(':')) {
                return `${label}: ${line}`;
            }
            return line;
        }).join('\n');
    }

    isLabelsVisible(): boolean {
        return this.labelsVisible;
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
        const initialContent = this.labelsVisible ? `${this.generateLabel()}: ` : '';
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
