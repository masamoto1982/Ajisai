import type { LayoutState } from '../gui-layout-state';

export type GuiLayoutState = LayoutState;

export const createGuiLayoutState = (): GuiLayoutState => ({
    currentMode: 'input',
    currentLeftMode: 'input',
    currentRightMode: 'stack'
});
