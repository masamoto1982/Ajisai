#!/usr/bin/env node
// Build a probed copy of the WebAssembly bundle: selected functions call an
// added import `probe.ev(id, a, b, c)` on entry and/or on exit, passing up to
// three of their own locals (params). The committed bundle and the source tree
// are untouched; the copy goes to <out-dir> with a JS glue that wires the
// import to `globalThis.__ajisaiProbe`.
//
//   node probe-wasm.mjs <in-dir> <out-dir> '<json spec>'
//   spec: [{ "fn": 57, "id": 1, "at": "entry"|"exit", "args": [2,3,0] }, ...]
//   (fn is the function index in the *original* module; on "exit" the args are
//   read after the body ran, so a pointer param still points at what the
//   function wrote through it)
//
// Adding an import shifts every defined function index by one, so every
// call / return_call / ref.func immediate, the element segments, the exports
// and the start function are renumbered. Exit probes wrap the body in a block
// of the function's result type (so a branch to the function label lands on
// the probe) and also fire before each `return`.
import { readFileSync, writeFileSync, mkdirSync, copyFileSync } from "node:fs";
import { join } from "node:path";

export function buildProbed(buf, spec) {
  let p = 0;
  const u8 = () => buf[p++];
  const u32 = () => { let r = 0, s = 0, b; do { b = buf[p++]; r |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return r >>> 0; };
  const skipLeb = () => { while (buf[p++] & 0x80); };
  const leb = (v) => { const o = []; do { let b = v & 0x7f; v >>>= 7; if (v) b |= 0x80; o.push(b); } while (v); return Buffer.from(o); };
  const B = (...a) => Buffer.from(a);
  const sec = (id, body) => Buffer.concat([B(id), leb(body.length), body]);

  p = 8; const sections = [];
  while (p < buf.length) { const hdr = p; const id = u8(); const size = u32(); sections.push({ id, hdr, start: p, end: p + size }); p += size; }
  const S = (id) => sections.find((s) => s.id === id);

  // types
  const types = []; { const s = S(1); p = s.start; const n = u32(); for (let i = 0; i < n; i++) { u8(); const np = u32(); const params = [...buf.subarray(p, p + np)]; p += np; const nr = u32(); const results = [...buf.subarray(p, p + nr)]; p += nr; types.push({ params, results }); } }
  const probeType = types.length;
  // imports
  let importedFuncs = 0; let importEntries; { const s = S(2); p = s.start; const n = u32(); const st = p;
    for (let i = 0; i < n; i++) { let l = u32(); p += l; l = u32(); p += l; const k = u8(); if (k === 0) { u32(); importedFuncs++; } else if (k === 1) { u8(); const f = u8(); u32(); if (f & 1) u32(); } else if (k === 2) { const f = u8(); u32(); if (f & 1) u32(); } else { u8(); u8(); } }
    importEntries = { n, bytes: buf.subarray(st, s.end) }; }
  // function types
  const funcType = []; { const s = S(3); p = s.start; const n = u32(); for (let i = 0; i < n; i++) funcType.push(u32()); }
  const sh = (idx) => (idx >= importedFuncs ? idx + 1 : idx);
  const byFn = new Map(); for (const e of spec) { if (!byFn.has(e.fn)) byFn.set(e.fn, []); byFn.get(e.fn).push(e); }

  const out = [buf.subarray(0, 8)];
  for (const s of sections) {
    if (s.id === 1) {
      out.push(sec(1, Buffer.concat([leb(types.length + 1), buf.subarray(s.start + leb(types.length).length, s.end), B(0x60, 4, 0x7f, 0x7f, 0x7f, 0x7f, 0)])));
    } else if (s.id === 2) {
      const mod = Buffer.from("probe"), nm = Buffer.from("ev");
      out.push(sec(2, Buffer.concat([leb(importEntries.n + 1), importEntries.bytes, leb(mod.length), mod, leb(nm.length), nm, B(0), leb(probeType)])));
    } else if (s.id === 7) {
      p = s.start; const n = u32(); const parts = [leb(n)];
      for (let i = 0; i < n; i++) { const l = u32(); const name = buf.subarray(p, p + l); p += l; const k = u8(); let idx = u32(); if (k === 0) idx = sh(idx); parts.push(leb(l), name, B(k), leb(idx)); }
      out.push(sec(7, Buffer.concat(parts)));
    } else if (s.id === 8) {
      p = s.start; out.push(sec(8, leb(sh(u32()))));
    } else if (s.id === 9) {
      p = s.start; const n = u32(); const parts = [leb(n)];
      const expr = () => { const o = []; for (;;) { const op = u8(); if (op === 0x0b) { o.push(B(0x0b)); break; } if (op === 0xd2) { o.push(B(0xd2), leb(sh(u32()))); } else if (op === 0xd0) { o.push(B(0xd0, u8())); } else if (op === 0x41) { const st = p; skipLeb(); o.push(B(0x41), buf.subarray(st, p)); } else if (op === 0x23) { o.push(B(0x23), leb(u32())); } else throw new Error("elem expr op " + op); } return Buffer.concat(o); };
      for (let i = 0; i < n; i++) {
        const flag = u32(); parts.push(leb(flag));
        if (flag === 0) { parts.push(expr()); const k = u32(); parts.push(leb(k)); for (let j = 0; j < k; j++) parts.push(leb(sh(u32()))); }
        else if (flag === 1 || flag === 3) { parts.push(B(u8())); const k = u32(); parts.push(leb(k)); for (let j = 0; j < k; j++) parts.push(leb(sh(u32()))); }
        else if (flag === 2) { parts.push(leb(u32()), expr(), B(u8())); const k = u32(); parts.push(leb(k)); for (let j = 0; j < k; j++) parts.push(leb(sh(u32()))); }
        else if (flag === 4) { parts.push(expr()); const k = u32(); parts.push(leb(k)); for (let j = 0; j < k; j++) parts.push(expr()); }
        else if (flag === 5 || flag === 7) { parts.push(B(u8())); const k = u32(); parts.push(leb(k)); for (let j = 0; j < k; j++) parts.push(expr()); }
        else if (flag === 6) { parts.push(leb(u32()), expr(), B(u8())); const k = u32(); parts.push(leb(k)); for (let j = 0; j < k; j++) parts.push(expr()); }
      }
      out.push(sec(9, Buffer.concat(parts)));
    } else if (s.id === 10) {
      p = s.start; const n = u32(); const parts = [leb(n)];
      for (let i = 0; i < n; i++) {
        const size = u32(); const st = p; const end = p + size; const fnIdx = importedFuncs + i;
        const probes = byFn.get(fnIdx) ?? []; const entry = probes.filter((e) => e.at === "entry"); const exit = probes.filter((e) => e.at === "exit");
        const ty = types[funcType[i]]; const nParams = ty.params.length;
        const nl = u32(); const localDecls = []; let nLocals = 0;
        for (let j = 0; j < nl; j++) { const c = u32(); const t = u8(); localDecls.push([c, t]); nLocals += c; }
        let tmp = -1;
        if (exit.length && ty.results.length === 1) { tmp = nParams + nLocals; localDecls.push([1, ty.results[0]]); }
        if (exit.length && ty.results.length > 1) throw new Error(`fn ${fnIdx}: multi-value result`);
        const call = (e) => Buffer.concat([B(0x41), leb(e.id), ...[0, 1, 2].map((k) => (e.args?.[k] != null ? Buffer.concat([B(0x20), leb(e.args[k])]) : B(0x41, 0))), B(0x10), leb(importedFuncs)]);
        const exitSeq = () => exit.length === 0 ? Buffer.alloc(0) : (tmp >= 0 ? Buffer.concat([B(0x21), leb(tmp), ...exit.map(call), B(0x20), leb(tmp)]) : Buffer.concat(exit.map(call)));
        const body = [];
        body.push(leb(localDecls.length)); for (const [c, t] of localDecls) body.push(leb(c), B(t));
        body.push(...entry.map(call));
        if (exit.length) body.push(B(0x02, ty.results.length ? ty.results[0] : 0x40));
        // copy instructions, renumbering calls
        while (p < end) {
          const ip = p; const op = u8();
          if (p >= end && op === 0x0b) { // the body's final end
            if (exit.length) body.push(B(0x0b), exitSeq());
            body.push(B(0x0b)); break;
          }
          switch (true) {
            case op === 0x10 || op === 0x12: { const t = u32(); body.push(B(op), leb(sh(t))); continue; }
            case op === 0xd2: { const t = u32(); body.push(B(op), leb(sh(t))); continue; }
            case op === 0x0f: { if (exit.length) body.push(exitSeq()); body.push(B(op)); continue; }
            case op === 0x02 || op === 0x03 || op === 0x04: { const b = buf[p]; if (b === 0x40 || (b >= 0x6f && b <= 0x7f)) p++; else skipLeb(); break; }
            case op === 0x0c || op === 0x0d: u32(); break;
            case op === 0x0e: { const k = u32(); for (let j = 0; j <= k; j++) u32(); break; }
            case op === 0x11 || op === 0x13: u32(); u32(); break;
            case op === 0x1c: { const k = u32(); p += k; break; }
            case op >= 0x20 && op <= 0x26: u32(); break;
            case op >= 0x28 && op <= 0x3e: u32(); u32(); break;
            case op === 0x3f || op === 0x40: u8(); break;
            case op === 0x41 || op === 0x42: skipLeb(); break;
            case op === 0x43: p += 4; break;
            case op === 0x44: p += 8; break;
            case op === 0xd0: u8(); break;
            case op === 0xfc: { const sub = u32(); if (sub === 8) { u32(); u8(); } else if (sub === 10) { u8(); u8(); } else if (sub === 11) u8(); else if (sub === 12 || sub === 14) { u32(); u32(); } else if (sub >= 9 && sub <= 17) u32(); break; }
            case op === 0xfd: { const sub = u32(); if (sub <= 11 || sub === 92 || sub === 93) { u32(); u32(); } else if (sub === 12 || sub === 13) p += 16; else if (sub >= 21 && sub <= 34) p += 1; else if (sub >= 84 && sub <= 91) { u32(); u32(); p += 1; } break; }
            case op === 0xfe: { const sub = u32(); if (sub === 3) u8(); else { u32(); u32(); } break; }
            default: break;
          }
          body.push(buf.subarray(ip, p));
        }
        const bb = Buffer.concat(body); parts.push(leb(bb.length), bb); p = end;
      }
      out.push(sec(10, Buffer.concat(parts)));
    } else out.push(buf.subarray(s.hdr, s.end));
  }
  return Buffer.concat(out);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [inDir, outDir, specJson] = process.argv.slice(2);
  mkdirSync(outDir, { recursive: true });
  for (const f of ["ajisai_core.d.ts", "ajisai_core_bg.wasm.d.ts"]) copyFileSync(join(inDir, f), join(outDir, f));
  const glue = readFileSync(join(inDir, "ajisai_core.js"), "utf8").replaceAll("const imports = __wbg_get_imports();", "const imports = __wbg_get_imports(); imports.probe = { ev: (i, a, b, c) => globalThis.__ajisaiProbe(i, a, b, c) };");
  writeFileSync(join(outDir, "ajisai_core.js"), glue);
  writeFileSync(join(outDir, "ajisai_core_bg.wasm"), buildProbed(readFileSync(join(inDir, "ajisai_core_bg.wasm")), JSON.parse(specJson)));
  console.error(`probed copy → ${outDir}`);
}
