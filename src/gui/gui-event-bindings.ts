import type { Display } from './output-display-renderer';
import type { Editor } from './code-input-editor';
import type { MobileHandler, ViewMode } from './mobile-view-switcher';
import type { Persistence } from './interpreter-state-persistence';
import type { ExecutionController } from './execution-controller';
import type { VocabularyManager } from './vocabulary-state-controller';
import type { GUIElements } from './gui-dom-cache';
import type { LayoutController, LayoutState } from './gui-layout-state';
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
const TAP_OPTIONS = { intervalMs: MULTI_TAP_INTERVAL_MS, movementTolerancePx: TAP_MOVEMENT_TOLERANCE_PX };

// `action` runs on the `count`th click of a run, while `enabled` holds.
const bindClickCount = (
    target: HTMLElement,
    count: number,
    enabled: (e: MouseEvent) => boolean,
    action: () => void
): void => {
    const recognizer = createMultiTapRecognizer(TAP_OPTIONS);
    target.addEventListener('click', (e: MouseEvent) => {
        if (!enabled(e)) return;
        if (recognizer.registerTap({ x: e.clientX, y: e.clientY }, Date.now()) >= count) {
            recognizer.reset();
            action();
        }
    });
};

// Reset is the one operation that throws away the stack *and* the dictionary,
// so it asks first.
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
    // Discard every value on the stack, leaving the dictionary alone.
    readonly clearStack: () => void;
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
    const { elements, mobile, layoutState, layoutController } = context;
    const switchArea = layoutController.setArea;

    for (const select of [elements.leftPanelSelect, elements.rightPanelSelect, elements.mobilePanelSelect]) {
        select.addEventListener('change', () => switchArea(select.value as ViewMode));
    }

    // On mobile a double-tap on Stack shows Output, and on Output shows Input.
    const bindDoubleTapTransition = (target: HTMLElement, activeMode: ViewMode, nextMode: ViewMode): void =>
        bindClickCount(
            target,
            2,
            (e) => mobile.isMobile()
                && layoutState.currentMode === activeMode
                && !(e.target as HTMLElement).closest('button, a'),
            () => switchArea(nextMode)
        );
    bindDoubleTapTransition(elements.stackDisplay, 'stack', 'output');
    bindDoubleTapTransition(elements.outputDisplay, 'output', 'input');

    window.addEventListener('resize', () => {
        layoutController.handleResize();
    });
}

function bindInteractionEvents(context: GuiEventBindingContext): void {
    const { elements, vocabulary, editor, mobile, layoutState, layoutController, display, persistence, executionController, clearStack } = context;
    const switchArea = layoutController.setArea;
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

    elements.exportBtn.addEventListener('click', () => persistence.exportUserWords());
    elements.importBtn.addEventListener('click', () => persistence.importUserWords());

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

    // Triple-tap the editor to Run — on mobile the one route, which the Input
    // surface's own text names (spec/gui-semantics.md, rule 4). A gesture that
    // shares its shape with the OS's own paragraph-select must therefore be
    // deliberate, so a tap here is a touch that went down and came up in the
    // same place, on its own: the end of a drag-to-select and one release out
    // of a pinch are not taps.
    {
        const recognizer = createMultiTapRecognizer(TAP_OPTIONS);
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

    // On desktop a triple-click on the editor runs the program, as Shift+Enter does.
    bindClickCount(elements.codeInput, 3, () => !mobile.isMobile(), runEditorCode);

    window.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
            // This listener captures and stops propagation, so the editor's own
            // Escape branch never sees the key. Dismissing an open suggestion
            // panel takes priority; Abort still gets Escape whenever there is
            // no panel to close.
            if (editor.dismissSuggestions()) {
                e.preventDefault();
                e.stopImmediatePropagation();
                return;
            }
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
