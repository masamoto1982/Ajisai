import type { AjisaiRuntime } from '../core/ajisai-runtime-types';
import { createGUI } from '../gui/gui-application';
import type { PlatformServices } from '../platform/platform-services';

export interface ElectronRendererBootstrap {
    readonly runtime: AjisaiRuntime;
    readonly root: ParentNode;
    readonly platform: PlatformServices;
}

export const startElectronRenderer = async ({ runtime, root, platform }: ElectronRendererBootstrap): Promise<void> => {
    const gui = createGUI({ runtime, root, platform });
    await gui.init();
    gui.updateAllDisplays();
};
