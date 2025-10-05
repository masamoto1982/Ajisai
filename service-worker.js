const CACHE_NAME = 'ajisai-v202510041830';
const urlsToCache = [
  '/',
  '/index.html',
  '/reference.html',
  '/style.css',
  '/manifest.json'
];

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

  event.respondWith(
    caches.match(event.request)
      .then(response => {
        if (response) {
          console.log('[SW] Serving from cache:', event.request.url);
          return response;
        }
        
        console.log('[SW] Fetching:', event.request.url);
        return fetch(event.request).then(response => {
          // 有効なレスポンスでない場合はキャッシュしない
          if (!response || response.status !== 200 || response.type !== 'basic') {
            return response;
          }
          
          // WASMファイルとJSファイルは動的にキャッシュ
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
        // オフライン時のフォールバック
        if (event.request.mode === 'navigate') {
          return caches.match('/index.html');
        }
      })
  );
});
