import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import type { Display } from './output-display-renderer';
import type { Editor } from './code-input-editor';
import type { MobileHandler, ViewMode } from './mobile-view-switcher';
import type { ModuleTabManager } from './module-selector-sheets';
import type { Persistence } from './interpreter-state-persistence';
import type { ExecutionController } from './execution-controller';
import type { VocabularyManager } from './vocabulary-state-controller';
import type { GUIElements } from './gui-dom-cache';
import type { LayoutState } from './gui-layout-state';
import type { LayoutController } from './layout/layout-controller';

export type GuiEventBindingContext = {
    readonly elements: GUIElements;
    readonly mobile: MobileHandler;
    readonly layoutState: LayoutState;
    readonly layoutController: LayoutController;
    readonly moduleTabManager: ModuleTabManager;
    readonly vocabulary: VocabularyManager;
    readonly display: Display;
    readonly editor: Editor;
    readonly executionController: ExecutionController;
    readonly persistence: Persistence;
    readonly switchArea: (mode: ViewMode) => void;
    readonly updateAllDisplays: () => void;
    readonly doSwitchDictionarySheet: (sheetId: string) => void;
};

const debounce = <T extends (...args: unknown[]) => void>(
    fn: T,
    delay: number
): ((...args: Parameters<T>) => void) => {
    let timeoutId: ReturnType<typeof setTimeout> | null = null;
    return (...args: Parameters<T>) => {
        if (timeoutId) clearTimeout(timeoutId);
        timeoutId = setTimeout(() => fn(...args), delay);
    };
};

function bindLayoutEvents(context: GuiEventBindingContext): void {
    const {
        elements,
        mobile,
        layoutState,
        switchArea,
        doSwitchDictionarySheet,
        layoutController
    } = context;

    elements.leftPanelSelect.addEventListener('change', () => {
        switchArea(elements.leftPanelSelect.value as ViewMode);
    });
    elements.rightPanelSelect.addEventListener('change', () => {
        switchArea(elements.rightPanelSelect.value as ViewMode);
    });
    elements.mobilePanelSelect.addEventListener('change', () => {
        switchArea(elements.mobilePanelSelect.value as ViewMode);
    });

    elements.dictionarySheetSelect.addEventListener('change', () => {
        doSwitchDictionarySheet(elements.dictionarySheetSelect.value);
    });

    const setupDoubleTapToTransition = (
        target: HTMLElement,
        activeMode: ViewMode,
        nextMode: ViewMode
    ): void => {
        const MULTI_TAP_INTERVAL_MS = 500;
        let tapCount = 0;
        let lastTapAt = 0;

        target.addEventListener('click', (e: MouseEvent) => {
            if (!mobile.isMobile()) return;
            if (layoutState.currentMode !== activeMode) return;
            if ((e.target as HTMLElement).closest('button, a')) return;

            const now = Date.now();
            if (now - lastTapAt <= MULTI_TAP_INTERVAL_MS) {
                tapCount += 1;
            } else {
                tapCount = 1;
            }
            lastTapAt = now;

            if (tapCount >= 2) {
                switchArea(nextMode);
                tapCount = 0;
                lastTapAt = 0;
            }
        });
    };

    setupDoubleTapToTransition(elements.stackDisplay, 'stack', 'output');
    setupDoubleTapToTransition(elements.outputDisplay, 'output', 'input');

    window.addEventListener('resize', () => {
        layoutController.handleResize();
    });
}

function bindInteractionEvents(context: GuiEventBindingContext): void {
    const { elements, vocabulary, moduleTabManager, editor, mobile, layoutState, switchArea, display, persistence, executionController, updateAllDisplays } = context;
    const applySearchFilter = (filter: string): void => {
        elements.dictionarySearch.value = filter;
        elements.mobileDictionarySearch.value = filter;
        vocabulary.updateSearchFilter(filter);
        moduleTabManager.updateSearchFilter(filter);
    };

    const applySearchInput = debounce(() => {
        applySearchFilter(elements.dictionarySearch.value);
    }, 150);

    const applyMobileSearchInput = debounce(() => {
        applySearchFilter(elements.mobileDictionarySearch.value);
    }, 150);

    elements.dictionarySearch.addEventListener('input', applySearchInput);
    elements.mobileDictionarySearch.addEventListener('input', applyMobileSearchInput);
    elements.dictionarySearchClearBtn.addEventListener('click', () => applySearchFilter(''));
    elements.mobileDictionarySearchClearBtn.addEventListener('click', () => applySearchFilter(''));

    elements.clearBtn.addEventListener('click', () => editor.clear());

    elements.testBtn?.addEventListener('click', async () => {
        switchArea('output');
        const { createTestRunner } = await import('./gui-test-runner');
        const testRunner = createTestRunner({
            showInfo: (text: string, append: boolean) => display.renderInfo(text, append),
            showError: (error: Error | string) => display.renderError(error),
            updateDisplays: updateAllDisplays
        });
        testRunner.runAllTests();
    });

    elements.outputArea.addEventListener('dblclick', (e: MouseEvent) => {
        if ((e.target as HTMLElement).closest('button, a')) return;
        if (!mobile.isMobile() && layoutState.currentLeftMode === 'output') {
            switchArea('input');
            editor.focus();
        }
    });

    elements.copyOutputBtn.addEventListener('click', (e: MouseEvent) => {
        e.stopPropagation();
        const text = display.extractState().mainOutput;
        navigator.clipboard.writeText(text).then(() => {
            const btn = elements.copyOutputBtn;
            const original = btn.textContent;
            btn.textContent = 'Copied!';
            setTimeout(() => { btn.textContent = original; }, 1500);
        });
    });

    elements.exportBtn?.addEventListener('click', () => persistence.exportUserWords());
    elements.importBtn?.addEventListener('click', () => persistence.importUserWords());

    elements.codeInput.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Enter' && e.shiftKey) {
            e.preventDefault();
            executionController.executeCode(editor.extractValue());
        }
        if (e.key === 'Enter' && e.ctrlKey && !e.altKey && !e.shiftKey) {
            e.preventDefault();
            executionController.executeStep();
        }
    });

    {
        const MULTI_TAP_INTERVAL_MS = 500;
        let tapCount = 0;
        let lastTapAt = 0;

        elements.codeInput.addEventListener('touchend', (e: TouchEvent) => {
            if (!mobile.isMobile()) return;
            if (e.changedTouches.length === 0) return;

            const now = Date.now();
            if (now - lastTapAt <= MULTI_TAP_INTERVAL_MS) {
                tapCount += 1;
            } else {
                tapCount = 1;
            }

            if (tapCount >= 3) {
                executionController.executeCode(editor.extractValue());
                switchArea('stack');
                tapCount = 0;
                lastTapAt = 0;
                return;
            }

            lastTapAt = now;
        }, { passive: true });
    }

    {
        const MULTI_CLICK_INTERVAL_MS = 500;
        let clickCount = 0;
        let lastClickAt = 0;

        elements.codeInput.addEventListener('click', () => {
            if (mobile.isMobile()) return;

            const now = Date.now();
            if (now - lastClickAt <= MULTI_CLICK_INTERVAL_MS) {
                clickCount += 1;
            } else {
                clickCount = 1;
            }

            if (clickCount >= 3) {
                executionController.executeCode(editor.extractValue());
                clickCount = 0;
                lastClickAt = 0;
                return;
            }

            lastClickAt = now;
        });
    }

    window.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
            WORKER_MANAGER.abortAll();
            executionController.abortExecution();
            e.preventDefault();
            e.stopImmediatePropagation();
        }
        if (e.key === 'Enter' && e.ctrlKey && e.altKey) {
            if (confirm('Are you sure you want to reset the system?')) {
                executionController.executeReset();
            }
            e.preventDefault();
            e.stopImmediatePropagation();
        }
    }, true);
}

export function bindGuiEvents(context: GuiEventBindingContext): void {
    bindLayoutEvents(context);
    bindInteractionEvents(context);
}
