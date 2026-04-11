

import { defineConfig } from 'vite';

// TAURI_ENV_TARGET_TRIPLE is set by Tauri CLI during `tauri dev`
const isTauri = !!process.env.TAURI_ENV_TARGET_TRIPLE;

export default defineConfig({
  root: '.',
  base: './',
  server: {
    port: 3000,
    open: !isTauri,
    strictPort: true
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    sourcemap: true
  },
  optimizeDeps: {
    exclude: ['./js/pkg/ajisai_core.js']
  },
  resolve: {
    alias: {
      '@': '/js'
    }
  },
  worker: {
    format: 'es'
  },
  publicDir: 'public'
});
