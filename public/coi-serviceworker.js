/*
 * coi-serviceworker — cross-origin isolation via a service worker.
 *
 * GitHub Pages (and other static hosts) cannot send the
 * `Cross-Origin-Opener-Policy` / `Cross-Origin-Embedder-Policy` response
 * headers that `crossOriginIsolated` — and therefore SharedArrayBuffer-backed
 * wasm threading — requires. This worker intercepts same-origin navigations
 * and re-emits each response with those headers added, so the page becomes
 * cross-origin isolated without any server configuration.
 *
 * Minimal, auditable implementation of the well-known technique popularised by
 * Guido Zuidhof's `coi-serviceworker` (MIT). It is intentionally header-only:
 * it adds isolation headers and a permissive `Cross-Origin-Resource-Policy`
 * but otherwise passes responses through untouched.
 *
 * Activation is opt-in: nothing registers this worker unless
 * `registerCrossOriginIsolation()` (src/platform/register-cross-origin-
 * isolation.ts) is called. See docs/dev/browser-parallelism-phase5-rollout.md.
 */

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));

self.addEventListener('message', (event) => {
    if (event.data && event.data.type === 'deregister') {
        self.registration
            .unregister()
            .then(() => self.clients.matchAll())
            .then((clients) => clients.forEach((client) => client.navigate(client.url)));
    }
});

self.addEventListener('fetch', (event) => {
    const request = event.request;

    // `only-if-cached` is only valid for same-origin `same-origin`-mode
    // requests; forwarding others throws, so leave them to the browser.
    if (request.cache === 'only-if-cached' && request.mode !== 'same-origin') {
        return;
    }

    event.respondWith(
        fetch(request)
            .then((response) => {
                // Opaque / network-error responses have no readable headers.
                if (response.status === 0) {
                    return response;
                }
                const headers = new Headers(response.headers);
                headers.set('Cross-Origin-Opener-Policy', 'same-origin');
                headers.set('Cross-Origin-Embedder-Policy', 'require-corp');
                headers.set('Cross-Origin-Resource-Policy', 'cross-origin');
                return new Response(response.body, {
                    status: response.status,
                    statusText: response.statusText,
                    headers,
                });
            })
            .catch((error) => {
                console.error('[coi-serviceworker] fetch failed', error);
                throw error;
            }),
    );
});
