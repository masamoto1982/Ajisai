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

// Source files have no sections of their own, so a bare `§N.M` in one names
// some other document — and after the sweep above, the only document it can
// name is the retired specification. A `.md` under docs/dev/ is different: a
// memo numbers its own sections and cites its siblings' legitimately, so bare
// references are only rejected in source.
//
// Multi-part only. A bare single-digit `§7` in source is usually a real
// reference to a document that owns numbered sections — the formalization
// roadmap's `§1.2-(T)`, a work order's `§3`, or SKILL.md's own `§6`/`§9`,
// which `generate-skill-md.mjs` writes *into* the file it generates. Those
// are correct, so the pattern requires a dot and the allowance below carries
// the rest.
const SOURCE_EXTENSIONS = /\.(rs|ts|js|mjs|cjs|css|html)$/;
const BARE_CITATION = /§ ?[0-9]+\.[0-9]+(?:\.[0-9]+)*/g;

// A `§N.M` is fine when the sentence around it names the document that owns
// the numbering. Four lines of lookback, because a doc comment wraps and the
// name often sits a line or two above the reference.
const NAMES_ITS_DOCUMENT =
  /three-layer|documentation model|work order|work-order|roadmap|migration plan|handoff|proposal|methodology|Phase \d|SKILL|\.(md|json|html|mjs)\b/i;

// A passage *about* a stale citation is not itself one. Keyed by path so a new
// dangling citation in the same file is still caught: the passage must also
// name the retired numbering it is discussing. Matched against the same short
// context window the bare check uses, because the disclaimer and the sections
// it disclaims routinely land on different lines of one wrapped comment.
const DISCUSSES_RATHER_THAN_CITES = new Map([
  ['docs/dev/semantic-spine-migration-plan.md', /削減前|pre-reduction/],
  // This file's own header enumerates the phantom subsections as evidence.
  ['scripts/check-spec-citations.mjs', /never existed/],
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
  // Both patterns need the section sign; only the first needs the word.
  // Testing for 'SPEC' alone here made the bare-citation pass vacuous, which a
  // probe caught and a green run did not.
  if (!text.includes('§')) continue;
  const isSource = SOURCE_EXTENSIONS.test(file);
  const lines = text.split('\n');
  lines.forEach((line, index) => {
    const context = lines.slice(Math.max(0, index - 3), index + 2).join(' ');
    if (exempt && exempt.test(context)) return;
    for (const match of line.match(CITATION) ?? []) {
      findings.push(`${rel}:${index + 1}: ${match}`);
    }
    if (!isSource) return;
    if (NAMES_ITS_DOCUMENT.test(context)) return;
    for (const match of line.match(BARE_CITATION) ?? []) {
      // `SPEC §N.M` was already reported by the pass above.
      if (new RegExp(`SPEC\\s*${match.replace('§', '§')}`).test(line)) continue;
      findings.push(`${rel}:${index + 1}: ${match} (bare)`);
    }
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
