// Presentation layer for Ajisai's four observation surfaces (SPEC §12.3:
// Input/π_Input, Output/π_Output, Stack/π_Stack, Dictionary/π_Dict). The
// concrete way those surfaces are made visible on a device is a "Presentation
// Profile" (SPEC Portability Profiles): a labeled transition system over
// visibility configurations. This module holds the desktop initial
// configuration c0; the transition cores live in `../gui-layout-state`
// (desktop) and `../mobile-view-switcher` (single-surface), and the spec
// invariants are checked in `./presentation-profile.test.ts`.
import type { LayoutState } from '../gui-layout-state';

export type GuiLayoutState = LayoutState;

export const createGuiLayoutState = (): GuiLayoutState => ({
    currentMode: 'input',
    currentLeftMode: 'input',
    currentRightMode: 'stack'
});
