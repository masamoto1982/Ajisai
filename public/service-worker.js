const CACHE_NAME = 'ajisai-v202604111200'; // CI が自動更新

const urlsToCache = [
    './',
    './index.html',
    './manifest.json'
];

// ── ストア戦略の判定 ────────────────────────────────────────────
//
// [Network-first]  → HTML。常に最新を取り、オフライン時にキャッシュを返す
// [Cache-first]    → Vite がコンテンツハッシュを付けた /assets/ とWASM。
//                    URL が変わる = 常に新鮮なので永続キャッシュで問題なし
// [Stale-while-revalidate] → public/ の静的ファイル(CSS, 画像など)。
//                    キャッシュから即返しつつ、裏でフェッチして次回に反映

const isNavigation  = req => req.mode === 'navigate';
const isHashedAsset = url => url.includes('/assets/');
const isWasm        = url => url.endsWith('.wasm');

// ── ライフサイクル ─────────────────────────────────────────────

self.addEventListener('install', event => {
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then(cache => cache.addAll(urlsToCache))
            .then(() => self.skipWaiting())
            .catch(err => console.error('[SW] Install failed:', err))
    );
});

self.addEventListener('activate', event => {
    event.waitUntil(
        caches.keys()
            .then(names => Promise.all(
                names.map(name => name !== CACHE_NAME ? caches.delete(name) : null)
            ))
            .then(() => self.clients.claim())
    );
});

// ── フェッチハンドラ ───────────────────────────────────────────

self.addEventListener('fetch', event => {
    if (!event.request.url.startsWith('http')) return;

    const url = event.request.url;

    if (isNavigation(event.request) || url.includes('index.html')) {
        event.respondWith(networkFirst(event.request));
    } else if (isHashedAsset(url) || isWasm(url)) {
        event.respondWith(cacheFirst(event.request));
    } else {
        event.respondWith(staleWhileRevalidate(event.request));
    }
});

// ── 戦略実装 ──────────────────────────────────────────────────

async function networkFirst(request) {
    try {
        const response = await fetch(request);
        if (response.ok) {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, response.clone());
        }
        return response;
    } catch {
        const cached = await caches.match(request);
        return cached ?? new Response('Offline', { status: 503 });
    }
}

async function cacheFirst(request) {
    const cached = await caches.match(request);
    if (cached) return cached;
    try {
        const response = await fetch(request);
        if (response.ok) {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, response.clone());
        }
        return response;
    } catch {
        return new Response('Network error', { status: 503 });
    }
}

async function staleWhileRevalidate(request) {
    const cache  = await caches.open(CACHE_NAME);
    const cached = await cache.match(request);

    const fetchPromise = fetch(request).then(response => {
        if (response.ok) cache.put(request, response.clone());
        return response;
    }).catch(() => null);

    return cached ?? await fetchPromise;
}
