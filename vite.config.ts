import { defineConfig } from 'vite';

const isTauri = !!process.env.TAURI_ENV_TARGET_TRIPLE;

function formatDatePart(date: Date): string {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, '0');
  const day = `${date.getDate()}`.padStart(2, '0');
  return `${year}${month}${day}`;
}

function formatBuildStamp(date: Date): string {
  const datePart = formatDatePart(date);
  const hours = `${date.getHours()}`.padStart(2, '0');
  const minutes = `${date.getMinutes()}`.padStart(2, '0');
  return `${datePart}${hours}${minutes}`;
}

const buildVersion = formatBuildStamp(new Date());

export default defineConfig({
  root: '.',
  base: './',
  define: {
    __AJISAI_BUILD_VERSION__: JSON.stringify(buildVersion)
  },
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
