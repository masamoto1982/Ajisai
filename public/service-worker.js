const CACHE_NAME = 'ajisai-v202512301000';
const urlsToCache = [
  './',
  './index.html',
  './style.css',
  './manifest.json',
  './ajisai-config.js',
  './ajisai-theme.js'
];

// ネットワーク優先で取得すべきファイル（設定・テーマ関連）
const networkFirstPatterns = [
  'ajisai-config.js',
  'ajisai-theme.js',
  'config.js',
  'script.js',
  'style.css',
  'index.html'
];

function shouldUseNetworkFirst(url) {
  return networkFirstPatterns.some(pattern => url.includes(pattern));
}

self.addEventListener('install', (event) => {
  console.log('[SW] Installing...');
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then(cache => {
        console.log('[SW] Caching app shell');
        return cache.addAll(urlsToCache);
      })
      .then(() => {
        console.log('[SW] Skip waiting');
        return self.skipWaiting();
      })
      .catch(err => {
        console.error('[SW] Cache installation failed:', err);
      })
  );
});

self.addEventListener('activate', (event) => {
  console.log('[SW] Activating...');
  event.waitUntil(
    caches.keys().then(cacheNames => {
      return Promise.all(
        cacheNames.map(cacheName => {
          if (cacheName !== CACHE_NAME) {
            console.log('[SW] Deleting old cache:', cacheName);
            return caches.delete(cacheName);
          }
        })
      );
    }).then(() => {
      console.log('[SW] Claiming clients');
      return self.clients.claim();
    })
  );
});

self.addEventListener('fetch', (event) => {
  // Chrome拡張機能などのリクエストは無視
  if (!event.request.url.startsWith('http')) {
    return;
  }

  // テーマ関連ファイルはネットワーク優先
  if (shouldUseNetworkFirst(event.request.url)) {
    event.respondWith(
      fetch(event.request)
        .then(response => {
          // 有効なレスポンスをキャッシュに保存
          if (response && response.status === 200) {
            const responseToCache = response.clone();
            caches.open(CACHE_NAME).then(cache => {
              cache.put(event.request, responseToCache);
            });
          }
          return response;
        })
        .catch(() => {
          // ネットワーク失敗時はキャッシュから
          console.log('[SW] Network failed, serving from cache:', event.request.url);
          return caches.match(event.request);
        })
    );
    return;
  }

  // その他のファイルはキャッシュ優先
  event.respondWith(
    caches.match(event.request)
      .then(response => {
        if (response) {
          console.log('[SW] Serving from cache:', event.request.url);
          return response;
        }

        console.log('[SW] Fetching:', event.request.url);
        return fetch(event.request).then(response => {
          if (!response || response.status !== 200 || response.type !== 'basic') {
            return response;
          }

          if (event.request.url.includes('.wasm') ||
              event.request.url.includes('.js') ||
              event.request.url.includes('.css')) {
            const responseToCache = response.clone();
            caches.open(CACHE_NAME).then(cache => {
              cache.put(event.request, responseToCache);
            });
          }

          return response;
        });
      })
      .catch(err => {
        console.error('[SW] Fetch failed:', err);
        if (event.request.mode === 'navigate') {
          return caches.match('./index.html');
        }
      })
  );
});
