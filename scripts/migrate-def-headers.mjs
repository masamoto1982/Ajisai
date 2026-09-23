#!/usr/bin/env node
// Rewrite `[ body ] 'NAME' DEF` sites to carry a parameter header
// (docs/dev/word-arity-header-work-order-2026-09.md, Phase 2).
//
// A site is migrated only when the body's arity is known without running it:
// the body is simulated against the Core stack effects in spec/words.json and
// the arities of Words defined earlier in the same file. A body that reads a
// name the simulation cannot size (EXEC, an unknown Word, a computed COLLECT
// count) is left as it is and reported.
//
// Two rewrites, both meaning-preserving for a body that consumes exactly n:
//   [ 'V' BIND V … ]      →  [ V | V … ]          (leading BINDs become the header)
//   [ 2 * ]               →  [ X | X 2 * ]        (operands pushed back, point-free kept)
//
// Usage:
//   node scripts/migrate-def-headers.mjs [--write] FILE...
// Without --write it prints what it would change.

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const words = JSON.parse(readFileSync(resolve(repoRoot, 'spec/words.json'), 'utf8')).entries;
const CORE = new Map();
for (const w of words) {
  CORE.set(w.name, w.stack);
  for (const alias of w.aliases ?? []) CORE.set(alias, w.stack);
}
const LITERAL_NAMES = new Set(['TRUE', 'FALSE', 'NIL']);
const PARAM_POOL = ['X', 'Y', 'Z', 'U', 'V', 'W', 'P', 'Q'];

/** Whitespace-delimited tokens, a quoted string kept whole. */
export function tokenize(text) {
  const tokens = [];
  let i = 0;
  while (i < text.length) {
    if (/\s/.test(text[i])) {
      i += 1;
      continue;
    }
    if (text[i] === "'") {
      let j = i + 1;
      while (j < text.length && !(text[j] === "'" && (j + 1 === text.length || /\s/.test(text[j + 1])))) j += 1;
      tokens.push(text.slice(i, j + 1));
      i = j + 1;
      continue;
    }
    let j = i;
    while (j < text.length && !/\s/.test(text[j])) j += 1;
    tokens.push(text.slice(i, j));
    i = j;
  }
  return tokens;
}

const isString = (t) => t.startsWith("'");
const isNumber = (t) => /^-?\d+(\.\d+)?(\/\d+)?([eE][-+]?\d+)?$/.test(t);

/**
 * How many operands `body` reads from below its start, or null when the
 * simulation cannot tell. `userArity` maps a defined Word to { inputs, outputs }.
 */
export function simulateArity(body, userArity) {
  const tokens = tokenize(body);
  let height = 0;
  let required = 0;
  let keep = false;
  const locals = new Set();
  const take = (n) => {
    if (height < n) {
      required += n - height;
      height = n;
    }
    height -= n;
  };
  for (let i = 0; i < tokens.length; i += 1) {
    const t = tokens[i];
    const upper = t.toUpperCase();
    if (t === '[' || t === '{') {
      let depth = 1;
      const close = t === '[' ? ']' : '}';
      i += 1;
      while (i < tokens.length && depth > 0) {
        if (tokens[i] === t) depth += 1;
        else if (tokens[i] === close) depth -= 1;
        if (depth > 0) i += 1;
      }
      if (depth > 0) return null;
      height += 1;
      continue;
    }
    if (t === ']' || t === '}' || t === '|') return null;
    if (isString(t) || isNumber(t) || LITERAL_NAMES.has(upper) || locals.has(upper)) {
      height += 1;
      continue;
    }
    if (upper === 'KEEP') {
      keep = true;
      continue;
    }
    let inputs;
    let outputs;
    if (CORE.has(upper) || CORE.has(t)) {
      ({ inputs, outputs } = CORE.get(upper) ?? CORE.get(t));
      if (upper === 'COLLECT') {
        const count = tokens[i - 1];
        if (!count || !/^\d+$/.test(count)) return null;
        inputs = Number(count) + 1;
      }
      if (upper === 'BIND') {
        const name = tokens[i - 1];
        if (!name || !isString(name)) return null;
        locals.add(name.slice(1, -1).toUpperCase());
      }
      if (upper === 'DEF' || upper === 'DEL') return null;
    } else if (userArity.has(upper)) {
      ({ inputs, outputs } = userArity.get(upper));
    } else {
      return null;
    }
    if (inputs === 'variable' || outputs === 'variable') return null;
    take(inputs);
    height += keep ? inputs + outputs : outputs;
    keep = false;
  }
  return { inputs: required, outputs: height };
}

/** The header form of `inner` (the text between the body's brackets), or null. */
export function withHeader(inner, arity, avoid) {
  const tokens = tokenize(inner);
  // Leading `'X' BIND` pairs bind the top operand first: `'B' BIND 'A' BIND`
  // names B the top and A the one below, so the header lists them reversed.
  const bound = [];
  let k = 0;
  while (k + 1 < tokens.length && isString(tokens[k]) && tokens[k + 1].toUpperCase() === 'BIND') {
    bound.push(tokens[k].slice(1, -1));
    k += 2;
  }
  if (bound.length === arity && arity > 0 && new Set(bound.map((b) => b.toUpperCase())).size === arity) {
    const header = [...bound].reverse().join(' ');
    // Drop exactly the leading BIND pairs from the original text, keeping the
    // rest of its layout.
    let rest = inner;
    for (let j = 0; j < arity; j += 1) {
      rest = rest.replace(/^\s*'[^']*'\s+BIND\b/i, '');
    }
    const lead = inner.match(/^\s*/)[0];
    return `${lead}${header} |${rest}`;
  }
  const used = new Set(tokens.map((t) => t.toUpperCase()));
  const names = PARAM_POOL.filter((n) => !used.has(n) && !avoid.has(n)).slice(0, arity);
  if (names.length < arity) return null;
  const lead = inner.match(/^\s*/)[0];
  return `${lead}${[...names, '|', ...names].join(' ')} ${inner.slice(lead.length)}`;
}


/**
 * Every literal DEF site in `text`: the `[` … `]` body followed by a quoted
 * name and `DEF` (optionally `KEEP DEF`). Bracket-matched backwards from the
 * name, so nested blocks inside the body are part of it.
 */
export function findSites(text) {
  const sites = [];
  const re = /\]\s+'([^'\s]+)'\s+(?:KEEP\s+)?DEF(?![A-Za-z0-9?!-])/g;
  let m;
  while ((m = re.exec(text)) !== null) {
    const close = m.index;
    let depth = 0;
    let open = -1;
    for (let i = close; i >= 0; i -= 1) {
      const ch = text[i];
      if (ch === ']' && (i === 0 || /\s/.test(text[i - 1]) || i === close) && (i + 1 >= text.length || /\s/.test(text[i + 1]) || i === close)) depth += 1;
      else if (ch === '[' && (i + 1 < text.length && /\s/.test(text[i + 1])) && (i === 0 || /[\s"'`>(]/.test(text[i - 1]))) {
        depth -= 1;
        if (depth === 0) {
          open = i;
          break;
        }
      }
      if (ch === '"' && i < close && depth === 1 && text[i - 1] !== '\\') break;
    }
    if (open < 0) continue;
    sites.push({ open, close, name: m[1].toUpperCase(), inner: text.slice(open + 1, close) });
  }
  return sites;
}

function migrate(text) {
  const sites = findSites(text);
  const userArity = new Map();
  const avoid = new Set(sites.map((s) => s.name));
  const changes = [];
  const skipped = [];
  for (const site of sites) {
    const inner = site.inner.replace(/\\n/g, '\n');
    if (tokenize(inner).includes('|')) {
      continue;
    }
    const arity = simulateArity(inner, userArity);
    if (arity === null) {
      skipped.push(site);
      continue;
    }
    userArity.set(site.name, arity);
    const rewritten = withHeader(site.inner, arity.inputs, avoid);
    if (rewritten === null) {
      skipped.push(site);
      continue;
    }
    changes.push({ ...site, rewritten });
  }
  let out = text;
  for (const c of [...changes].sort((a, b) => b.open - a.open)) {
    out = out.slice(0, c.open + 1) + c.rewritten + out.slice(c.close);
  }
  return { out, changes, skipped };
}

const args = process.argv.slice(2);
const write = args.includes('--write');
let total = 0;
let skippedTotal = 0;
for (const file of args.filter((a) => a !== '--write')) {
  const text = readFileSync(file, 'utf8');
  const { out, changes, skipped } = migrate(text);
  total += changes.length;
  skippedTotal += skipped.length;
  if (changes.length || skipped.length) {
    console.log(`${file}: ${changes.length} migrated, ${skipped.length} left`);
    if (!write) {
      for (const c of changes) console.log(`   ${c.name}: [${c.inner.trim().slice(0, 60)}] -> [${c.rewritten.trim().slice(0, 70)}]`);
    }
    for (const s of skipped) console.log(`   left ${s.name}: [${s.inner.trim().slice(0, 70)}]`);
  }
  if (write && out !== text) writeFileSync(file, out);
}
console.log(`[migrate-def-headers] ${total} migrated, ${skippedTotal} left as they are`);
