private async runCode(): Promise<void> {
    const code = this.editor.getValue();
    if (!code) return;

    try {
        console.log('[GUI] Executing code via workers');
        this.display.showInfo('Executing...', true);
        
        let result: ExecuteResult;
        
        if (this.workerInitialized) {
            // 🆕 プログレスコールバックを設定
            result = await WORKER_MANAGER.execute(code, (progressResult) => {
                console.log('[GUI] Progress callback:', progressResult);
                // 各ステップの結果を即座に表示
                if (progressResult.output) {
                    this.display.showExecutionResult(progressResult);
                }
                // スタック表示を更新
                this.updateAllDisplays();
            });
        } else {
            // Fallback to main thread
            result = await window.ajisaiInterpreter.execute(code) as ExecuteResult;
        }

        if (result.definition_to_load) {
            this.editor.setValue(result.definition_to_load);
            const wordName = code.replace("?", "").trim();
            this.display.showInfo(`Loaded definition for ${wordName}.`);
        } else if (result.status === 'OK' && !result.error) {
            this.display.showExecutionResult(result);
            this.editor.clear();

            if (this.mobile.isMobile()) {
                this.setMode('execution');
            }
        } else if (result.status === 'COMPLETED') {
            // Progressive execution completed
            this.display.showInfo('Progressive execution completed.', true);
            this.editor.clear();

            if (this.mobile.isMobile()) {
                this.setMode('execution');
            }
        } else {
            this.display.showError(result.message || 'Unknown error');
        }
    } catch (error) {
        console.error('[GUI] Code execution failed:', error);
        
        if (error instanceof Error && error.message.includes('aborted')) {
            this.display.showInfo('Execution aborted by user.', true);
        } else {
            this.display.showError(error as Error);
        }
    }

    this.updateAllDisplays();
    await this.persistence.saveCurrentState();

    if (!code.trim().endsWith("?")) {
        this.display.showInfo('State saved.', true);
    }
}
