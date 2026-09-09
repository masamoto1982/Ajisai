// Reject citations to `SPEC §<number>` anywhere in the repository.
//
// That numbering belongs to `SPECIFICATION.md`, a Markdown specification of
// seventeen numbered sections deleted in 3f21400c ("Migrate SPECIFICATION and
// README from Markdown to HTML"). Every citation to it has been dangling ever
// since, and nothing noticed: 241 of them accumulated across 120 files, of
// which 34 named subsections (§8.7, §4.8, §12.3, §2.5, §7.1.1, §7.15, §14.4)
// that never existed in that document either.
//
// The current specification numbers nothing. It publishes stable `LANG.*`
// clause IDs precisely so a citation survives the document being reorganized —
// which is what LANG.CONFORMANCE.CHANGE asks of a derived surface. So a
// citation is either a `LANG.*` clause ID, or it is prose that stands on its
// own; a section number is neither.
//
// This gate is the part that makes the fix stick. An earlier pass (710e8168)
// repointed the stale §-citations it found in `docs/dev/` and added no gate,
// so the far larger population in `rust/src`, `rust/tests` and `src/` stayed
// invisible for another six weeks.

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const ROOT = process.cwd();
const SKIP_DIRS = new Set(['node_modules', 'target', '.git', 'dist', 'build']);
const EXTENSIONS = /\.(rs|ts|js|mjs|cjs|css|html|json|md|sh|yml|yaml)$/;
const CITATION = /SPEC\s*§\s*[0-9]+(?:\.[0-9]+)*/g;

// A passage *about* a stale citation is not itself one. Keyed by path so a new
// dangling citation in the same file is still caught: the line must also name
// the retired numbering it is discussing.
const DISCUSSES_RATHER_THAN_CITES = new Map([
  ['docs/dev/semantic-spine-migration-plan.md', /削減前|pre-reduction/],
]);

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    if (SKIP_DIRS.has(name)) continue;
    const full = join(dir, name);
    if (statSync(full).isDirectory()) yield* walk(full);
    else if (EXTENSIONS.test(name)) yield full;
  }
}

const findings = [];
for (const file of walk(ROOT)) {
  const rel = relative(ROOT, file);
  const exempt = DISCUSSES_RATHER_THAN_CITES.get(rel);
  let text;
  try {
    text = readFileSync(file, 'utf8');
  } catch {
    continue;
  }
  if (!text.includes('SPEC')) continue;
  text.split('\n').forEach((line, index) => {
    const matches = line.match(CITATION);
    if (!matches) return;
    if (exempt && exempt.test(line)) return;
    for (const match of matches) findings.push(`${rel}:${index + 1}: ${match}`);
  });
}

if (findings.length > 0) {
  console.error(
    `[spec-citations] ${findings.length} citation(s) to the retired numbered specification:`,
  );
  for (const finding of findings) console.error(`  ${finding}`);
  console.error(
    '\nSPECIFICATION.md was deleted in 3f21400c; its section numbers name nothing.',
  );
  console.error(
    'Cite the stable clause ID instead (LANG.VALUES.EXACT, LANG.FAILURE.ERROR, …),',
  );
  console.error(
    'or drop the reference and let the surrounding prose stand on its own.',
  );
  process.exitCode = 1;
} else {
  console.log('[spec-citations] no citations to the retired numbered specification.');
}
