#!/usr/bin/env node
// Lexicon-emergence experiment driver (docs/dev/lexicon-emergence-experiment-work-order-2026-09.md).
//
//   node cli.mjs prompt  <run> <condition> <gen> <agent>   subject prompt for one agent
//   node cli.mjs grade   <run> <condition> <gen>           grade every submission of a generation
//   node cli.mjs evolve  <run> <condition> <gen> <K>       choose generation gen+1's lexicon
//   node cli.mjs analyze <run>                             H1–H5 over every condition and generation
//
// Route B (the API harness, lib/harness.mjs) — needs ANTHROPIC_API_KEY and spends money:
//   node cli.mjs run   <run> <condition> <gen> <A,B,…> [options]   one generation's subjects
//   node cli.mjs pilot <run> [options]                             Phase 1's settings end to end
// Options: --model <id|large|medium|small> (default large), --effort <level> (default high),
//          --budget <usd> (stop before any request once spent), --max-turns <n> (default 60),
//          and for pilot --gens <n> (3) --agents <n> (3) --k <n> (8).
//
// A run lives under runs/<run>/<condition>/gen<g>/ as lexicon.json, submissions/*.json, graded/*.json,
// and, for route B, transcripts/*.json.

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { connect } from './lib/ajisai.mjs';
import { analyze, renderReport } from './lib/analyze.mjs';
import { classify, collectWords, nextLexicon } from './lib/evolve.mjs';
import { grade, loadFamilies } from './lib/grade.mjs';
import { anthropicClient, Budget, MODELS, runSubject, subjectSystem } from './lib/harness.mjs';
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

const argv = process.argv.slice(2);
const options = {};
const positional = [];
for (let i = 0; i < argv.length; i += 1) {
  if (argv[i].startsWith('--')) options[argv[i].slice(2)] = argv[(i += 1)];
  else positional.push(argv[i]);
}
const [command, run, condition, gen, arg] = positional;
const families = loadFamilies(TASKS);

async function gradeGeneration(ajisai, dir) {
  const lexicon = lexiconOf(dir);
  for (const submission of readAll(new URL('submissions/', dir))) {
    const graded = await grade(ajisai, submission, lexicon, families);
    writeJson(new URL(`graded/${submission.agent}.json`, dir), graded);
    const correct = graded.results.filter((r) => r.correct).length;
    console.log(`${submission.agent}: ${correct}/${graded.results.length} correct`);
  }
}

async function evolveGeneration(ajisai, runName, conditionName, generation, capacity) {
  const dir = genDir(runName, conditionName, generation);
  const lexicon = lexiconOf(dir);
  const graded = readAll(new URL('graded/', dir));
  const words = collectWords(graded, [lexicon]);
  const classes = await classify(ajisai, words);
  const next = nextLexicon(generation + 1, capacity, graded, words, classes);
  writeJson(new URL('lexicon.json', genDir(runName, conditionName, generation + 1)), next);
  console.log(`gen${generation + 1} lexicon: ${next.entries.map((e) => e.name).join(' ')}`);
}

/** Route B: run every named subject of one generation in parallel through the API harness. */
async function runGeneration(ajisai, harness, runName, conditionName, generation, agents) {
  const dir = genDir(runName, conditionName, generation);
  const lexicon = lexiconOf(dir);
  const outcomes = await Promise.all(
    agents.map(async (agent) => {
      const meta = { run: runName, condition: conditionName, generation, agent };
      const { submission, record } = await runSubject({
        ...harness,
        ajisai,
        prompt: subjectPrompt(meta, lexicon, families, 'harness'),
        meta,
      });
      writeJson(new URL(`transcripts/${agent}.json`, dir), record);
      if (submission) writeJson(new URL(`submissions/${agent}.json`, dir), submission);
      console.log(`${agent}: ${record.ended} after ${record.turns} request(s), $${record.costUsd.toFixed(3)}`);
      return record;
    }),
  );
  return outcomes;
}

async function harnessSettings() {
  const model = MODELS[options.model ?? 'large'] ?? options.model;
  return {
    client: await anthropicClient(),
    model,
    effort: options.effort ?? 'high',
    system: subjectSystem(new URL('../../SKILL.md', ROOT)),
    budget: new Budget(options.budget != null ? Number(options.budget) : null),
    maxTurns: Number(options['max-turns'] ?? 60),
  };
}

const letters = (n) => [...'ABCDEFGHIJ'].slice(0, n);

if (command === 'prompt') {
  process.stdout.write(subjectPrompt({ run, condition, generation: Number(gen), agent: arg }, lexiconOf(genDir(run, condition, gen)), families));
} else if (command === 'grade') {
  const ajisai = await connect();
  await gradeGeneration(ajisai, genDir(run, condition, gen));
  await ajisai.close();
} else if (command === 'evolve') {
  const ajisai = await connect();
  await evolveGeneration(ajisai, run, condition, Number(gen), Number(arg));
  await ajisai.close();
} else if (command === 'run') {
  const harness = await harnessSettings();
  const ajisai = await connect();
  try {
    await runGeneration(ajisai, harness, run, condition, Number(gen), arg.split(','));
  } finally {
    await ajisai.close();
  }
  console.log(`spent $${harness.budget.spentUsd.toFixed(3)}`);
} else if (command === 'pilot') {
  // Phase 1's design (work order §6): C-bottleneck over `gens` generations of
  // `agents` subjects with capacity K, and C-solo as one generation.
  const gens = Number(options.gens ?? 3);
  const agents = Number(options.agents ?? 3);
  const capacity = Number(options.k ?? 8);
  const harness = await harnessSettings();
  const ajisai = await connect();
  try {
    for (let g = 0; g < gens; g += 1) {
      await runGeneration(ajisai, harness, run, 'bottleneck', g, letters(agents).map((l) => `G${g}${l}`));
      await gradeGeneration(ajisai, genDir(run, 'bottleneck', g));
      if (g + 1 < gens) await evolveGeneration(ajisai, run, 'bottleneck', g, capacity);
    }
    await runGeneration(ajisai, harness, run, 'solo', 0, letters(agents).map((l) => `S0${l}`));
    await gradeGeneration(ajisai, genDir(run, 'solo', 0));
  } finally {
    await ajisai.close();
  }
  writeJson(new URL(`runs/${run}/harness.json`, ROOT), {
    model: harness.model,
    effort: harness.effort,
    maxTurns: harness.maxTurns,
    spentUsd: harness.budget.spentUsd,
    design: { gens, agents, capacity },
  });
  console.log(`spent $${harness.budget.spentUsd.toFixed(3)}; now: node cli.mjs analyze ${run}`);
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
  console.error('usage: node cli.mjs prompt|grade|evolve|analyze|run|pilot …  (see the header of cli.mjs)');
  process.exit(2);
}
