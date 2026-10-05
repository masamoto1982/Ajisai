#!/usr/bin/env node
// Checks that the probe points in trace.mjs (function indices and memory
// offsets, specific to the committed bundle at 7edd4c0) still mean what they
// meant: run programs whose route is known and compare the records.
//   node scripts/bench/fusion-stats/selftest.mjs
import { loadProbed, makeTracer } from "./trace.mjs";
const { w, ex } = await loadProbed(); const tr = makeTracer(ex);
const cases = [
  ["0 9 RANGE [ 1 ADD ] MAP", { word: "MAP", lowered: true, fused: true, n: 10, line: 1, col: 21 }],
  ["0 4 RANGE [ 1 POW ] MAP", { word: "MAP", lowered: false, elems: 5 }],
  ["[ 1 2 3 ] [ 2 GT ] FILTER", { word: "FILTER", lowered: true, fused: true, n: 3 }],
  ["1 360 RANGE [ 1000000 ] [ 'E' BIND 'X' BIND X 1.005 MUL ] FOLD", { word: "FOLD", lowered: true, fused: false, n: 360, elems: 360 }],
  ["1 360 RANGE 1000000 [ 'E' BIND 'X' BIND X 1.005 MUL ] FOLD", { word: "FOLD", lowered: true, fused: true, n: 360 }],
  ["\n  0 3 RANGE 0 [ ADD ] SCAN", { word: "SCAN", lowered: true, fused: true, n: 4, line: 2, col: 23 }],
];
let bad = 0;
for (const [src, want] of cases) {
  tr.reset(); const it = new w.AjisaiInterpreter(); await it.execute(src); it.free();
  const c = tr.result()[0] ?? {}; const got = { word: c.word, lowered: c.lowered, fused: c.fused, n: c.target?.n, elems: c.elems, line: c.line, col: c.col };
  const ok = Object.entries(want).every(([k, v]) => got[k] === v);
  if (!ok) bad++; console.log(ok ? "ok  " : "FAIL", src.trim(), ok ? "" : JSON.stringify(got));
}
process.exit(bad ? 1 : 0);
