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

function normalizeChangeNote(note: string): string {
  const cleaned = note
    .replace(/[()（）]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

  return cleaned.length > 0 ? cleaned : 'update';
}

function extractChangeFromBranchName(branchName: string): string {
  const matched = branchName.match(/^\d{8}[（(](.+)[）)]$/);
  if (!matched) return '';
  return matched[1].trim();
}

function extractChangeFromCommitSubject(subject: string): string {
  if (subject.startsWith('Merge pull request')) {
    return '';
  }
  return subject;
}

const envChangeNote = process.env.AJISAI_CHANGE_NOTE ?? '';
const gitBranchName = runGitCommand('git rev-parse --abbrev-ref HEAD');
const branchChangeNote = extractChangeFromBranchName(gitBranchName);
const gitSubject = runGitCommand('git log -1 --pretty=%s');
const commitChangeNote = extractChangeFromCommitSubject(gitSubject);
const changeNote = normalizeChangeNote(envChangeNote || branchChangeNote || commitChangeNote || 'update');

export default defineConfig({
  root: '.',
  base: './',
  define: {
    __AJISAI_CHANGE_NOTE__: JSON.stringify(changeNote)
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
