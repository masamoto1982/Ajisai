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
};

// `setArea` realizes a Presentation Profile transition (SPEC Portability
// Profiles): selecting one observation surface (SPEC §12.3) drives the
// device-appropriate transition core via `applyAreaState`. Invariants 1–6 of
// the Presentation Profile are verified in `./presentation-profile.test.ts`.
export const createLayoutController = (deps: LayoutControllerDeps): LayoutController => {
    const setArea = (mode: ViewMode): void => {
        deps.state.currentMode = mode;
        applyAreaState(deps.buildApplyAreaStateDeps(), mode);
    };

    const handleResize = (): void => {
        applyAreaState(deps.buildApplyAreaStateDeps(), deps.state.currentMode);
        updateEditorPlaceholder(deps.elements, deps.mobile);
    };

    return {
        getState: () => deps.state,
        setArea,
        handleResize
    };
};
