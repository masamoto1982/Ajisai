import { applyAreaState, type ApplyAreaStateDeps, type LayoutState, updateEditorPlaceholder } from '../gui-layout-state';
import type { ViewMode } from '../mobile-view-switcher';
import type { GUIElements } from '../gui-dom-cache';
import type { MobileHandler } from '../mobile-view-switcher';

export type LayoutController = {
    readonly getState: () => LayoutState;
    readonly setArea: (mode: ViewMode) => void;
    readonly handleResize: () => void;
};

export type LayoutControllerDeps = {
    readonly state: LayoutState;
    readonly elements: GUIElements;
    readonly mobile: MobileHandler;
    readonly buildApplyAreaStateDeps: () => ApplyAreaStateDeps;
    readonly syncDictionarySearchVisibility: () => void;
};

export const createLayoutController = (deps: LayoutControllerDeps): LayoutController => {
    const setArea = (mode: ViewMode): void => {
        deps.state.currentMode = mode;
        applyAreaState(deps.buildApplyAreaStateDeps(), mode);
        deps.syncDictionarySearchVisibility();
    };

    const handleResize = (): void => {
        applyAreaState(deps.buildApplyAreaStateDeps(), deps.state.currentMode);
        deps.syncDictionarySearchVisibility();
        updateEditorPlaceholder(deps.elements, deps.mobile);
    };

    return {
        getState: () => deps.state,
        setArea,
        handleResize
    };
};
