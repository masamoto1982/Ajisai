import type { ViewMode } from '../mobile-view-switcher';
import type { LayoutState } from '../gui-layout-state';

export type GuiLayoutState = LayoutState;
export type GuiArea = ViewMode;

export const createGuiLayoutState = (): GuiLayoutState => ({
    currentMode: 'input',
    currentLeftMode: 'input',
    currentRightMode: 'stack'
});
