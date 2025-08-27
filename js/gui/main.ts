// js/gui/main.ts の setupEventListeners メソッドに追加
private setupEventListeners(): void {
    console.log('Setting up event listeners...');
    
    this.elements.runBtn.addEventListener('click', () => this.runCode());
    this.elements.clearBtn.addEventListener('click', () => this.editor.clear());
    
    if (this.elements.testBtn) {
        console.log('Adding test button event listener');
        this.elements.testBtn.addEventListener('click', () => {
            console.log('Test button clicked!');
            this.runTests();
        });
    } else {
        console.error('Cannot add event listener: test button not found');
    }

    this.elements.codeInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            if (e.shiftKey) {
                e.preventDefault();
                this.runCode();
            } else if (e.ctrlKey && e.altKey) {
                e.preventDefault();
                this.executeAmnesia();
            } else if (e.ctrlKey) {
                e.preventDefault();
                if (this.stepMode) {
                    this.executeNextStep();
                } else {
                    this.startStepExecution();
                }
            }
        } else if (e.key === 'Escape' && this.stepMode) {
            e.preventDefault();
            this.endStepExecution();
        }
    });

    // 新規追加：言語モード切り替えショートカット
    document.addEventListener('keydown', (e) => {
        if (e.ctrlKey && e.key === 'j') {
            e.preventDefault();
            this.setLanguageMode('japanese');
        } else if (e.ctrlKey && e.key === 'e') {
            e.preventDefault();
            this.setLanguageMode('english');
        }
    });

    this.elements.bookshelfArea.addEventListener('click', () => {
        if (this.mobile.isMobile() && this.mode === 'execution') {
            this.setMode('input');
        }
    });

    window.addEventListener('resize', () => this.mobile.updateView(this.mode));
}

// 新規メソッド追加
private setLanguageMode(mode: 'japanese' | 'english'): void {
    this.librarians.setLanguageMode(mode);
    
    // モード変更通知
    const modeText = mode === 'japanese' ? '日本語' : 'English';
    this.display.showInfo(`Language mode: ${modeText}`, true);
}
