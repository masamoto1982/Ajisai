// js/gui/main.ts（該当部分のみ）

    private async runNormal(): Promise<void> {
        const code = this.editor.getValue();
        if (!code) return;

        this.stepMode = false;
        this.updateRunButton();

        try {
            const result = window.ajisaiInterpreter.execute(code) as ExecuteResult;
            if (result.status === 'OK') {
                this.display.showOutput(result.output || 'OK');
                
                if (result.autoNamed && result.autoNamedWord) {
                    this.editor.setValue(result.autoNamedWord);
                } else if (!result.autoNamed) {
                    this.editor.clear();
                }
                
                if (this.mobile.isMobile()) {
                    this.setMode('execution');
                }
            } else {
                // ExecuteResultをエラーメッセージに変換
                this.display.showError(result.message || result.status || 'Unknown error');
            }
        } catch (error) {
            this.display.showError(error as Error);
        }
        
        this.updateAllDisplays();
        await this.persistence.saveCurrentState();
        this.display.showInfo('State saved.', true);
    }
