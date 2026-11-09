// LANG.AUTHORITY.IDENTITY — the counts half of the clause.
//
// The Specification states how many Words the vocabulary has, and says
// `docs/word-manifest.json` is the count of record. Prose that restates a
// count is a copy of that record, and nothing regenerated the copies: when
// BOTH retired, the manifest went to 77 and 49 while the Specification kept
// saying 78 and 50, and the Japanese Reference kept one "78" beside three
// "77"s. Every other gate passed, because every other gate reads the registry
// and none reads the prose.
//
// This gate reads the prose — every tracked text file, not a list of surfaces,
// because the first version read seven named files, while a CI workflow
// comment's "65語" was stale long after the semantics table covered 76. The
// rule (which phrasings, where a bare "N Words" counts, what is history) is in
// scripts/lib/vocabulary-counts.mjs and its test.
//
// Generated surfaces (SKILL.md, docs/word-reference.md, the marked word index
// in the Reference) are read too; their generators take the counts from the
// manifest, so they agree whenever their own --check gates pass.

import { execFileSync } from 'node:child_process';
import { readJson, readText, repoRoot, reporter } from './lib/common.mjs';
import { scanText, SELF, TEXT_FILE } from './lib/vocabulary-counts.mjs';

const counts = readJson('docs/word-manifest.json').counts;
const RECORD = {
  canonical: counts.canonicalWords,
  kernel: counts.semanticKernelWords,
  standard: counts.standardWords,
};

const files = execFileSync('git', ['ls-files', '-z'], { cwd: repoRoot, encoding: 'utf8' })
  .split('\0')
  .filter((path) => TEXT_FILE.test(path) && !SELF.includes(path));

const report = reporter('vocabulary-counts');
let stated = 0;

for (const path of files) {
  const result = scanText(path, readText(path), RECORD);
  stated += result.stated;
  for (const { line, written, expected, which, phrase } of result.mismatches) {
    const record = which === 'any'
      ? `no count of record is ${written} (${expected.join(', ')})`
      : `docs/word-manifest.json records ${expected}`;
    report.fail(`${path}:${line} states ${written} (${which}) in "${phrase}", but ${record}`);
  }
}

if (stated === 0) report.fail('no count was found in any file; the patterns have drifted from the prose');
if (report.failed) {
  console.error(
    '[vocabulary-counts] Restate the count from docs/word-manifest.json (`counts`), the count of record, or drop the number.'
    + ' A count that was true in the past belongs on a line that says so ("当時", "once"); a number of Words that is not'
    + ' about the vocabulary goes in EXEMPT in scripts/lib/vocabulary-counts.mjs with its reason.',
  );
}
report.done(
  `${stated} stated counts in ${files.length} tracked files agree with the manifest (${RECORD.canonical} Words: ${RECORD.kernel} Kernel, ${RECORD.standard} Standard).`,
);
