#!/usr/bin/env node
// Step 3: link the runtime record of every higher-order call (trace.mjs) to the
// block it ran (static.mjs), and aggregate.
//
//   node scripts/bench/fusion-stats/stats.mjs <corpus.json> <trace.json> <out-dir>
//
// Linking. The probe gives each call its Word, the top-level token being run
// (line/column) and its enclosing higher-order call. A top-level call whose
// token is the Word itself is the block written right before it. A call inside
// another call's block, or inside a User Word body reached from the top-level
// token, is matched to the candidate blocks of that Word in that code (the
// enclosing block, or the bodies reachable from it); when several match, by
// order among the calls the same enclosing element made. Each link is labelled
// exact / ordinal / unlinked.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { lexProgram } from "./lexer.mjs";
import { analyzeProgram } from "./static.mjs";

const [corpusFile, traceFile, outDir] = process.argv.slice(2);
mkdirSync(outDir, { recursive: true });
const corpus = JSON.parse(readFileSync(corpusFile, "utf8"));
const traces = new Map(JSON.parse(readFileSync(traceFile, "utf8")).map((t) => [t.id, t]));

const blocks = []; // one per static HOF site that some call ran
const nonHof = { DEF: 0, EXEC: 0 };
const link = { exact: 0, ordinal: 0, unlinked: 0 };
const unlinkedCalls = [];
const validation = { lowerAgree: 0, lowerDisagree: 0, disagreements: [], elemCheck: { equal: 0, differ: 0 } };

for (const prog of corpus) {
  const tr = traces.get(prog.id); if (!tr) continue;
  const lx = lexProgram(prog.program); if (lx.error) continue;
  const toks = lx.tokens;
  const { sites, defs, groups } = analyzeProgram(toks);
  for (const g of groups) if (g.consumer === "DEF" || g.consumer === "EXEC") nonHof[g.consumer]++;
  const tokAt = new Map(toks.map((t, i) => [`${t.line}:${t.col}`, i]));
  const siteByHofTok = new Map(sites.map((s) => [`${s.hofLine}:${s.hofCol}`, s]));
  // sites inside a code region, plus the bodies of User Words it names (transitively)
  const sitesIn = (pred, seenDefs = new Set()) => {
    const direct = sites.filter((s) => pred(s.context));
    const names = new Set();
    const scan = (from, to) => { for (let i = from; i < to; i++) { const t = toks[i]; if (t.kind === "symbol" && defs.has(t.value.toUpperCase())) names.add(t.value.toUpperCase()); } };
    return { direct, names, scan };
  };
  const candidatesForRegion = (from, to, seen = new Set()) => {
    const out = sites.filter((s) => s.open >= from && s.close < to && !sites.some((o) => o !== s && o.open < s.open && o.close > s.close && o.open >= from && o.close < to));
    for (let i = from; i < to; i++) { const t = toks[i]; const u = t.kind === "symbol" ? t.value.toUpperCase() : null; if (u && defs.has(u) && !seen.has(u)) { seen.add(u); const d = defs.get(u); out.push(...candidatesForRegion(d.open + 1, d.close, seen)); } }
    return out.sort((a, b) => a.open - b.open);
  };
  const linked = new Map(); // call seq → site
  const siblingCount = new Map();
  for (const c of tr.calls) {
    let site = null, how = null;
    if (c.parent == null) {
      const s = siteByHofTok.get(`${c.line}:${c.col}`);
      if (s && s.hof === c.word && s.context.top) { site = s; how = "exact"; }
      else {
        const ti = tokAt.get(`${c.line}:${c.col}`);
        let cands = [];
        if (ti !== undefined) {
          const t = toks[ti]; const u = t.kind === "symbol" ? t.value.toUpperCase() : null;
          if (u && defs.has(u)) { const d = defs.get(u); cands = candidatesForRegion(d.open + 1, d.close, new Set([u])); }
          else if (u === "EXEC") { const g = groups.find((g) => g.close === ti - 1); if (g) cands = candidatesForRegion(g.open + 1, g.close); }
          else if (s && s.hof === c.word) cands = [s];
        }
        const key = `top:${c.line}:${c.col}`;
        ({ site, how } = pick(cands, c, key));
      }
    } else {
      const ps = linked.get(c.parent);
      if (ps) { const cands = candidatesForRegion(ps.open + 1, ps.close); ({ site, how } = pick(cands, c, `p:${c.parent}:${c.parentElem}`)); }
    }
    if (!site) { link.unlinked++; unlinkedCalls.push({ prog: prog.id, word: c.word, line: c.line, col: c.col, n: callN(c) }); continue; }
    link[how]++; linked.set(c.seq, site);
    (site.calls ??= []).push(c);
  }
  function pick(cands, c, key) {
    const same = cands.filter((s) => s.hof === c.word);
    if (same.length === 1) return { site: same[0], how: "exact" };
    if (!same.length) return { site: null, how: null };
    const k = siblingCount.get(key + c.word) ?? 0; siblingCount.set(key + c.word, k + 1);
    return { site: same[k % same.length], how: "ordinal" };
  }
  for (const s of sites) {
    if (!s.calls) continue;
    const calls = s.calls.map((c) => ({ n: callN(c), lowered: c.lowered, fused: c.fused, target: c.target, seed: c.seed, elems: c.elems }));
    const work = calls.filter((c) => c.n > 0);
    for (const c of calls) if (c.lowered && c.target && c.fused === false) { if (c.elems === c.target.n) validation.elemCheck.equal++; else validation.elemCheck.differ++; }
    const rtLowered = work.length ? work.every((c) => c.lowered) : null;
    if (rtLowered !== null) {
      if (rtLowered === s.staticFusible) validation.lowerAgree++;
      else { validation.lowerDisagree++; validation.disagreements.push({ prog: prog.id, kind: prog.kind, text: s.text, static: s.reasons.map((r) => r.reason), outer: s.outer, rtLowered, mixed: work.some((c) => c.lowered) && !rtLowered }); }
    }
    blocks.push({ prog: prog.id, kind: prog.kind, file: prog.file, hof: s.hof, text: s.text, context: s.context.top ? "top" : s.context.site != null ? "nested" : s.context.def ? "def" : "exec",
      laneWalkable: s.laneWalkable, reasons: s.reasons, userWords: s.userWords, bodyFusible: s.bodyFusible, outer: s.outer, staticFusible: s.staticFusible,
      calls: calls.length, workCalls: work.length, n: work.reduce((a, c) => a + c.n, 0),
      nFused: work.filter((c) => c.fused).reduce((a, c) => a + c.n, 0),
      loweredCalls: work.filter((c) => c.lowered).length, fusedCalls: work.filter((c) => c.fused).length,
      valueDeclined: work.filter((c) => c.lowered && c.fused === false).map((c) => ({ n: c.n, elems: c.elems, target: c.target, seed: c.seed })) });
  }
}
function callN(c) { return c.lowered && c.target?.n != null ? c.target.n : c.elems; }

writeFileSync(join(outDir, "blocks.json"), JSON.stringify(blocks, null, 1));
writeFileSync(join(outDir, "meta.json"), JSON.stringify({ link, nonHof, validation, unlinked: unlinkedCalls.slice(0, 200), unlinkedTotal: unlinkedCalls.length, unlinkedN: unlinkedCalls.reduce((a, c) => a + c.n, 0) }, null, 1));
console.error(`blocks ${blocks.length}; links ${JSON.stringify(link)}; static vs runtime lowering agree ${validation.lowerAgree} / disagree ${validation.lowerDisagree}; element count check ${JSON.stringify(validation.elemCheck)}`);
