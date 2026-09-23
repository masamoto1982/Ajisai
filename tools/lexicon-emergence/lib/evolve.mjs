// Classifying Words into "the same Word" classes, and choosing the next
// generation's lexicon under the capacity bottleneck (work order §5.1).

import { identify } from './identity.mjs';
import { closure, norm, tokenize } from './source.mjs';

/** Every Word that was ever in play: lexicon entries and each submission's own definitions. */
export function collectWords(graded, lexicons) {
  const words = new Map();
  for (const lexicon of lexicons) {
    for (const entry of lexicon.entries) words.set(norm(entry.name), { ...entry, origin: 'lexicon' });
  }
  for (const g of graded) {
    for (const d of g.definitions) {
      words.set(norm(d.name), { ...d, definedBy: g.agent, generation: g.generation, condition: g.condition });
    }
  }
  return words;
}

/** Union Words that share a D0, D0α or D1 key. Returns name → { classId, link }. */
export async function classify(ajisai, words) {
  const ids = [];
  for (const name of words.keys()) ids.push(await identify(ajisai, name, words));

  const parent = new Map(ids.map((id) => [id.name, id.name]));
  const find = (x) => (parent.get(x) === x ? x : find(parent.get(x)));
  const link = new Map();
  for (const level of ['d0', 'd0a', 'd1']) {
    const first = new Map();
    for (const id of ids) {
      const key = id[level];
      if (key === null) continue;
      if (!first.has(key)) {
        first.set(key, id.name);
        continue;
      }
      const a = find(first.get(key));
      const b = find(id.name);
      if (a !== b) {
        parent.set(b, a);
        link.set(id.name, level);
      }
    }
  }
  const classes = new Map();
  for (const id of ids) {
    classes.set(id.name, { classId: find(id.name), link: link.get(id.name) ?? 'self', ...id });
  }
  return classes;
}

/** Uses per class: how many correct solutions reached it, and by which agents. */
export function usage(graded, classes) {
  const byClass = new Map();
  for (const g of graded) {
    for (const r of g.results) {
      if (!r.correct) continue;
      const reached = new Set(r.uses.map((name) => classes.get(name)?.classId).filter(Boolean));
      for (const classId of reached) {
        const u = byClass.get(classId) ?? { uses: 0, agents: new Set() };
        u.uses += 1;
        u.agents.add(g.agent);
        byClass.set(classId, u);
      }
    }
  }
  return byClass;
}

/**
 * The top-K classes by use, each represented by its most-used member and
 * carried with the definitions that member depends on. The dependencies
 * count against K: capacity is a limit on what the next generation reads.
 *
 * A dependency whose class is already represented is rewritten to call the
 * representative instead of being carried a second time — but only when that
 * class was joined at D0 or D0α, where the two are the same Word. A class
 * joined at D1 agreed on the probes only, so its members are carried as they
 * are.
 */
export function nextLexicon(generation, capacity, graded, words, classes) {
  const perClass = usage(graded, classes);
  const memberUses = new Map();
  for (const g of graded) {
    for (const r of g.results) if (r.correct) for (const n of r.uses) memberUses.set(n, (memberUses.get(n) ?? 0) + 1);
  }
  const membersOf = new Map();
  for (const c of classes.values()) membersOf.set(c.classId, [...(membersOf.get(c.classId) ?? []), c]);
  const strong = (classId) => membersOf.get(classId).every((c) => c.link !== 'd1');
  const byPreference = (a, b) =>
    (memberUses.get(b) ?? 0) - (memberUses.get(a) ?? 0) ||
    tokenize(words.get(a).body).length - tokenize(words.get(b).body).length;

  const representative = new Map(); // classId → chosen name, for strong classes
  const chosen = [];
  const chosenNames = new Set();
  const rewrite = (body) =>
    tokenize(body)
      .map((t) => {
        const c = classes.get(norm(t));
        return c && strong(c.classId) && representative.has(c.classId) ? representative.get(c.classId) : t;
      })
      .join(' ');
  const include = (name) => {
    const c = classes.get(name);
    if (chosenNames.has(name) || (strong(c.classId) && representative.has(c.classId))) return [];
    const plan = [];
    for (const dep of closure([name], words).slice(0, -1)) plan.push(...include(norm(dep.name)));
    return [...plan, name];
  };

  const ranked = [...perClass.entries()].sort(([, a], [, b]) => b.uses - a.uses || b.agents.size - a.agents.size);
  for (const [classId] of ranked) {
    if (representative.has(classId)) continue;
    const members = membersOf.get(classId).map((c) => c.name).sort(byPreference);
    const plan = [...new Set(include(members[0]))];
    if (chosen.length + plan.length > capacity) continue;
    for (const name of plan) {
      const c = classes.get(name);
      const d = words.get(name);
      chosen.push({
        name: d.name,
        body: rewrite(d.body),
        note: d.note ?? '',
        classId: c.classId,
        uses: perClass.get(c.classId)?.uses ?? 0,
        agents: [...(perClass.get(c.classId)?.agents ?? [])],
      });
      chosenNames.add(name);
      if (strong(c.classId)) representative.set(c.classId, d.name);
    }
  }
  return { generation, capacity, entries: chosen };
}
