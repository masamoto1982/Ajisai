// Step 2 (static half): every code site of a program, and what the fused
// lowering (rust/src/interpreter/fused_block_lower.rs) would say about each
// MAP / FILTER / FOLD / SCAN block — re-implemented over the token sequence.
//
// What the lowering accepts, op by op (`Lowering::line`):
//   number literal → Push; TRUE / FALSE → PushWord; NIL → declines
//   'NAME' BIND (a one-name string directly before BIND) → Bind
//   ADD SUB MUL DIV LT GT EQ MIN MAX FLOOR ROUND NOT AND SELECT → op
//   a name bound earlier in the same frame → Load
//   a name bound outside the block (top-level frame only) → Push of its value
//     if that value is a plain rational/Boolean — only known at run time
//   a User Word → its body inlined in a frame of its own (sees no outer names)
//   anything else (other Words, vector literals, nested blocks, strings) → declines
// plus: no op may pop more than the block's stack holds (1 input for
// MAP/FILTER, 2 for FOLD/SCAN), and the block must leave a value.
import { readFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const CORE = new Set(JSON.parse(readFileSync(join(repo, "spec/words.json"), "utf8")).entries.map((e) => e.name));
export const SUBSET = new Map([["ADD", 2], ["SUB", 2], ["MUL", 2], ["DIV", 2], ["LT", 2], ["GT", 2], ["EQ", 2], ["MIN", 2], ["MAX", 2], ["AND", 2], ["FLOOR", 1], ["ROUND", 1], ["NOT", 1], ["SELECT", 3]]);
export const HOFS = new Set(["MAP", "FILTER", "FOLD", "SCAN"]);
const up = (t) => (t.kind === "symbol" ? t.value.toUpperCase() : null);

// Bracket groups: index of open → index of matching close.
function matchBrackets(tokens) {
  const close = new Map(); const st = [];
  tokens.forEach((t, i) => { if (t.kind === "open") st.push(i); else if (t.kind === "close") { const o = st.pop(); if (o !== undefined) close.set(o, i); } });
  return close;
}

// Walk a code range [from, to) and classify every group in it.
export function analyzeProgram(tokens) {
  const close = matchBrackets(tokens);
  const sites = []; const defs = new Map(); const groups = [];
  const BOUND_OUTER = new Set(); // names bound by a BIND anywhere outside blocks' own frames (approximation)
  function walkCode(from, to, context) {
    for (let i = from; i < to; i++) {
      const t = tokens[i];
      if (t.kind === "string" && up(tokens[i + 1] ?? {}) === "BIND") for (const n of t.value.split(/\s+/)) BOUND_OUTER.add(n.toUpperCase());
      if (t.kind !== "open") continue;
      const j = close.get(i); if (j === undefined) return;
      const next = tokens[j + 1]; const nextU = next ? up(next) : null;
      const g = { open: i, close: j, context, line: t.line, col: t.col };
      if (nextU && HOFS.has(nextU)) {
        g.consumer = nextU; g.hofToken = j + 1;
        const site = { id: sites.length, hof: nextU, open: i, close: j, hofLine: next.line, hofCol: next.col, line: t.line, col: t.col, context };
        sites.push(site); g.site = site.id;
        walkCode(i + 1, j, { site: site.id });
        i = j + 1;
      } else if (nextU === "EXEC") { g.consumer = "EXEC"; walkCode(i + 1, j, { exec: i }); i = j + 1; }
      else if (next?.kind === "string" && up(tokens[j + 2] ?? {}) === "DEF") {
        g.consumer = "DEF"; const name = next.value.toUpperCase(); defs.set(name, { open: i, close: j }); walkCode(i + 1, j, { def: name }); i = j + 2;
      } else { g.consumer = "data"; i = j; }
      groups.push(g);
    }
  }
  walkCode(0, tokens.length, { top: true });

  // Lower one block body [from, to) for `inputs` operands.
  function lower(from, to, inputs, seesOuter, callDepth, via, out) {
    const frame = new Set(); let depth = inputs; let stopped = false;
    const stop = (reason, at) => { out.reasons.push({ reason, via, line: tokens[at]?.line, col: tokens[at]?.col, beforeStop: !stopped }); stopped = true; };
    const pop = (k, at) => { if (stopped) return; if (depth < k) stop("underflow", at); else depth -= k; };
    for (let i = from; i < to; i++) {
      const t = tokens[i];
      if (t.kind === "number") { if (!stopped) depth++; continue; }
      if (t.kind === "string") {
        if (up(tokens[i + 1] ?? {}) === "BIND" && i + 1 < to) {
          const names = t.value.trim().split(/\s+/);
          if (names.length !== 1 || CORE.has(names[0].toUpperCase())) stop("BIND (destructuring / reserved name)", i);
          else { pop(1, i); frame.add(names[0].toUpperCase()); }
          i++; continue;
        }
        stop("string literal", i); continue;
      }
      if (t.kind === "open") {
        const j = close.get(i); const nu = up(tokens[j + 1] ?? {});
        if (nu && HOFS.has(nu)) { stop(`nested block → ${nu}`, i); i = j + 1; }
        else if (nu === "EXEC") { stop("nested block → EXEC", i); i = j + 1; }
        else if (tokens[j + 1]?.kind === "string" && up(tokens[j + 2] ?? {}) === "DEF") { stop("nested block → DEF", i); i = j + 2; }
        else { stop("vector literal", i); i = j; }
        continue;
      }
      if (t.kind === "close") continue;
      const u = up(t);
      if (u === "TRUE" || u === "FALSE") { if (!stopped) depth++; continue; }
      if (u === "NIL") { stop("NIL literal", i); continue; }
      if (SUBSET.has(u)) { pop(SUBSET.get(u), i); if (!stopped) depth++; continue; }
      if (CORE.has(u)) { stop(`word ${u}`, i); continue; }
      if (frame.has(u)) { if (!stopped) depth++; continue; }
      if (defs.has(u)) {
        out.userWords.push(u);
        const d = defs.get(u);
        if (callDepth + 1 > 64) { stop("user word depth", i); continue; }
        const sub = { reasons: [], userWords: [], outer: [], bodyFusible: out.bodyFusible };
        const subDepth = lowerBody(d.open + 1, d.close, depth, callDepth + 1, u, sub);
        out.userWords.push(...sub.userWords);
        out.bodyFusible[u] = sub.reasons.length === 0;
        for (const r of sub.reasons) out.reasons.push({ ...r, beforeStop: r.beforeStop && !stopped });
        if (sub.reasons.length) stopped = true; else depth = subDepth;
        continue;
      }
      if (seesOuter && BOUND_OUTER.has(u)) { out.outer.push(u); if (!stopped) depth++; continue; }
      stop(`unresolved name ${u}`, i);
    }
    if (!stopped && depth === 0) stop("block leaves no value", to);
    return depth;
  }
  // A User Word body: the same rules in a frame of its own that sees no outer names.
  function lowerBody(from, to, depth0, callDepth, name, out) {
    out.bodyFusible = out.bodyFusible ?? {};
    const frame = new Set(); let depth = depth0; let stopped = false;
    const inner = { reasons: out.reasons, userWords: out.userWords, outer: out.outer, bodyFusible: out.bodyFusible };
    // reuse lower() but with seesOuter=false and the caller's depth; the
    // "leaves no value" check applies to the whole block, not to a body
    const r = lower2(from, to, depth0, false, callDepth, name, inner);
    return r;
  }
  function lower2(from, to, inputs, seesOuter, callDepth, via, out) {
    const saved = out.reasons.length;
    const d = lower(from, to, inputs, seesOuter, callDepth, via, out);
    // drop the end-of-block check for a body (the block it is inlined into decides)
    const last = out.reasons[out.reasons.length - 1];
    if (out.reasons.length > saved && last.reason === "block leaves no value" && last.via === via) out.reasons.pop();
    return d;
  }
  for (const s of sites) {
    const out = { reasons: [], userWords: [], outer: [], bodyFusible: {} };
    lower(s.open + 1, s.close, s.hof === "FOLD" || s.hof === "SCAN" ? 2 : 1, true, 0, null, out);
    s.reasons = out.reasons; s.userWords = [...new Set(out.userWords)]; s.outer = [...new Set(out.outer)]; s.bodyFusible = out.bodyFusible;
    s.staticFusible = out.reasons.length === 0;
    if (s.staticFusible && (s.hof === "FOLD" || s.hof === "SCAN")) s.laneWalkable = laneWalkable(s.open + 1, s.close);
    s.text = tokens.slice(s.open, s.close + 1).map((t) => (t.kind === "string" ? `'${t.value}'` : t.value)).join(" ");
  }
  // FusedBlock::lane_mixed_per_run: with a one-lane seed ([ 0 ]) the walk is
  // taken only when the lane (the accumulator) reaches nothing but
  // ADD/SUB/MUL/DIV, BIND and a bound name, and the result is the lane.
  // (User Word bodies are not followed here.)
  function laneWalkable(from, to) {
    const st = [true, false]; const slots = new Map();
    for (let i = from; i < to; i++) {
      const t = tokens[i]; const u = up(t);
      if (t.kind === "number" || u === "TRUE" || u === "FALSE") { st.push(false); continue; }
      if (t.kind === "string") { slots.set(t.value.toUpperCase(), st.pop()); i++; continue; }
      if (["ADD", "SUB", "MUL", "DIV"].includes(u)) { const b = st.pop(), a = st.pop(); st.push(a || b); continue; }
      if (["LT", "GT", "EQ", "AND", "MIN", "MAX"].includes(u)) { const b = st.pop(), a = st.pop(); if (a || b) return false; st.push(false); continue; }
      if (["FLOOR", "ROUND", "NOT"].includes(u)) { if (st.pop()) return false; st.push(false); continue; }
      if (u === "SELECT") { const m = st.pop(), f = st.pop(), tt = st.pop(); if (m || f || tt) return false; st.push(false); continue; }
      if (slots.has(u)) { st.push(slots.get(u)); continue; }
      st.push(false); // outer name or User Word (approximation)
    }
    return st[st.length - 1] === true;
  }
  return { sites, defs, groups, close };
}
