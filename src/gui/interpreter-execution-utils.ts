export const resolveExecutionException = (
    context: string,
    error: unknown,
    showInfo: (text: string, append: boolean) => void,
    showError: (error: Error | string) => void
): void => {
    console.error(`[${context}] Execution failed:`, error);
    if (error instanceof Error && error.message.includes('aborted')) {
        showInfo('Execution aborted', true);
        return;
    }
    showError(error as Error);
};
