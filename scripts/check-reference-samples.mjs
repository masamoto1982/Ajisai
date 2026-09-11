// LANG.AUTHORITY.PRESENT — the worked-example half of the clause.
//
// The Reference (public/docs/ja/index.html) shows 80-odd sample blocks, each a
// program next to the value it is said to produce, each with a "Playgroundで開く"
// button that loads it ready to run. A reader learns the language by running
// them, so an example that does not produce its stated value is not a cosmetic
// defect: the diagnostic is precise, the Reference is confident, and the reader
// concludes they mistyped it.
//
// Two such examples shipped. The `COND` clause-form and paired-form examples
// both dispatched on `[ 0 ]` / `[ -5 ]` and compared against `[ 0 ]`, and
// comparison Words lift element-wise, so each guard answered `[ FALSE ]` /
// `[ TRUE ]` — a one-element Vector, not the bare Boolean a guard must return
// (LANG.VALUES.TRUTH, LANG.VALUES.DISJOINT). Both raised `nonTruthGuard`
// instead of the `'zero'` / `'negative'` printed beside them. They were the
// language's two introductory examples of conditional dispatch.
//
// Every other reading surface that carries runnable code is already gated —
// SKILL.md against its own corpus, the MCP evaluation corpus against the CLI —
// and the Reference was the one that was not. This gate runs each sample
// through the same `ajisai` binary a reader's Playground runs and compares the
// result against the expectation printed beside it, so the Reference cannot
// drift from the language again without CI saying so.
//
// The three expectation notations are distinguished by their own markup,
// not by guessing at the text:
//
//   <td><code>VALUE</code></td>   under <th>期待値</th>         → final stack
//   <td><code>TEXT</code></td>    under <th>期待される出力</th>  → standard output
//   <td>error</td>                (bare, no <code>)            → any error
//   <td>—</td>                    (bare em dash)                → empty stack
//
// A row that is none of these is reported rather than skipped: a sample with no
// checkable expectation is exactly the hole this gate exists to close.

import { readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const SURFACE = 'public/docs/ja/index.html';
const BINARY = 'rust/target/debug/ajisai';

const STACK_HEADER = '期待値';
const OUTPUT_HEADER = '期待される出力';
const ANY_ERROR = 'error';
const EMPTY_STACK = '—';

const html = readFileSync(SURFACE, 'utf8');

// `<br>` is a line break in the sample source, and line breaks are significant
// in Ajisai (a definition body's own level separates statements), so it has to
// become a newline rather than be stripped with the rest of the markup.
const cellText = (cell) =>
  cell
    .replace(/<br\s*\/?>/g, '\n')
    .replace(/<[^>]+>/g, '')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, '&');

// Whitespace is insignificant between the printed form and the CLI's, and only
// there: this normalization is for comparing two renderings of one value, never
// for deciding what a program means.
const normalize = (text) => text.replace(/\s+/g, '');

const samples = [];
const errors = [];

const blockPattern = /<div class="sample">([\s\S]*?)<div class="sample-actions">/g;
const rowPattern =
  /<tr>\s*<td><code>([\s\S]*?)<\/code><\/td>\s*<td>([\s\S]*?)<\/td>\s*<td class="notes">[\s\S]*?<\/td>\s*<\/tr>/g;

for (const block of html.matchAll(blockPattern)) {
  const body = block[1];
  const line = html.slice(0, block.index).split('\n').length;
  const header = /<thead>\s*<tr>([\s\S]*?)<\/tr>/.exec(body);
  const headings = header ? [...header[1].matchAll(/<th>([\s\S]*?)<\/th>/g)].map((m) => cellText(m[1]).trim()) : [];
  const checksOutput = headings.includes(OUTPUT_HEADER);
  if (!checksOutput && !headings.includes(STACK_HEADER)) {
    errors.push(`line ${line}: sample block heads its expectation column "${headings[1] ?? '(none)'}", which is neither "${STACK_HEADER}" nor "${OUTPUT_HEADER}"`);
    continue;
  }
  for (const row of body.matchAll(rowPattern)) {
    const rawExpectation = row[2].trim();
    const expectation = cellText(rawExpectation).trim();
    // A `<code>`-wrapped expectation is a value; a bare one is a notation.
    const quoted = /^<code>[\s\S]*<\/code>$/.test(rawExpectation);
    let kind;
    if (quoted) kind = checksOutput ? 'output' : 'stack';
    else if (expectation === ANY_ERROR) kind = 'error';
    else if (expectation === EMPTY_STACK) kind = 'empty';
    else {
      errors.push(`line ${line}: sample expects "${expectation}", which is not a value, "${ANY_ERROR}" or "${EMPTY_STACK}"`);
      continue;
    }
    samples.push({ line, code: cellText(row[1]), expectation, kind });
  }
}

if (samples.length === 0) {
  console.error(`[reference-samples] found no samples in ${SURFACE} — the extraction pattern no longer matches the page.`);
  process.exit(1);
}

const workdir = mkdtempSync(join(tmpdir(), 'ajisai-reference-samples-'));
const program = join(workdir, 'sample.ajisai');

try {
  for (const sample of samples) {
    writeFileSync(program, `${sample.code}\n`, 'utf8');
    const run = spawnSync(BINARY, ['run', program], { encoding: 'utf8', timeout: 60_000 });
    if (run.error) {
      console.error(`[reference-samples] could not run ${BINARY}: ${run.error.message}`);
      console.error('[reference-samples] Build it first: cargo build --manifest-path rust/Cargo.toml --bin ajisai');
      process.exit(1);
    }
    const combined = `${run.stdout ?? ''}${run.stderr ?? ''}`;
    const lines = combined.split('\n');
    const stackLine = lines.find((l) => l.startsWith('stack:'));
    const stack = stackLine ? stackLine.slice('stack:'.length).trim() : null;
    // Everything the program printed, which is every line before the stack
    // report the CLI appends.
    const output = lines.slice(0, stackLine ? lines.indexOf(stackLine) : lines.length).join('\n').trim();
    const failed = stack === null;
    const at = `line ${sample.line}: ${sample.code.replace(/\n/g, ' ⏎ ')}`;

    if (sample.kind === 'error') {
      if (!failed) errors.push(`${at}\n  expected an error, got stack ${stack}`);
      continue;
    }
    if (failed) {
      errors.push(`${at}\n  expected ${sample.kind === 'output' ? 'output' : 'stack'} ${sample.expectation}, got ${lines[0]}`);
      continue;
    }
    if (sample.kind === 'empty') {
      if (stack !== '(empty)') errors.push(`${at}\n  expected an empty stack, got ${stack}`);
      continue;
    }
    const actual = sample.kind === 'output' ? output : stack;
    if (normalize(actual) !== normalize(sample.expectation)) {
      errors.push(`${at}\n  expected ${sample.kind} ${sample.expectation}, got ${actual}`);
    }
  }
} finally {
  rmSync(workdir, { recursive: true, force: true });
}

if (errors.length) {
  for (const error of errors) console.error(`[reference-samples] ${error}`);
  console.error(`[reference-samples] ${errors.length} of ${samples.length} Reference samples do not produce the value printed beside them.`);
  console.error('[reference-samples] A sample carries a "Playgroundで開く" button, so a reader runs it: fix the sample, or fix the expectation.');
  process.exitCode = 1;
} else {
  console.log(`[reference-samples] all ${samples.length} samples in ${SURFACE} produce the value printed beside them.`);
}
