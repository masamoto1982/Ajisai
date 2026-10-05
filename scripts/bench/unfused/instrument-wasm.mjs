#!/usr/bin/env node
// Build a call-counting copy of the WebAssembly module (step 3).
//
//   node instrument-wasm.mjs <in-dir> <out-dir>
//
// Copies the wasm-bindgen bundle from <in-dir> to <out-dir> and rewrites
// ajisai_core_bg.wasm so that every defined function increments its own
// exported mutable i32 global `__cnt_<funcIndex>` on entry. Only globals and
// exports are appended, so no existing function, global, table or data index
// moves and the JS glue is unchanged. The source tree and the committed bundle
// are not touched; the copy exists only for counting (its timings are
// meaningless). With the allocator's entry points identified by
// wasm-inspect.mjs (__rust_alloc / __rust_dealloc / __rust_realloc behind the
// __wbindgen_* exports), the counters give allocations, frees and calls per
// element.
import { readFileSync, writeFileSync, mkdirSync, copyFileSync } from "node:fs";
import { join } from "node:path";

const [inDir, outDir] = process.argv.slice(2);
mkdirSync(outDir, { recursive: true });
for (const f of ["ajisai_core.js", "ajisai_core.d.ts", "ajisai_core_bg.wasm.d.ts"]) copyFileSync(join(inDir, f), join(outDir, f));
const buf = readFileSync(join(inDir, "ajisai_core_bg.wasm"));

let p = 0;
const u8 = () => buf[p++];
const u32 = () => { let r = 0, s = 0, b; do { b = buf[p++]; r |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return r >>> 0; };
const leb = (v) => { const o = []; do { let b = v & 0x7f; v >>>= 7; if (v) b |= 0x80; o.push(b); } while (v); return Buffer.from(o); };
const name = () => { const n = u32(); p += n; };

p = 8;
const sections = [];
while (p < buf.length) { const hdr = p; const id = u8(); const size = u32(); sections.push({ id, hdr, start: p, end: p + size }); p += size; }

let importedFuncs = 0, importedGlobals = 0;
for (const s of sections.filter((s) => s.id === 2)) {
  p = s.start; const n = u32();
  for (let i = 0; i < n; i++) {
    name(); name(); const kind = u8();
    if (kind === 0) { u32(); importedFuncs++; }
    else if (kind === 1) { u8(); const f = u8(); u32(); if (f & 1) u32(); }
    else if (kind === 2) { const f = u8(); u32(); if (f & 1) u32(); }
    else if (kind === 3) { u8(); u8(); importedGlobals++; }
  }
}
const codeSec = sections.find((s) => s.id === 10);
p = codeSec.start; const nFuncs = u32();
const globSec = sections.find((s) => s.id === 6);
p = globSec.start; const nGlobals = u32(); const globBodyStart = p;
const base = importedGlobals + nGlobals; // first new global index

const sec = (id, body) => Buffer.concat([Buffer.from([id]), leb(body.length), body]);

// globals: existing + one i32 mut per defined function
const newGlob = [leb(nGlobals + nFuncs), buf.subarray(globBodyStart, globSec.end)];
for (let i = 0; i < nFuncs; i++) newGlob.push(Buffer.from([0x7f, 0x01, 0x41, 0x00, 0x0b]));

// exports: existing + __cnt_<funcIndex>
const expSec = sections.find((s) => s.id === 7);
p = expSec.start; const nExp = u32(); const expBodyStart = p;
const newExp = [leb(nExp + nFuncs), buf.subarray(expBodyStart, expSec.end)];
for (let i = 0; i < nFuncs; i++) { const nm = Buffer.from(`__cnt_${importedFuncs + i}`); newExp.push(leb(nm.length), nm, Buffer.from([0x03]), leb(base + i)); }

// code: counter bump after each body's local declarations
p = codeSec.start; u32();
const newCode = [leb(nFuncs)];
for (let i = 0; i < nFuncs; i++) {
  const size = u32(); const start = p; const end = p + size;
  const nl = u32(); for (let j = 0; j < nl; j++) { u32(); u8(); }
  const localsEnd = p;
  const g = leb(base + i);
  const bump = Buffer.concat([Buffer.from([0x23]), g, Buffer.from([0x41, 0x01, 0x6a, 0x24]), g]);
  const body = Buffer.concat([buf.subarray(start, localsEnd), bump, buf.subarray(localsEnd, end)]);
  newCode.push(leb(body.length), body);
  p = end;
}

const out = [buf.subarray(0, 8)];
for (const s of sections) {
  if (s.id === 6) out.push(sec(6, Buffer.concat(newGlob)));
  else if (s.id === 7) out.push(sec(7, Buffer.concat(newExp)));
  else if (s.id === 10) out.push(sec(10, Buffer.concat(newCode)));
  else out.push(buf.subarray(s.hdr, s.end));
}
writeFileSync(join(outDir, "ajisai_core_bg.wasm"), Buffer.concat(out));
console.error(`instrumented ${nFuncs} functions (first counter global ${base}) → ${outDir}`);
