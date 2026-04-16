import { execSync } from 'node:child_process';
import { defineConfig } from 'vite';

const isTauri = !!process.env.TAURI_ENV_TARGET_TRIPLE;

function runGitCommand(command: string): string {
  try {
    return execSync(command, { stdio: ['ignore', 'pipe', 'ignore'] }).toString().trim();
  } catch {
    return '';
  }
}

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

function normalizeChangeNote(note: string): string {
  const cleaned = note
    .replace(/[()]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

  return cleaned.length > 0 ? cleaned : 'update';
}

const buildStamp = formatBuildStamp(new Date());
const envChangeNote = process.env.AJISAI_CHANGE_NOTE ?? '';
const gitSubject = runGitCommand('git log -1 --pretty=%s');
const changeNote = normalizeChangeNote(envChangeNote || gitSubject || 'update');
const buildVersion = `${buildStamp}(${changeNote})`;

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
