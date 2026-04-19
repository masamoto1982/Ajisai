import { execSync } from 'node:child_process';
import { defineConfig } from 'vite';

const isTauri = !!process.env.TAURI_ENV_TARGET_TRIPLE;
const target = process.env.AJISAI_BUILD_TARGET === 'tauri' ? 'tauri' : 'web';

const GENERIC_BRANCH_NAMES = new Set([
  'main',
  'master',
  'develop',
  'development',
  'dev',
  'work',
  'trunk'
]);

function runGitCommand(command: string): string {
  try {
    return execSync(command, { stdio: ['ignore', 'pipe', 'ignore'] }).toString().trim();
  } catch {
    return '';
  }
}

function toKebabCase(text: string): string {
  const normalized = text
    .replace(/[()（）]/g, ' ')
    .replace(/[_\s]+/g, '-')
    .replace(/[^a-zA-Z0-9-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
    .toLowerCase();

  return normalized.length > 0 ? normalized : 'update';
}

function simplifyBranchLikeToken(token: string): string {
  const lastSegment = token.split('/').pop() ?? token;
  const withoutAjisaiTail = lastSegment.replace(/-in-ajisai.*$/i, '');
  const withoutRandomSuffix = withoutAjisaiTail.replace(/-[a-z0-9]{5,}$/i, '');
  return toKebabCase(withoutRandomSuffix);
}

function extractChangeFromBranchName(branchName: string): string {
  const dated = branchName.match(/^\d{8}[（(](.+)[）)]$/);
  if (dated) return toKebabCase(dated[1]);

  const simplified = simplifyBranchLikeToken(branchName);
  if (GENERIC_BRANCH_NAMES.has(simplified)) {
    return '';
  }

  return simplified;
}

function extractChangeFromCommitSubject(subject: string): string {
  const merge = subject.match(/^Merge pull request #\d+ from .+\/(.+)$/);
  if (merge) {
    return simplifyBranchLikeToken(merge[1]);
  }
  return toKebabCase(subject);
}

function buildTimestampLabel(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = `${now.getMonth() + 1}`.padStart(2, '0');
  const day = `${now.getDate()}`.padStart(2, '0');
  const hours = `${now.getHours()}`.padStart(2, '0');
  const minutes = `${now.getMinutes()}`.padStart(2, '0');
  return `${year}${month}${day}${hours}${minutes}`;
}

const envChangeNote = process.env.AJISAI_CHANGE_NOTE ?? '';
const gitBranchName = runGitCommand('git rev-parse --abbrev-ref HEAD');
const branchChangeNote = extractChangeFromBranchName(gitBranchName);
const gitSubject = runGitCommand('git log -1 --pretty=%s');
const commitChangeNote = extractChangeFromCommitSubject(gitSubject);
const changeNote = toKebabCase(envChangeNote || branchChangeNote || commitChangeNote || 'update');

export default defineConfig({
  root: '.',
  base: target === 'tauri' ? '/' : './',
  define: {
    __AJISAI_CHANGE_NOTE__: JSON.stringify(changeNote),
    __AJISAI_TARGET__: JSON.stringify(target),
    __AJISAI_BUILD_TIMESTAMP__: JSON.stringify(buildTimestampLabel())
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
