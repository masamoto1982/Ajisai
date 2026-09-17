import { WORKER_MANAGER } from '../workers/execution-worker-manager';
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
import {
    checkIsStationary,
    createMultiTapRecognizer,
    type GesturePoint
} from './touch-gestures';

// Gesture tuning, in the same class as the mobile breakpoint: a run of taps is
// one gesture while the taps stay inside this much time and this much of the
// screen. The tolerance is generous enough for a thumb that does not land
// twice on the same pixel and tight enough that a drag-to-select is not three
// taps in a row.
const MULTI_TAP_INTERVAL_MS = 500;
const TAP_MOVEMENT_TOLERANCE_PX = 24;

// Reset is the one operation that throws away the stack *and* the dictionary,
// so both of its triggers — the shortcut and the mobile button — ask first.
const RESET_CONFIRM_MESSAGE = 'Are you sure you want to reset the system?';

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
        const recognizer = createMultiTapRecognizer({
            intervalMs: MULTI_TAP_INTERVAL_MS,
            movementTolerancePx: TAP_MOVEMENT_TOLERANCE_PX
        });

        target.addEventListener('click', (e: MouseEvent) => {
            if (!mobile.isMobile()) return;
            if (layoutState.currentMode !== activeMode) return;
            if ((e.target as HTMLElement).closest('button, a')) return;

            if (recognizer.registerTap({ x: e.clientX, y: e.clientY }, Date.now()) >= 2) {
                recognizer.reset();
                switchArea(nextMode);
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

    elements.editorClearBtn.addEventListener('click', () => editor.clear());
    // Same control, same corner, same gesture as clearing the editor — the
    // Stack area's `×` throws away the values and keeps the dictionary, which
    // is what separates it from Reset.
    elements.stackClearBtn.addEventListener('click', () => clearStack());
    elements.editorFormatBtn.addEventListener('click', () => editor.format());

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

    // The touch action bar under the editor. Every operation on it also has a
    // keyboard shortcut, and on a device with no hardware keyboard the shortcut
    // is not a route to anything — Run, Step, Abort, Lookup and Reset had no
    // on-screen control at all, so a program could be typed on a phone and then
    // neither stepped nor reset. These buttons are that route; the shortcuts and
    // the triple-tap keep working unchanged.
    elements.touchRunBtn.addEventListener('click', () => runEditorCode());
    elements.touchStepBtn.addEventListener('click', () => { void executionController.executeStep(); });
    // The same pair the Escape branch below runs, and equally harmless with
    // nothing in flight.
    elements.touchAbortBtn.addEventListener('click', () => {
        WORKER_MANAGER.abortAll();
        executionController.abortExecution();
    });
    elements.touchLookupBtn.addEventListener('click', () => {
        executionController.lookupWord(editor.getWordAtCursor());
    });
    elements.touchResetBtn.addEventListener('click', () => {
        if (confirm(RESET_CONFIRM_MESSAGE)) {
            void executionController.executeReset();
        }
    });



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

    // Triple-tap the editor to Run. This is the shortcut, not the only way in:
    // the Run button below the editor is, and a gesture that shares its shape
    // with the OS's own paragraph-select must never be the sole route to
    // running a program. What it must be is deliberate, so a tap here is a
    // touch that went down and came up in the same place, on its own: the end
    // of a drag-to-select and one release out of a pinch are not taps, and
    // before this guard three of either ran the program.
    {
        const recognizer = createMultiTapRecognizer({
            intervalMs: MULTI_TAP_INTERVAL_MS,
            movementTolerancePx: TAP_MOVEMENT_TOLERANCE_PX
        });
        let touchOrigin: GesturePoint | null = null;

        elements.codeInput.addEventListener('touchstart', (e: TouchEvent) => {
            const touch = e.changedTouches[0];
            if (e.touches.length > 1 || !touch) {
                recognizer.reset();
                touchOrigin = null;
                return;
            }
            touchOrigin = { x: touch.clientX, y: touch.clientY };
        }, { passive: true });

        elements.codeInput.addEventListener('touchend', (e: TouchEvent) => {
            const origin = touchOrigin;
            touchOrigin = null;
            if (!mobile.isMobile()) return;

            const touch = e.changedTouches[0];
            if (origin === null || !touch || e.touches.length > 0) {
                recognizer.reset();
                return;
            }

            const end: GesturePoint = { x: touch.clientX, y: touch.clientY };
            if (!checkIsStationary(origin, end, TAP_MOVEMENT_TOLERANCE_PX)) {
                recognizer.reset();
                return;
            }

            if (recognizer.registerTap(end, Date.now()) >= 3) {
                recognizer.reset();
                // Run; the post-execution auto-navigation (applyExecutionAreaState)
                // chooses the destination surface from what actually changed, so
                // we deliberately do not force a switch to Stack here.
                runEditorCode();
            }
        }, { passive: true });
    }

    {
        const recognizer = createMultiTapRecognizer({
            intervalMs: MULTI_TAP_INTERVAL_MS,
            movementTolerancePx: TAP_MOVEMENT_TOLERANCE_PX
        });

        elements.codeInput.addEventListener('click', (e: MouseEvent) => {
            if (mobile.isMobile()) return;

            if (recognizer.registerTap({ x: e.clientX, y: e.clientY }, Date.now()) >= 3) {
                recognizer.reset();
                runEditorCode();
            }
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
            if (confirm(RESET_CONFIRM_MESSAGE)) {
                executionController.executeReset();
            }
            e.preventDefault();
            e.stopImmediatePropagation();
        }
        // Ctrl+Alt+S clears the stack and Ctrl+Alt+E clears the editor. Both are
        // bound on the window rather than their buttons because the Stack area
        // can hold focus, and `e.code` so the binding does not move with the
        // layout. Neither confirms: unlike Reset, Stack clear loses only values
        // (one re-run away) and Editor clear loses only unsaved typing
        // (recoverable via Recall).
        if (e.code === 'KeyS' && e.ctrlKey && e.altKey && !e.shiftKey && !e.metaKey) {
            clearStack();
            e.preventDefault();
            e.stopImmediatePropagation();
        }
        if (e.code === 'KeyE' && e.ctrlKey && e.altKey && !e.shiftKey && !e.metaKey) {
            editor.clear();
            e.preventDefault();
            e.stopImmediatePropagation();
        }
        // Ctrl+Alt+L looks up the word at the cursor — no typed spelling, no
        // button, since the target comes from the cursor rather than an
        // argument someone could type or click. Silently does nothing when
        // the cursor is not on a word.
        if (e.code === 'KeyL' && e.ctrlKey && e.altKey && !e.shiftKey && !e.metaKey) {
            executionController.lookupWord(editor.getWordAtCursor());
            e.preventDefault();
            e.stopImmediatePropagation();
        }
    }, true);
}

export function bindGuiEvents(context: GuiEventBindingContext): void {
    bindLayoutEvents(context);
    bindInteractionEvents(context);
}
