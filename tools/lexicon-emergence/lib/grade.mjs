// Grading: a solution is correct when, for every input of its family, the
// engine leaves the same final stack as the family's reference solution.
// The judge is always the engine (work order §0.4), never a model.

import { readFileSync } from 'node:fs';
import { stackOf } from './ajisai.mjs';
import { closure, expand, norm, prelude, referencedWords, tokenize } from './source.mjs';

export function loadFamilies(dir) {
  return ['stats', 'text'].map((family) => JSON.parse(readFileSync(new URL(`${family}.json`, dir), 'utf8')));
}

/** Every definition a submission can call: the lexicon it was given, then its own. */
export function definitionsOf(submission, lexicon) {
  const byName = new Map();
  for (const entry of lexicon?.entries ?? []) byName.set(norm(entry.name), entry);
  for (const definition of submission.definitions ?? []) byName.set(norm(definition.name), definition);
  return byName;
}

const sameStack = (a, b) => a !== null && b !== null && JSON.stringify(a) === JSON.stringify(b);

export async function grade(ajisai, submission, lexicon, families) {
  const byName = definitionsOf(submission, lexicon);
  const known = new Set(byName.keys());
  const bodies = new Map([...byName].map(([name, entry]) => [name, entry.body]));
  const results = [];

  for (const family of families) {
    for (const task of family.tasks) {
      const solution = submission.solutions?.[task.id];
      if (typeof solution !== 'string') {
        results.push({ task: task.id, submitted: false, correct: false });
        continue;
      }
      const used = [...closure(referencedWords(solution, known), byName)].map((entry) => norm(entry.name));
      const setup = prelude(closure(used, byName));
      const expanded = expand(solution, bodies);
      const perInput = [];
      for (const input of family.inputs) {
        const expected = stackOf(await ajisai.compute(`${input} ${task.reference}`));
        const run = await ajisai.compute(`${setup}\n${input} ${solution}`);
        const got = stackOf(run);
        // H5: the same solution with every user Word expanded away, run in Core alone.
        const coreOnly = stackOf(await ajisai.compute(`${input} ${expanded}`));
        perInput.push({
          input,
          ok: sameStack(got, expected),
          got: got ?? { error: run.message ?? run.status },
          expected,
          coreEquivalent: sameStack(got, coreOnly),
        });
      }
      results.push({
        task: task.id,
        submitted: true,
        solution,
        correct: perInput.every((r) => r.ok),
        coreEquivalent: perInput.every((r) => r.coreEquivalent),
        tokens: tokenize(solution).length,
        expandedTokens: tokenize(expanded).length,
        uses: used,
        perInput,
      });
    }
  }
  return {
    agent: submission.agent,
    generation: submission.generation,
    condition: submission.condition,
    definitions: submission.definitions ?? [],
    lexiconNames: (lexicon?.entries ?? []).map((entry) => norm(entry.name)),
    results,
  };
}
