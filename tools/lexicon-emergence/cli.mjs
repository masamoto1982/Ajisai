#!/usr/bin/env node
// Lexicon-emergence experiment driver (docs/dev/lexicon-emergence-experiment-work-order-2026-09.md).
//
//   node cli.mjs prompt  <run> <condition> <gen> <agent>   subject prompt for one agent
//   node cli.mjs grade   <run> <condition> <gen>           grade every submission of a generation
//   node cli.mjs evolve  <run> <condition> <gen> <K>       choose generation gen+1's lexicon
//   node cli.mjs analyze <run>                             H1–H5 over every condition and generation
//
// A run lives under runs/<run>/<condition>/gen<g>/ as lexicon.json, submissions/*.json, graded/*.json.

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { connect } from './lib/ajisai.mjs';
import { analyze, renderReport } from './lib/analyze.mjs';
import { classify, collectWords, nextLexicon } from './lib/evolve.mjs';
import { grade, loadFamilies } from './lib/grade.mjs';
import { subjectPrompt } from './lib/prompt.mjs';

const ROOT = new URL('./', import.meta.url);
const TASKS = new URL('tasks/', ROOT);
const genDir = (run, condition, gen) => new URL(`runs/${run}/${condition}/gen${gen}/`, ROOT);

const readJson = (url) => JSON.parse(readFileSync(url, 'utf8'));
const writeJson = (url, value) => {
  mkdirSync(new URL('./', url), { recursive: true });
  writeFileSync(url, `${JSON.stringify(value, null, 2)}\n`);
};
const readAll = (dir) =>
  existsSync(dir) ? readdirSync(dir).filter((f) => f.endsWith('.json')).sort().map((f) => readJson(new URL(f, dir))) : [];
const lexiconOf = (dir) => (existsSync(new URL('lexicon.json', dir)) ? readJson(new URL('lexicon.json', dir)) : { entries: [] });

function* generations(run) {
  const base = new URL(`runs/${run}/`, ROOT);
  const conditions = readdirSync(base, { withFileTypes: true }).filter((e) => e.isDirectory()).map((e) => e.name);
  for (const condition of conditions) {
    const conditionDir = new URL(`${condition}/`, base);
    for (const g of readdirSync(conditionDir).filter((d) => /^gen\d+$/.test(d))) {
      const dir = new URL(`${g}/`, conditionDir);
      yield { condition, generation: Number(g.slice(3)), dir };
    }
  }
}

const [command, run, condition, gen, arg] = process.argv.slice(2);
const families = loadFamilies(TASKS);

if (command === 'prompt') {
  process.stdout.write(subjectPrompt({ run, condition, generation: Number(gen), agent: arg }, lexiconOf(genDir(run, condition, gen)), families));
} else if (command === 'grade') {
  const dir = genDir(run, condition, gen);
  const lexicon = lexiconOf(dir);
  const ajisai = await connect();
  for (const submission of readAll(new URL('submissions/', dir))) {
    const graded = await grade(ajisai, submission, lexicon, families);
    writeJson(new URL(`graded/${submission.agent}.json`, dir), graded);
    const correct = graded.results.filter((r) => r.correct).length;
    console.log(`${submission.agent}: ${correct}/${graded.results.length} correct`);
  }
  await ajisai.close();
} else if (command === 'evolve') {
  const dir = genDir(run, condition, gen);
  const lexicon = lexiconOf(dir);
  const graded = readAll(new URL('graded/', dir));
  const words = collectWords(graded, [lexicon]);
  const ajisai = await connect();
  const classes = await classify(ajisai, words);
  await ajisai.close();
  const next = nextLexicon(Number(gen) + 1, Number(arg), graded, words, classes);
  writeJson(new URL('lexicon.json', genDir(run, condition, Number(gen) + 1)), next);
  console.log(`gen${Number(gen) + 1} lexicon: ${next.entries.map((e) => e.name).join(' ')}`);
} else if (command === 'analyze') {
  const gens = [...generations(run)].map(({ condition: c, generation, dir }) => ({
    condition: c,
    generation,
    lexicon: lexiconOf(dir),
    graded: readAll(new URL('graded/', dir)),
  }));
  const ajisai = await connect();
  const report = await analyze(ajisai, gens, families);
  await ajisai.close();
  writeJson(new URL(`runs/${run}/report.json`, ROOT), report);
  writeFileSync(new URL(`runs/${run}/report.md`, ROOT), renderReport(report));
  console.log(renderReport(report));
} else {
  console.error('usage: node cli.mjs prompt|grade|evolve|analyze …  (see the header of cli.mjs)');
  process.exit(2);
}
