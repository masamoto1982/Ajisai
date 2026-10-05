#!/usr/bin/env node
// Step 2 (runtime half): run every corpus program on a probed copy of the
// committed WebAssembly bundle and record, for every higher-order Word call,
// what the fused route did.
//
//   node scripts/bench/fusion-stats/trace.mjs <corpus.json> <out.json>
//
// The probed copy (probe-wasm.mjs, rebuilt into a temp dir each run) calls
// back into JS at:
//   wasm-function[328] entry/exit — execute_builtin (`interpreter/higher_order.rs`
//       and the Core Word match are inlined into it); param 2 is the WordId,
//       param 1 the Interpreter. Its entry/exit pairs give the nesting.
//   wasm-function[57] entry/exit — FusedBlock::run (`fused_block*.rs`
//       panic locations): entered only when the lowering succeeded, i.e. the
//       block is inside the fused subset; params target/seed/walk; on exit,
//       byte 0 behind the out-pointer (saved at entry) is 255 for None (the walk was declined at
//       run time: a value the fused form does not reproduce, or a limit).
//   wasm-function[910] entry — once per element of an interpreted MAP /
//       FILTER / FOLD / SCAN walk (counted to equal n on all four).
// Function indices and offsets are those of the bundle at 7edd4c0 (found with
// scripts/bench/unfused/wasm-inspect.mjs and call counting); selftest.mjs checks
// them against programs whose route is known before a run is trusted.
//
// Memory layout read (wasm32, this build): Value = 40 bytes, ValueData tag in
// byte 0 (0 Boolean, 1 Scalar, 2 ExactScalar, 3 Vector, 4 Tensor, 5 Nil,
// 6 Symbol, 7 Text, 8 Record), absence Box at +32. Vector: Arc at +4, Vec
// {cap, ptr, len} at arc+8. Tensor: shape Arc at +8, Vec{cap, ptr, len} at
// arc+8. Interpreter.current_source_span: line at +892, column at +896.
import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..", "..", "..");
export const FN = { BUILTIN: 328, FUSED_RUN: 57, ELEMENT: 910 };
const SPAN_LINE = 892, SPAN_COL = 896;
const WORD_IDS = ["TRUE","FALSE","AND","NOT","SELECT","EQ","LT","GT","ADD","SUB","MUL","DIV","FLOOR","ROUND","MIN","MAX","SQRT","POW","GCD","RATIO","GET","LENGTH","TAKE","DROP","CONCAT","REVERSE","COLLECT","RANGE","FILL","SHAPE","RESHAPE","FLATTEN","DEPTH","SORT","ORDER","UNIQUE","TALLY","ZIP","PUT","GROUP","INDEX-OF","MEMBER?","BSEARCH","RECORD","KEYS","VALUES","WITHOUT","HAS?","MERGE","MAP","FILTER","FOLD","SCAN","CHARS","JOIN","TRIM","UPPER","LOWER","TOKENIZE","SEARCH","REPLACE","NUM","STR","FORMAT","JSON-DECODE","JSON-ENCODE","EXEC","CONTRACT","FAIL","NIL","NIL?","NIL-REASON","ABSENT","BIND","DEF","DEL","DIGEST","PRINT"];
const HOF = new Set(["MAP", "FILTER", "FOLD", "SCAN"]);
const TAGS = ["Boolean", "Scalar", "ExactScalar", "Vector", "Tensor", "Nil", "Symbol", "Text", "Record"];

let probedDir = null, loads = 0;
export async function loadProbed() {
  if (probedDir) { const w = await import(join(probedDir, "ajisai_core.js") + `?r=${++loads}`); const ex = await w.default({ module_or_path: readFileSync(join(probedDir, "ajisai_core_bg.wasm")) }); return { w, ex }; }
  const dir = mkdtempSync(join(tmpdir(), "ajisai-probed-")); probedDir = dir;
  const spec = [
    { fn: FN.BUILTIN, id: 1, at: "entry", args: [2, 1] }, { fn: FN.BUILTIN, id: 2, at: "exit" },
    { fn: FN.FUSED_RUN, id: 3, at: "entry", args: [4, 5, 3] }, { fn: FN.FUSED_RUN, id: 6, at: "entry", args: [0] }, { fn: FN.FUSED_RUN, id: 4, at: "exit" },
    { fn: FN.ELEMENT, id: 5, at: "entry" },
  ];
  execFileSync("node", [join(here, "probe-wasm.mjs"), join(repo, "src/wasm/generated"), dir, JSON.stringify(spec)], { stdio: "ignore" });
  const w = await import(join(dir, "ajisai_core.js"));
  const ex = await w.default({ module_or_path: readFileSync(join(dir, "ajisai_core_bg.wasm")) });
  return { w, ex };
}

export function makeTracer(ex) {
  const dv = () => new DataView(ex.memory.buffer);
  function summarize(p) {
    if (!p) return null;
    const d = dv(); const tag = d.getUint8(p); const absent = d.getUint32(p + 32, true) !== 0;
    const s = { tag: TAGS[tag] ?? `tag${tag}`, absent };
    if (tag === 3) {
      const arc = d.getUint32(p + 4, true); const ptr = d.getUint32(arc + 12, true); const len = d.getUint32(arc + 16, true);
      s.n = len; const hist = {};
      for (let i = 0; i < len; i++) { const q = ptr + 40 * i; const t = TAGS[d.getUint8(q)] ?? "?"; const k = d.getUint32(q + 32, true) !== 0 ? t + "(absent)" : t; hist[k] = (hist[k] ?? 0) + 1; }
      s.elements = hist;
    } else if (tag === 4) {
      const sarc = d.getUint32(p + 8, true); const sptr = d.getUint32(sarc + 12, true); const rank = d.getUint32(sarc + 16, true);
      s.rank = rank; s.n = rank ? d.getUint32(sptr, true) : 0;
      s.elements = rank === 1 ? { Scalar: s.n } : { "Vector(row)": s.n };
    }
    return s;
  }
  let stack, calls, outPtrs;
  const reset = () => { stack = []; calls = []; outPtrs = []; };
  globalThis.__ajisaiProbe = (id, a, b, c) => {
    if (id === 1) {
      const word = WORD_IDS[a] ?? `#${a}`; const d = dv();
      const frame = { word, line: d.getUint32(b + SPAN_LINE, true), col: d.getUint32(b + SPAN_COL, true), parent: null, elems: 0, lowered: false, fused: null, target: null, seed: null, walk: null, seq: calls.length };
      // nearest enclosing HOF frame
      for (let k = stack.length - 1; k >= 0; k--) if (stack[k].hof) { frame.parent = stack[k].rec.seq; frame.parentElem = stack[k].rec.elems; break; }
      if (HOF.has(word)) { calls.push(frame); stack.push({ hof: true, rec: frame }); } else stack.push({ hof: false });
    } else if (id === 2) {
      stack.pop();
    } else if (id === 3) {
      const top = [...stack].reverse().find((f) => f.hof); if (!top) return;
      const r = top.rec; r.lowered = true; r.walk = ["Map", "Filter", "Fold", "Scan"][c] ?? c; r.target = summarize(a); r.seed = b ? summarize(b) : null;
    } else if (id === 6) {
      outPtrs.push(a); // the out-pointer, saved at entry: the body reuses the param's local
    } else if (id === 4) {
      const p = outPtrs.pop();
      const top = [...stack].reverse().find((f) => f.hof); if (!top) return;
      top.rec.fused = dv().getUint8(p) !== 255;
    } else if (id === 5) {
      const top = [...stack].reverse().find((f) => f.hof); if (top) top.rec.elems++;
    }
  };
  return { reset, result: () => calls };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [corpusFile, outFile] = process.argv.slice(2);
  const corpus = JSON.parse(readFileSync(corpusFile, "utf8"));
  let { w, ex } = await loadProbed();
  let tr = makeTracer(ex);
  const out = [];
  let i = 0;
  for (const prog of corpus) {
    tr.reset();
    const it = new w.AjisaiInterpreter();
    let status, message;
    try { const r = await it.execute(prog.program); status = r.status; message = r.message ?? null; }
    catch (e) { status = "THROW"; message = String(e).slice(0, 200); }
    const calls = tr.result();
    if (status === "THROW") { ({ w, ex } = await loadProbed()); tr = makeTracer(ex); out.push({ id: prog.id, status, message, calls }); continue; }
    try { it.free(); } catch {}
    out.push({ id: prog.id, status, message, calls });
    if (++i % 200 === 0) console.error(`${i}/${corpus.length}`);
  }
  writeFileSync(outFile, JSON.stringify(out));
  const n = out.reduce((a, p) => a + p.calls.length, 0);
  console.error(`traced ${out.length} programs, ${n} higher-order calls`);
}
