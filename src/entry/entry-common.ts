import { getPlatform } from '../platform';
import { GUI_INSTANCE } from '../gui/gui-application';
import { initWasm } from '../wasm-module-loader';
import type { WasmModule, AjisaiInterpreter, HostProfile } from '../wasm-interpreter-types';

declare const __AJISAI_BUILD_TIMESTAMP__: string;

declare global {
    interface Window {
        AjisaiWasm: WasmModule;
        ajisaiInterpreter: AjisaiInterpreter;
    }
}

function formatTimestamp(date: Date): string {
    const year = date.getFullYear();
    const month = `${date.getMonth() + 1}`.padStart(2, '0');
    const day = `${date.getDate()}`.padStart(2, '0');
    const hours = `${date.getHours()}`.padStart(2, '0');
    const minutes = `${date.getMinutes()}`.padStart(2, '0');
    return `${year}${month}${day}${hours}${minutes}`;
}

export function setBuildVersionLabel(): void {
    const versionElement = document.querySelector<HTMLElement>('.version');
    if (!versionElement) return;

    const timestamp = __AJISAI_BUILD_TIMESTAMP__ || formatTimestamp(new Date());
    versionElement.textContent = `ver.${timestamp}`;
}

/**
 * Say which resource profile this host applies, beside the build version.
 *
 * SPEC §2.5 makes limits a host safety control rather than value semantics, so
 * two conforming hosts legitimately enforce different ceilings — and they do:
 * `[ 0 100001 ] RANGE` materializes here and answers `NIL(spaceExhausted)`
 * under the MCP agent profile. That difference is only a trap when neither
 * host discloses what it applies, which is what this fixes on this side.
 * See docs/dev/mcp-host-profiles.md for the comparison.
 */
export function setHostProfileLabel(interpreter: AjisaiInterpreter): void {
    const element = document.querySelector<HTMLElement>('#host-profile');
    if (!element) return;
    let profile: HostProfile;
    try {
        profile = JSON.parse(interpreter.host_profile()) as HostProfile;
    } catch {
        return;
    }
    element.hidden = false;
    element.textContent = `resource limits: ${profile.profile}`;
    element.title = [
        'Resource limits enforced by this host:',
        ...Object.entries(profile.limits).map(([name, value]) => `${name}: ${value.toLocaleString()}`),
    ].join('\n');
}

export async function initializeApplication(): Promise<void> {
    console.log('[Main] Starting Ajisai application...');

    try {
        console.log('[Main] Initializing WASM...');
        const wasm = await initWasm();
        if (!wasm) {
            throw new Error('WASM initialization failed. Application cannot start.');
        }
        window.AjisaiWasm = wasm;

        console.log('[Main] Creating main thread interpreter...');
        window.ajisaiInterpreter = new window.AjisaiWasm.AjisaiInterpreter();
        setHostProfileLabel(window.ajisaiInterpreter);

        console.log('[Main] Initializing GUI...');
        await GUI_INSTANCE.init();
        GUI_INSTANCE.updateAllDisplays();

        console.log('[Main] Application initialization completed successfully');
    } catch (error) {
        console.error('[Main] Application startup failed:', error);
        const outputDisplay = document.getElementById('output-display');
        if (outputDisplay) {
            outputDisplay.innerHTML = '';
            const errorSpan = document.createElement('span');
            errorSpan.style.color = '#dc3545';
            errorSpan.style.fontWeight = 'bold';
            errorSpan.textContent = `Application startup failed: ${(error as Error).message}`;
            outputDisplay.appendChild(errorSpan);
        }
    }
}

export function bootstrapApplication(): void {
    getPlatform().runtime.onReady(() => {
        setBuildVersionLabel();
        void initializeApplication();
    });
}
