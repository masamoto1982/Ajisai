// Opt-in registrar for the cross-origin-isolation service worker
// (implicit-parallelism roadmap Phase 5).
//
// On a static host such as GitHub Pages the COOP/COEP response headers cannot
// be set, so `crossOriginIsolated` is false and SharedArrayBuffer threading is
// unavailable. Registering `public/coi-serviceworker.js` makes the worker
// re-emit responses with those headers; the page must then reload once so the
// now-controlling worker can supply isolation on the next load.
//
// This is deliberately NOT invoked at startup yet: it ships inert so the live
// deployment is unchanged until a threaded wasm build exists to benefit from
// it. To turn it on, call `registerCrossOriginIsolation()` from the entry
// bootstrap (see docs/dev/browser-parallelism-phase5-rollout.md). The reload is
// guarded by a sessionStorage flag so it happens at most once per session,
// never looping.

const RELOAD_GUARD_KEY = 'ajisai:coi-reloaded';

export interface RegisterCoiOptions {
    /** Path to the service worker script. Defaults to the deployed location. */
    scriptUrl?: string;
    /** Reload the page once after the worker takes control. Default true. */
    reloadOnce?: boolean;
}

export interface RegisterCoiResult {
    /** Already isolated; nothing to do. */
    alreadyIsolated: boolean;
    /** Service workers are unavailable (e.g. insecure context, Tauri). */
    unsupported: boolean;
    /** The worker was registered this call. */
    registered: boolean;
    /** A one-time reload was scheduled to gain isolation. */
    willReload: boolean;
}

/**
 * Register the COI service worker when the page is not yet cross-origin
 * isolated. Safe to call unconditionally: it no-ops when already isolated or
 * when service workers are unavailable, and reloads at most once per session.
 */
export async function registerCrossOriginIsolation(
    options: RegisterCoiOptions = {},
): Promise<RegisterCoiResult> {
    const scriptUrl = options.scriptUrl ?? 'coi-serviceworker.js';
    const reloadOnce = options.reloadOnce ?? true;
    const result: RegisterCoiResult = {
        alreadyIsolated: false,
        unsupported: false,
        registered: false,
        willReload: false,
    };

    if (typeof globalThis === 'undefined') {
        result.unsupported = true;
        return result;
    }

    const isolated = (globalThis as { crossOriginIsolated?: boolean }).crossOriginIsolated === true;
    if (isolated) {
        result.alreadyIsolated = true;
        return result;
    }

    const nav = (globalThis as { navigator?: Navigator }).navigator;
    if (!nav || !('serviceWorker' in nav)) {
        result.unsupported = true;
        return result;
    }

    try {
        const registration = await nav.serviceWorker.register(scriptUrl);
        result.registered = true;

        // If a worker is already controlling this page but we are still not
        // isolated, one reload lets the controller add the headers. Guard
        // against reload loops with a per-session flag.
        const controller = nav.serviceWorker.controller;
        const store = (globalThis as { sessionStorage?: Storage }).sessionStorage;
        const alreadyReloaded = store?.getItem(RELOAD_GUARD_KEY) === '1';

        if (reloadOnce && (controller || registration.active) && !alreadyReloaded) {
            store?.setItem(RELOAD_GUARD_KEY, '1');
            result.willReload = true;
            const loc = (globalThis as { location?: Location }).location;
            loc?.reload();
        }
    } catch (error) {
        console.error('[coi] service worker registration failed', error);
        result.unsupported = true;
    }

    return result;
}
