import type { AjisaiRuntime } from '../core/ajisai-runtime-types';
import { createGUI } from '../gui/gui-application';

export interface ElectronRendererBootstrap {
    readonly runtime: AjisaiRuntime;
    readonly root: ParentNode;
}

export const startElectronRenderer = async ({ runtime, root }: ElectronRendererBootstrap): Promise<void> => {
    const gui = createGUI({ runtime, root });
    await gui.init();
    gui.updateAllDisplays();
};
