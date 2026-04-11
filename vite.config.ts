

import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const projectRoot = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: projectRoot,
  base: './',
  server: {
    port: 3000,
    open: true
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    sourcemap: true,
    rollupOptions: {
      input: {
        main: path.resolve(projectRoot, 'index.html'),
        reference: path.resolve(projectRoot, 'language-reference-playground.html')
      }
    }
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
  publicDir: path.resolve(projectRoot, 'public')
});
