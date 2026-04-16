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

function normalizeBranchName(rawBranchName: string, datePart: string): string {
  if (/^\d{8}\(.+\)$/.test(rawBranchName)) {
    return rawBranchName;
  }

  const cleaned = rawBranchName
    .replace(/^\/*/, '')
    .replace(/\/+$/, '')
    .replace(/\//g, '-')
    .replace(/\s+/g, '-')
    .replace(/\(+/g, '-')
    .replace(/\)+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');

  const detail = cleaned.length > 0 ? cleaned : 'update';
  return `${datePart}(${detail})`;
}

const now = new Date();
const datePart = formatDatePart(now);
const buildStamp = formatBuildStamp(now);
const gitBranchName = runGitCommand('git rev-parse --abbrev-ref HEAD') || 'detached-head';
const branchLabel = normalizeBranchName(gitBranchName, datePart);
const buildVersion = `${buildStamp} (${branchLabel})`;

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
