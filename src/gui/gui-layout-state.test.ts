import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
    analyzeStackModifiers,
    applyExecutionAreaState,
    type ApplyAreaStateDeps,
    type LayoutState,
} from './gui-layout-state';
import type { ViewMode } from './mobile-view-switcher';

const makeElement = () => ({ hidden: false }) as HTMLElement;

const makeDeps = (mobileMode: boolean, state: LayoutState): ApplyAreaStateDeps => {
    const dictionarySheetSelect = { value: 'core' } as HTMLSelectElement;
    return {
        elements: {
            inputArea: makeElement(),
            outputArea: makeElement(),
            stackArea: makeElement(),
            dictionaryArea: makeElement(),
            leftPanelSelect: { value: '' } as HTMLSelectElement,
            rightPanelSelect: { value: '' } as HTMLSelectElement,
            mobilePanelSelect: { value: '' } as HTMLSelectElement,
            dictionarySheetSelect,
        } as unknown as ApplyAreaStateDeps['elements'],
        state,
        mobile: {
            isMobile: () => mobileMode,
            extractCurrentMode: () => state.currentMode,
            updateView: vi.fn((mode: ViewMode) => { state.currentMode = mode; }),
        },
        moduleTabManager: {
            lookupModuleArea: vi.fn(() => null),
        } as unknown as ApplyAreaStateDeps['moduleTabManager'],
        switchDictionarySheet: vi.fn(),
    };
};

describe('analyzeStackModifiers', () => {
    it('defaults to TOP and EAT when no modifier token is present', () => {
        expect(analyzeStackModifiers('1 2 ADD')).toEqual({ stak: false, keep: false });
        expect(analyzeStackModifiers('')).toEqual({ stak: false, keep: false });
    });

    it('reads the canonical TOP/STAK and EAT/KEEP tokens', () => {
        expect(analyzeStackModifiers('. ADD')).toEqual({ stak: false, keep: false });
        expect(analyzeStackModifiers('.. ADD')).toEqual({ stak: true, keep: false });
        expect(analyzeStackModifiers(', ADD')).toEqual({ stak: false, keep: false });
        expect(analyzeStackModifiers(',, ADD')).toEqual({ stak: false, keep: true });
    });

    it('reads the combined modifier forms of SPEC §6.3', () => {
        expect(analyzeStackModifiers('.,, ADD')).toEqual({ stak: false, keep: true });
        expect(analyzeStackModifiers('..,, ADD')).toEqual({ stak: true, keep: true });
        expect(analyzeStackModifiers('..,  ADD')).toEqual({ stak: true, keep: false });
    });

    it('reads the ; and ;; sugar (. , and .. ,,)', () => {
        expect(analyzeStackModifiers('; ADD')).toEqual({ stak: false, keep: false });
        expect(analyzeStackModifiers(';; ADD')).toEqual({ stak: true, keep: true });
    });

    it('never mistakes a decimal literal for a modifier', () => {
        expect(analyzeStackModifiers('.5 ADD')).toEqual({ stak: false, keep: false });
        expect(analyzeStackModifiers('5. ADD')).toEqual({ stak: false, keep: false });
        expect(analyzeStackModifiers('3.14 ADD')).toEqual({ stak: false, keep: false });
    });

    it('treats either axis as triggered if any token selects the non-default', () => {
        expect(analyzeStackModifiers('1 .. ADD 2 . SUB')).toEqual({ stak: true, keep: false });
        expect(analyzeStackModifiers('1 ,, ADD 2 , SUB')).toEqual({ stak: false, keep: true });
    });
});

describe('applyExecutionAreaState', () => {
    beforeEach(() => {
        vi.stubGlobal('document', { body: { dataset: {} } });
    });

    it('switches the desktop right pane to Stack for stack-only execution changes', () => {
        const state: LayoutState = {
            currentMode: 'dictionary',
            currentLeftMode: 'input',
            currentRightMode: 'dictionary',
        };
        const deps = makeDeps(false, state);

        applyExecutionAreaState(deps, {
            outputChanged: false,
            stackChanged: true,
            dictionaryChanged: false,
        });

        expect(state.currentLeftMode).toBe('input');
        expect(state.currentRightMode).toBe('stack');
        expect(deps.elements.stackArea.hidden).toBe(false);
        expect(deps.elements.dictionaryArea.hidden).toBe(true);
        expect(deps.elements.rightPanelSelect.value).toBe('stack');
    });

    it('keeps desktop panes unchanged when execution changes no observable surface', () => {
        const state: LayoutState = {
            currentMode: 'dictionary',
            currentLeftMode: 'input',
            currentRightMode: 'dictionary',
        };
        const deps = makeDeps(false, state);

        applyExecutionAreaState(deps, {
            outputChanged: false,
            stackChanged: false,
            dictionaryChanged: false,
        });

        expect(state.currentLeftMode).toBe('input');
        expect(state.currentRightMode).toBe('dictionary');
    });

    it('switches the desktop left pane to Output for output-only changes, keeping the right pane', () => {
        const state: LayoutState = {
            currentMode: 'stack',
            currentLeftMode: 'input',
            currentRightMode: 'stack',
        };
        const deps = makeDeps(false, state);

        applyExecutionAreaState(deps, {
            outputChanged: true,
            stackChanged: false,
            dictionaryChanged: false,
        });

        expect(state.currentLeftMode).toBe('output');
        expect(state.currentRightMode).toBe('stack');
        expect(deps.elements.outputArea.hidden).toBe(false);
        expect(deps.elements.inputArea.hidden).toBe(true);
    });

    it('shows Output on the left and Stack on the right when both surfaces change', () => {
        const state: LayoutState = {
            currentMode: 'dictionary',
            currentLeftMode: 'input',
            currentRightMode: 'dictionary',
        };
        const deps = makeDeps(false, state);

        applyExecutionAreaState(deps, {
            outputChanged: true,
            stackChanged: true,
            dictionaryChanged: false,
        });

        expect(state.currentLeftMode).toBe('output');
        expect(state.currentRightMode).toBe('stack');
    });

    it('reveals the changed Words sheet on the desktop right pane for dictionary changes', () => {
        const state: LayoutState = {
            currentMode: 'stack',
            currentLeftMode: 'input',
            currentRightMode: 'stack',
        };
        const deps = makeDeps(false, state);

        applyExecutionAreaState(deps, {
            outputChanged: false,
            stackChanged: false,
            dictionaryChanged: true,
            dictionarySheetId: 'user',
        });

        expect(state.currentRightMode).toBe('dictionary');
        expect(deps.elements.dictionarySheetSelect.value).toBe('user');
        expect(deps.switchDictionarySheet).toHaveBeenCalledWith('user');
    });

    it('lets Dictionary outrank Stack for the desktop right pane when both change', () => {
        const state: LayoutState = {
            currentMode: 'input',
            currentLeftMode: 'input',
            currentRightMode: 'stack',
        };
        const deps = makeDeps(false, state);

        applyExecutionAreaState(deps, {
            outputChanged: false,
            stackChanged: true,
            dictionaryChanged: true,
            dictionarySheetId: 'module-math',
        });

        expect(state.currentRightMode).toBe('dictionary');
        expect(deps.elements.dictionarySheetSelect.value).toBe('module-math');
        expect(deps.switchDictionarySheet).toHaveBeenCalledWith('module-math');
    });

    it('reveals the changed Words sheet on mobile when the dictionary changes', () => {
        const state: LayoutState = {
            currentMode: 'input',
            currentLeftMode: 'input',
            currentRightMode: 'stack',
        };
        const deps = makeDeps(true, state);

        applyExecutionAreaState(deps, {
            outputChanged: false,
            stackChanged: false,
            dictionaryChanged: true,
            dictionarySheetId: 'user',
        });

        expect(state.currentMode).toBe('dictionary');
        expect(deps.elements.dictionarySheetSelect.value).toBe('user');
        expect(deps.switchDictionarySheet).toHaveBeenCalledWith('user');
        expect(deps.elements.mobilePanelSelect.value).toBe('dictionary');
    });

    it('uses Dictionary before Output and Stack on mobile because only one surface is visible', () => {
        const state: LayoutState = {
            currentMode: 'input',
            currentLeftMode: 'input',
            currentRightMode: 'stack',
        };
        const deps = makeDeps(true, state);

        applyExecutionAreaState(deps, {
            outputChanged: true,
            stackChanged: true,
            dictionaryChanged: true,
        });

        expect(state.currentMode).toBe('dictionary');
        expect(deps.elements.mobilePanelSelect.value).toBe('dictionary');
    });
});
