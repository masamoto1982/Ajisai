import { WORKER_MANAGER } from '../workers/execution-worker-manager';
import { getPlatform } from '../platform';
import type { Display } from './output-display-renderer';
import type { Editor } from './code-input-editor';
import type { MobileHandler, ViewMode } from './mobile-view-switcher';
import type { Persistence } from './interpreter-state-persistence';
import type { ExecutionController } from './execution-controller';
import type { VocabularyManager } from './vocabulary-state-controller';
import type { GUIElements } from './gui-dom-cache';
import type { LayoutState } from './gui-layout-state';
import type { LayoutController } from './layout/layout-controller';
import { createEditorHistory } from './editor-history';

export type GuiEventBindingContext = {
    readonly elements: GUIElements;
    readonly mobile: MobileHandler;
    readonly layoutState: LayoutState;
    readonly layoutController: LayoutController;
    readonly vocabulary: VocabularyManager;
    readonly display: Display;
    readonly editor: Editor;
    readonly executionController: ExecutionController;
    readonly persistence: Persistence;
    readonly switchArea: (mode: ViewMode) => void;
    readonly updateAllDisplays: () => void;
    /// Discard every value on the stack, leaving the dictionary alone. Lives on
    /// the context rather than being reached from here because the interpreter
    /// client is owned by the application module.
    readonly clearStack: () => void;
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
        layoutController,
        persistence
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
        void persistence.saveCurrentState();
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
    const { elements, vocabulary, editor, mobile, layoutState, switchArea, display, persistence, executionController, clearStack } = context;
    // Session-lived recall of submitted programs, so a run (which clears the
    // editor) and a Reset are both recoverable. See editor-history.ts.
    const history = createEditorHistory();
    const applySearchFilter = (filter: string): void => {
        elements.dictionarySearch.value = filter;
        elements.mobileDictionarySearch.value = filter;
        vocabulary.updateSearchFilter(filter);
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
    // Same control, same corner, same gesture as clearing the editor — the
    // Stack area's `×` throws away the values and keeps the dictionary, which
    // is what separates it from Reset.
    elements.stackClearBtn.addEventListener('click', () => clearStack());
    elements.formatBtn.addEventListener('click', () => editor.format());

    // Reformat the editor before running it, so the source that defines words
    // (and therefore the stored definition) is tidied at execution time.
    //
    // The submitted source is recorded before execution, not after: a run that
    // succeeds clears the editor and a Reset clears it too, so recording later
    // would be recording exactly the cases the user can no longer recover.
    const runEditorCode = (): void => {
        editor.format();
        const source = editor.extractValue();
        history.record(source);
        executionController.executeCode(source);
    };

    // Ctrl+Up / Ctrl+Down walk the session's submitted programs back into the
    // editor. The plain arrows are left alone — they are how you move the caret
    // through a multi-line program, and the suggestion panel already uses them
    // to move through its list.
    const recallHistory = (direction: 'older' | 'newer'): void => {
        const recalled = direction === 'older'
            ? history.recallOlder(elements.codeInput.value)
            : history.recallNewer();
        if (recalled === null) return;
        editor.updateValue(recalled);
    };

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
    elements.importJsonBtn?.addEventListener('click', () => persistence.importJsonAsVector());



    elements.codeInput.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Enter' && e.shiftKey) {
            e.preventDefault();
            runEditorCode();
        }
        if (e.key === 'Enter' && e.ctrlKey && !e.altKey && !e.shiftKey) {
            e.preventDefault();
            executionController.executeStep();
        }
        if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && e.ctrlKey && !e.altKey && !e.shiftKey) {
            e.preventDefault();
            recallHistory(e.key === 'ArrowUp' ? 'older' : 'newer');
        }
        // Shift+Alt+F reformats the editor contents (matches the format button).
        if (e.code === 'KeyF' && e.altKey && e.shiftKey && !e.ctrlKey && !e.metaKey) {
            e.preventDefault();
            editor.format();
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
                // Run; the post-execution auto-navigation (applyExecutionAreaState)
                // chooses the destination surface from what actually changed, so
                // we deliberately do not force a switch to Stack here.
                runEditorCode();
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
                runEditorCode();
                clickCount = 0;
                lastClickAt = 0;
                return;
            }

            lastClickAt = now;
        });
    }

    window.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
            // This listener captures and stops propagation, so the editor's own
            // Escape branch never sees the key: an open suggestion panel could
            // not be dismissed the way every other editor dismisses one, and the
            // panel stayed over the code while the user pressed Escape at it.
            // Dismissing takes priority; Abort still gets Escape whenever there
            // is no panel to close.
            if (editor.dismissSuggestions()) {
                e.preventDefault();
                e.stopImmediatePropagation();
                return;
            }
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
        // Shift+Alt+C clears the stack, following Shift+Alt+F for Format. It is
        // bound on the window rather than the editor because the Stack area can
        // hold focus, and `e.code` so the binding does not move with the layout.
        // No confirmation: unlike Reset this loses only values, and the same
        // values are one re-run away.
        if (e.code === 'KeyC' && e.altKey && e.shiftKey && !e.ctrlKey && !e.metaKey) {
            clearStack();
            e.preventDefault();
            e.stopImmediatePropagation();
        }
    }, true);
}

export function bindGuiEvents(context: GuiEventBindingContext): void {
    bindLayoutEvents(context);
    bindInteractionEvents(context);
}
