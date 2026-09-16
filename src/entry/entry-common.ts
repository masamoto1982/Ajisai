import { getPlatform } from '../platform';
import { GUI_INSTANCE } from '../gui/gui-application';
import { initWasm } from '../wasm-module-loader';
import { EXECUTION_TIMEOUT_MS } from '../workers/execution-timeout';
import type { WasmModule, AjisaiInterpreter, HostProfile } from '../wasm-interpreter-types';

declare const __AJISAI_BUILD_TIMESTAMP__: string;
declare const __AJISAI_RELEASE_VERSION__: string;

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

/**
 * Apply a mutation to every element matching any of the given selectors.
 * The header badges and the splash screen (see initSplashScreen()) each keep
 * their own copy of the version/build/resource-limit labels — the splash
 * reaches a touch device before the header's hover-only tooltip ever could —
 * so both need to be written in lockstep from the same source data.
 */
function setLabelForAll(selectors: string[], mutate: (el: HTMLElement) => void): void {
    for (const selector of selectors) {
        const el = document.querySelector<HTMLElement>(selector);
        if (el) mutate(el);
    }
}

/**
 * Label the header badge `playground`, matching the Reference header's own
 * `リファレンス` badge, and put the release and build stamp on screen beside it.
 *
 * The build identity used to be hover-only — the badge read the constant word
 * `playground` and the version lived in a `title` tooltip. A tooltip does not
 * exist on a touch device and is not something anyone thinks to reach for, so
 * in practice the site said nothing about which build it was running, while
 * the MCP server reports `engineVersion` and `registryDigest` on every
 * response.
 *
 * That asymmetry matters because the Playground is a separately deployed
 * artifact and a deploy can be stranded — see the `workflow_dispatch` note in
 * `.github/workflows/build.yml` for the incident that added it. When that
 * happens the symptom is a site whose behaviour disagrees with the
 * specification, and with no version on screen there is nothing to compare
 * against. The badge keeps saying which page this is; the stamp says which
 * build, so the divergence is checkable by looking.
 */
export function setBuildVersionLabel(): void {
    const timestamp = __AJISAI_BUILD_TIMESTAMP__ || formatTimestamp(new Date());
    const compareNote = 'Compare against the repository when the Playground disagrees with the specification.';

    setLabelForAll(['.version'], (el) => {
        el.textContent = 'playground';
        el.title = `Ajisai ${__AJISAI_RELEASE_VERSION__}\nBuild ${timestamp}`;
    });

    setLabelForAll(['#build-stamp', '#splash-build-stamp'], (el) => {
        el.hidden = false;
        el.textContent = `v${__AJISAI_RELEASE_VERSION__} · ${timestamp}`;
        el.title = `Ajisai ${__AJISAI_RELEASE_VERSION__}, playground build ${timestamp}.\n${compareNote}`;
    });

    // Written out as plain text in the splash, not tucked into a hover title —
    // a touch device has no hover to tuck it behind.
    setLabelForAll(['#splash-build-note'], (el) => {
        el.hidden = false;
        el.textContent = compareNote;
    });
}

/**
 * Say which resource profile this host applies, beside the build version.
 *
 * LANG.MACHINE.LIMITS makes limits a host safety control rather than value semantics, so
 * two conforming hosts legitimately enforce different ceilings — and they do:
 * `[ 0 100001 ] RANGE` materializes here and answers `NIL(spaceExhausted)`
 * under the MCP agent profile. That difference is only a trap when neither
 * host discloses what it applies, which is what this fixes on this side.
 * See docs/dev/mcp-host-profiles.md for the comparison.
 */
export function setHostProfileLabel(interpreter: AjisaiInterpreter): void {
    let profile: HostProfile;
    try {
        profile = JSON.parse(interpreter.host_profile()) as HostProfile;
    } catch {
        return;
    }
    const text = `resource limits: ${profile.profile}`;
    const limitLines = [
        ...Object.entries(profile.limits).map(([name, value]) => `${name}: ${value.toLocaleString()}`),
        // The wall-clock guard is this host's alone and is not one of the
        // interpreter's ceilings, so `host_profile()` cannot report it — and a
        // list that omits the guard most likely to stop a long run reads as a
        // complete list that is wrong.
        `executionTimeoutMs: ${EXECUTION_TIMEOUT_MS.toLocaleString()} (wall clock, this host only)`,
    ];

    // The header badge stays a badge: summary text on screen, the ceiling
    // table on hover — a desktop convenience that costs nothing there.
    setLabelForAll(['#host-profile'], (el) => {
        el.hidden = false;
        el.textContent = text;
        el.title = ['Resource limits enforced by this host:', ...limitLines].join('\n');
    });

    // The splash has no hover to fall back on, so it gets the full ceiling
    // table written out as plain, always-visible text instead of a title.
    setLabelForAll(['#splash-host-profile'], (el) => {
        el.hidden = false;
        el.textContent = text;
    });
    const limitsList = document.querySelector<HTMLElement>('#splash-limits');
    if (limitsList) {
        limitsList.hidden = false;
        limitsList.replaceChildren(
            ...limitLines.map((line) => {
                const item = document.createElement('li');
                item.textContent = line;
                return item;
            })
        );
    }
}

/**
 * First-visit walkthrough: shows the Ajisai logo, then auto-reveals the same
 * technical detail the header badges carry (see setBuildVersionLabel() /
 * setHostProfileLabel() above, which write into both). Dismissed by click,
 * tap, or any key press — never a wait the user is forced to sit through —
 * and shown at most once per session so reloading mid-session to retry a
 * program never re-interrupts. Runs independently of WASM/GUI startup: the
 * splash is pure DOM and does not gate `initializeApplication()`.
 */
export function initSplashScreen(): void {
    const splash = document.querySelector<HTMLElement>('#splash-screen');
    if (!splash) return;

    const SESSION_KEY = 'ajisai-splash-seen';
    try {
        if (sessionStorage.getItem(SESSION_KEY) === '1') {
            splash.remove();
            return;
        }
    } catch {
        // sessionStorage unavailable (e.g. private browsing) - show the splash
        // every time; harmless, since it is never more than one extra tap.
    }

    splash.hidden = false;
    splash.focus();

    const DETAIL_REVEAL_DELAY_MS = 900;
    window.setTimeout(() => splash.classList.add('splash-detail-visible'), DETAIL_REVEAL_DELAY_MS);

    let dismissed = false;
    const dismiss = (): void => {
        if (dismissed) return;
        dismissed = true;
        try {
            sessionStorage.setItem(SESSION_KEY, '1');
        } catch {
            // Best effort only; worst case the splash reappears next reload.
        }
        splash.classList.add('splash-dismissing');
        splash.addEventListener('transitionend', () => splash.remove(), { once: true });
        // Fallback in case the transition never fires (e.g. reduced-motion
        // environments where the opacity change is instant).
        window.setTimeout(() => splash.remove(), 400);
    };

    splash.addEventListener('click', dismiss);
    splash.addEventListener('keydown', dismiss);
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
        initSplashScreen();
        setBuildVersionLabel();
        void initializeApplication();
    });
}
