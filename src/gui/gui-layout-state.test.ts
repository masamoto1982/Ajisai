import { beforeEach, describe, expect, it, vi } from 'vitest';
import { applyExecutionAreaState, type ApplyAreaStateDeps, type LayoutState } from './gui-layout-state';
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
