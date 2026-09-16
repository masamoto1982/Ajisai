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

// Fixed once at load: the build and host-profile labels are written at
// different moments (the second only after the interpreter is up) and both
// state this stamp, so deriving it per call would let the fallback branch
// report two different times for one page.
const BUILD_TIMESTAMP = __AJISAI_BUILD_TIMESTAMP__ || formatTimestamp(new Date());

const COMPARE_NOTE =
    'Compare against the repository when the Playground disagrees with the specification.';

/** The build half of the detail, in the order the splash screen lists it. */
function buildDetailLines(): string[] {
    return [`Version: ${__AJISAI_RELEASE_VERSION__}`, `Build: ${BUILD_TIMESTAMP}`, COMPARE_NOTE];
}

/**
 * Apply a mutation to every element matching any of the given selectors.
 *
 * Every match, not just the first: the header and the splash screen (see
 * initSplashScreen()) each carry their own `.version` badge, so a
 * `querySelector` here would write to whichever comes first in the document
 * — the splash's copy, which is then removed on dismissal — and silently
 * leave the header's behind.
 */
function setLabelForAll(selectors: string[], mutate: (el: HTMLElement) => void): void {
    for (const selector of selectors) {
        document.querySelectorAll<HTMLElement>(selector).forEach(mutate);
    }
}

/**
 * Label the `playground` badge, matching the Reference header's own
 * `リファレンス` badge, and state which build is deployed.
 *
 * Which build matters because the Playground is a separately deployed
 * artifact and a deploy can be stranded — see the `workflow_dispatch` note in
 * `.github/workflows/build.yml` for the incident that added it. When that
 * happens the symptom is a site whose behaviour disagrees with the
 * specification, and with no version anywhere there is nothing to compare
 * against.
 *
 * The header says it on hover and the splash says it in plain text, which is
 * what makes the pair work: a tooltip does not exist on a touch device, so
 * the splash is how the detail reaches one — and once it has, the header does
 * not need to keep spending its brand row on a stamp nobody reads twice.
 */
export function setBuildVersionLabel(): void {
    setLabelForAll(['.version'], (el) => {
        el.textContent = 'playground';
    });
    // Until the interpreter is up this is all there is to tell; the host
    // profile rewrites this tooltip with its ceilings appended.
    setPlaygroundBadgeTooltip(buildDetailLines());

    // The splash spells the same thing out on screen, one labeled line each.
    setLabelForAll(['#splash-version-line'], (el) => {
        el.hidden = false;
        el.textContent = `Version: ${__AJISAI_RELEASE_VERSION__}`;
    });
    setLabelForAll(['#splash-build-line'], (el) => {
        el.hidden = false;
        el.textContent = `Build: ${BUILD_TIMESTAMP}`;
    });
    setLabelForAll(['#splash-build-note'], (el) => {
        el.hidden = false;
        el.textContent = COMPARE_NOTE;
    });
}

/**
 * Put the technical detail on the header's `playground` badge, which is the
 * only place it remains once the splash is dismissed.
 *
 * Always the complete text, never an append: the two halves arrive at
 * different moments, so a caller that added only its own half would leave the
 * tooltip reading differently depending on which ran last.
 *
 * The splash's own badge is deliberately left alone — everything this tooltip
 * says is already on screen beside it there.
 */
function setPlaygroundBadgeTooltip(lines: string[]): void {
    setLabelForAll(['header .version'], (el) => {
        el.title = lines.join('\n');
    });
}

/**
 * Say which resource profile this host applies, and what its ceilings are.
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

    // Now that the ceilings are known, the badge's tooltip can state the whole
    // thing — the same detail, in the same order, the splash shows on screen.
    setPlaygroundBadgeTooltip([...buildDetailLines(), '', text, ...limitLines]);

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
 * First-visit walkthrough: shows the Ajisai logo, then auto-reveals the build
 * and resource-limit detail spelled out in full (see setBuildVersionLabel() /
 * setHostProfileLabel() above, which fill it in) — everything the header
 * badges can only offer through a hover title, which is what a touch device
 * never gets. Dismissed by click, tap, or any key press — never a wait the
 * user is forced to sit through — and shown at most once per session so
 * reloading mid-session to retry a program never re-interrupts. Runs
 * independently of WASM/GUI startup: the splash is pure DOM and does not gate
 * `initializeApplication()`.
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
        splash.addEventListener('transitionend', (event) => {
            // The detail rows' own reveal transitions bubble up here too, and
            // one still in flight ends before this fade does — taking the
            // splash off screen mid-fade if it is allowed to answer for it.
            if (event.target !== splash) return;
            splash.remove();
        });
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
