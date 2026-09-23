// H1–H5 (work order §3) over a whole run. Every number here is counted from
// graded files; nothing is estimated.

import { readFileSync } from 'node:fs';
import { classify, collectWords } from './evolve.mjs';
import { norm, referencedWords, tokenize } from './source.mjs';

const CORE = JSON.parse(readFileSync(new URL('../../mcp-server/assets/words.json', import.meta.url), 'utf8'));
const CORE_NAMES = new Set(CORE.entries.map((w) => w.name));
const ALIASES = new Map(CORE.entries.flatMap((w) => (w.aliases ?? []).map((a) => [a, w.name])));

const coreWordOf = (token) => {
  const t = norm(token);
  return CORE_NAMES.has(t) ? t : ALIASES.get(token) ?? null;
};

export async function analyze(ajisai, gens, families) {
  const graded = gens.flatMap((g) => g.graded.map((x) => ({ ...x, condition: g.condition })));
  const words = collectWords(graded, gens.map((g) => g.lexicon));
  const classes = await classify(ajisai, words);
  const known = new Set(words.keys());

  // Per generation: accuracy, solution length, compression against the expanded form, H5.
  const perGeneration = gens
    .map((g) => {
      const results = g.graded.flatMap((x) => x.results.filter((r) => r.submitted));
      const correct = results.filter((r) => r.correct);
      const sum = (rs, k) => rs.reduce((n, r) => n + r[k], 0);
      return {
        condition: g.condition,
        generation: g.generation,
        agents: g.graded.length,
        lexiconSize: g.lexicon.entries.length,
        accuracy: `${correct.length}/${g.graded.length * families.reduce((n, f) => n + f.tasks.length, 0)}`,
        meanTokens: correct.length ? +(sum(correct, 'tokens') / correct.length).toFixed(1) : null,
        compression: correct.length ? +(sum(correct, 'tokens') / sum(correct, 'expandedTokens')).toFixed(3) : null,
        coreEquivalent: `${correct.filter((r) => r.coreEquivalent).length}/${correct.length}`,
      };
    })
    .sort((a, b) => a.condition.localeCompare(b.condition) || a.generation - b.generation);

  // H1: classes defined independently by two or more agents. An agent that
  // was handed a member of the class in its lexicon is not independent.
  const inherited = new Map(graded.map((g) => [g.agent, new Set(g.lexiconNames.map((n) => classes.get(n)?.classId))]));
  const byClass = new Map();
  for (const [name, c] of classes) {
    const w = words.get(name);
    const entry = byClass.get(c.classId) ?? { members: [], definers: new Set(), links: new Set() };
    entry.members.push({ name: w.name, body: w.body, note: w.note ?? '', definedBy: w.definedBy ?? null });
    if (w.definedBy && !inherited.get(w.definedBy)?.has(c.classId)) entry.definers.add(w.definedBy);
    if (c.link !== 'self') entry.links.add(c.link);
    byClass.set(c.classId, entry);
  }
  const convergent = [...byClass.entries()]
    .filter(([, e]) => e.definers.size >= 2)
    .map(([classId, e]) => ({ classId, definers: [...e.definers], links: [...e.links], members: e.members }))
    .sort((a, b) => b.definers.length - a.definers.length);

  // H2: how many lexicon entries are built from other lexicon entries.
  const compositional = gens
    .filter((g) => g.lexicon.entries.length > 0)
    .map((g) => {
      const names = new Set(g.lexicon.entries.map((e) => norm(e.name)));
      const built = g.lexicon.entries.filter((e) => referencedWords(e.body, names).size > 0).length;
      return { condition: g.condition, generation: g.generation, compositional: `${built}/${names.size}` };
    });

  // H4: Core usage across correct solutions, after expanding user Words away.
  const coreUse = new Map([...CORE_NAMES].map((n) => [n, 0]));
  for (const g of graded) {
    const defs = new Map(g.definitions.map((d) => [norm(d.name), d.body]));
    for (const r of g.results) {
      if (!r.correct) continue;
      const seen = new Set();
      const walk = (source) => {
        for (const t of tokenize(source)) {
          const core = coreWordOf(t);
          if (core) seen.add(core);
          else if (known.has(norm(t)) && !seen.has(`user:${norm(t)}`)) {
            seen.add(`user:${norm(t)}`);
            walk(defs.get(norm(t)) ?? words.get(norm(t)).body);
          }
        }
      };
      walk(r.solution);
      for (const n of seen) if (!n.startsWith('user:')) coreUse.set(n, coreUse.get(n) + 1);
    }
  }

  const coreFailures = graded.flatMap((g) =>
    g.results.filter((r) => r.correct && !r.coreEquivalent).map((r) => ({ agent: g.agent, task: r.task })),
  );

  return {
    perGeneration,
    convergent,
    classCount: byClass.size,
    wordCount: words.size,
    compositional,
    coreUse: [...coreUse.entries()].sort((a, b) => b[1] - a[1]),
    coreFailures,
  };
}

export function renderReport(report) {
  const out = [];
  out.push('# Lexicon-emergence run report', '');
  out.push('## Per generation', '');
  out.push('| condition | gen | agents | lexicon | correct | mean tokens | tokens ÷ expanded | Core-equivalent (H5) |');
  out.push('| --- | --- | --- | --- | --- | --- | --- | --- |');
  for (const g of report.perGeneration) {
    out.push(`| ${g.condition} | ${g.generation} | ${g.agents} | ${g.lexiconSize} | ${g.accuracy} | ${g.meanTokens} | ${g.compression} | ${g.coreEquivalent} |`);
  }
  out.push('', `## H1 — classes defined independently by two or more agents (${report.convergent.length} of ${report.classCount} classes, ${report.wordCount} Words)`, '');
  for (const c of report.convergent) {
    out.push(`- **${c.definers.length} agents** (${c.definers.join(', ')}), joined at ${c.links.join('/') || 'D0'}:`);
    for (const m of c.members) out.push(`  - \`${m.name}\` = \`${m.body}\`${m.note ? ` — ${m.note}` : ''}`);
  }
  out.push('', '## H2 — lexicon entries built from other entries', '');
  for (const c of report.compositional) out.push(`- ${c.condition} gen${c.generation}: ${c.compositional}`);
  out.push('', '## H4 — Core Words by number of correct solutions reaching them', '');
  const used = report.coreUse.filter(([, n]) => n > 0);
  const unused = report.coreUse.filter(([, n]) => n === 0).map(([w]) => w);
  out.push(used.map(([w, n]) => `\`${w}\` ${n}`).join(' · '));
  out.push('', `Unused (${unused.length}): ${unused.map((w) => `\`${w}\``).join(' ')}`);
  out.push('', '## H5 — correct solutions whose Core-only expansion disagreed', '');
  out.push(report.coreFailures.length ? report.coreFailures.map((f) => `- ${f.agent} ${f.task}`).join('\n') : 'None.');
  return `${out.join('\n')}\n`;
}
