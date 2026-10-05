#!/usr/bin/env node
// Attribute anonymous `wasm-function[N]` entries (the committed bundle has no
// name section) to what they do, from the module itself:
//
// - source locations: rustc passes `&'static Location {file, line, col}` to
//   every panic path (bounds checks, unwrap, RefCell borrow, overflow …) as an
//   i32.const address. Each Location found in the data segments is resolved to
//   `file:line`, so a function that can panic names the source it came from.
// - allocator: the internal functions the `__wbindgen_malloc/realloc/free`
//   exports call (dlmalloc behind __rust_alloc), and every function that calls
//   them directly.
// - bulk memory (memory.copy / memory.fill), call_indirect, br_table, size.
//
//   node wasm-inspect.mjs <module.wasm> [func indices…]   → per-function report
//   node wasm-inspect.mjs <module.wasm> --json out.json   → all functions
import { readFileSync, writeFileSync } from "node:fs";

export function inspect(buf) {
  let p = 0;
  const u8 = () => buf[p++];
  const u32 = () => { let r = 0, s = 0, b; do { b = buf[p++]; r |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return r >>> 0; };
  const s32 = () => { let r = 0, s = 0, b; do { b = buf[p++]; r |= (b & 0x7f) << s; s += 7; } while (b & 0x80); if (s < 32 && (b & 0x40)) r |= -1 << s; return r | 0; };
  const skipLeb = () => { while (buf[p++] & 0x80); };
  const name = () => { const n = u32(); const s = buf.subarray(p, p + n).toString("utf8"); p += n; return s; };

  if (buf.readUInt32LE(0) !== 0x6d736100) throw new Error("not wasm");
  p = 8;
  let importedFuncs = 0; const exports = []; const bodies = []; const data = [];
  const funcTypes = []; const types = [];
  while (p < buf.length) {
    const id = u8(); const size = u32(); const end = p + size;
    if (id === 1) {
      const n = u32();
      for (let i = 0; i < n; i++) { u8(); const np = u32(); p += np; const nr = u32(); p += nr; types.push({ np, nr }); }
    } else if (id === 2) {
      const n = u32();
      for (let i = 0; i < n; i++) {
        name(); name(); const kind = u8();
        if (kind === 0) { u32(); importedFuncs++; }
        else if (kind === 1) { u8(); const f = u8(); u32(); if (f & 1) u32(); }
        else if (kind === 2) { const f = u8(); u32(); if (f & 1) u32(); }
        else if (kind === 3) { u8(); u8(); }
      }
    } else if (id === 3) {
      const n = u32(); for (let i = 0; i < n; i++) funcTypes.push(u32());
    } else if (id === 7) {
      const n = u32(); for (let i = 0; i < n; i++) { const nm = name(); const kind = u8(); const idx = u32(); exports.push({ nm, kind, idx }); }
    } else if (id === 10) {
      const n = u32();
      for (let i = 0; i < n; i++) { const sz = u32(); bodies.push({ start: p, end: p + sz }); p += sz; }
    } else if (id === 11) {
      const n = u32();
      for (let i = 0; i < n; i++) {
        const flag = u32(); let off = null;
        if (flag === 2) u32();
        if (flag === 0 || flag === 2) { const op = u8(); if (op !== 0x41) throw new Error("data offset op " + op); off = s32(); if (u8() !== 0x0b) throw new Error("data end"); }
        const len = u32(); data.push({ off, bytes: buf.subarray(p, p + len) }); p += len;
      }
    }
    p = end;
  }

  // Memory image of the active data segments.
  const active = data.filter((d) => d.off !== null);
  const lo = Math.min(...active.map((d) => d.off)), hi = Math.max(...active.map((d) => d.off + d.bytes.length));
  const mem = Buffer.alloc(hi - lo);
  for (const d of active) d.bytes.copy(mem, d.off - lo);
  const rd32 = (addr) => (addr >= lo && addr + 4 <= hi ? mem.readUInt32LE(addr - lo) : null);
  const str = (addr, len) => (addr >= lo && addr + len <= hi ? mem.subarray(addr - lo, addr - lo + len).toString("utf8") : null);
  // Location structs: {ptr,len,line,col}, ptr → "….rs"
  const locations = new Map();
  for (let a = lo; a + 16 <= hi; a += 4) {
    const ptr = rd32(a), len = rd32(a + 4), line = rd32(a + 8), col = rd32(a + 12);
    if (len > 3 && len < 300 && line > 0 && line < 100000 && col < 1000 && ptr >= lo && ptr + len <= hi) {
      const s = str(ptr, len);
      if (s && s.endsWith(".rs") && !/[\x00-\x1f]/.test(s)) locations.set(a, `${s}:${line}`);
    }
  }

  // Decode function bodies.
  const funcs = [];
  for (let i = 0; i < bodies.length; i++) {
    const { start, end } = bodies[i];
    p = start;
    const nl = u32(); for (let j = 0; j < nl; j++) { u32(); u8(); }
    const f = { index: importedFuncs + i, size: end - start, calls: new Map(), callIndirect: 0, brTable: 0, memCopy: 0, memFill: 0, memGrow: 0, locs: new Set(), consts: [] , insns: 0 };
    while (p < end) {
      const op = u8(); f.insns++;
      switch (true) {
        case op === 0x02 || op === 0x03 || op === 0x04: {
          const b = buf[p]; if (b === 0x40 || (b >= 0x6f && b <= 0x7f)) p++; else skipLeb(); break; }
        case op === 0x0c || op === 0x0d: u32(); break;
        case op === 0x0e: { const n = u32(); for (let k = 0; k <= n; k++) u32(); f.brTable++; break; }
        case op === 0x10 || op === 0x12: { const t = u32(); f.calls.set(t, (f.calls.get(t) ?? 0) + 1); break; }
        case op === 0x11 || op === 0x13: u32(); u32(); f.callIndirect++; break;
        case op === 0x1c: { const n = u32(); p += n; break; }
        case op >= 0x20 && op <= 0x26: u32(); break;
        case op >= 0x28 && op <= 0x3e: u32(); u32(); break;
        case op === 0x3f: u8(); break;
        case op === 0x40: u8(); f.memGrow++; break;
        case op === 0x41: { const v = s32() >>> 0; if (locations.has(v)) f.locs.add(locations.get(v)); else f.consts.push(v); break; }
        case op === 0x42: skipLeb(); break;
        case op === 0x43: p += 4; break;
        case op === 0x44: p += 8; break;
        case op === 0xd0: u8(); break;
        case op === 0xd2: u32(); break;
        case op === 0xfc: {
          const sub = u32();
          if (sub === 8) { u32(); u8(); } else if (sub === 9) u32();
          else if (sub === 10) { u8(); u8(); f.memCopy++; } else if (sub === 11) { u8(); f.memFill++; }
          else if (sub === 12 || sub === 14) { u32(); u32(); } else if (sub >= 13 && sub <= 17) u32();
          break; }
        case op === 0xfd: {
          const sub = u32();
          if (sub <= 11 || sub === 92 || sub === 93) { u32(); u32(); }
          else if (sub === 12 || sub === 13) p += 16;
          else if (sub >= 21 && sub <= 34) p += 1;
          else if (sub >= 84 && sub <= 91) { u32(); u32(); p += 1; }
          break; }
        case op === 0xfe: { const sub = u32(); if (sub === 3) u8(); else { u32(); u32(); } break; }
        default: break; // no immediates
      }
    }
    funcs.push(f);
  }
  const byIndex = new Map(funcs.map((f) => [f.index, f]));
  // callers
  for (const f of funcs) f.callers = new Map();
  for (const f of funcs) for (const [t, k] of f.calls) byIndex.get(t)?.callers.set(f.index, k);

  // allocator: what the wasm-bindgen alloc exports reach
  const exp = Object.fromEntries(exports.filter((e) => e.kind === 0).map((e) => [e.nm, e.idx]));
  const reach = (start, depth) => { const seen = new Set([start]); let front = [start]; for (let d = 0; d < depth; d++) { const nx = []; for (const x of front) for (const t of byIndex.get(x)?.calls.keys() ?? []) if (!seen.has(t)) { seen.add(t); nx.push(t); } front = nx; } return seen; };
  const allocRoots = ["__wbindgen_malloc", "__wbindgen_realloc", "__wbindgen_free"].filter((k) => k in exp).map((k) => [k, exp[k]]);
  return { importedFuncs, exports, funcs, byIndex, locations, allocRoots, reach, str, rd32, lo, hi };
}

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const m = inspect(readFileSync(process.argv[2]));
  console.log(`imported funcs ${m.importedFuncs}, defined ${m.funcs.length}, locations ${m.locations.size}`);
  for (const [k, idx] of m.allocRoots) { const f = m.byIndex.get(idx); console.log(`${k} = ${idx} calls ${[...f.calls.keys()].join(",")}`); }
  const ids = process.argv.slice(3).filter((a) => /^\d+$/.test(a)).map(Number);
  for (const i of ids) {
    const f = m.byIndex.get(i); if (!f) { console.log(i, "imported"); continue; }
    const files = [...f.locs].map((l) => l.replace(/^.*?(src\/|library\/|\.cargo\/registry\/src\/[^/]+\/)/, ""));
    console.log(`\n#${i} size ${f.size} insns ${f.insns} callInd ${f.callIndirect} brTable ${f.brTable} memcpy ${f.memCopy} memfill ${f.memFill} grow ${f.memGrow}`);
    console.log(`  calls: ${[...f.calls].map(([t, k]) => `${t}x${k}`).join(" ")}`);
    console.log(`  callers: ${[...f.callers].slice(0, 12).map(([t, k]) => `${t}x${k}`).join(" ")}${f.callers.size > 12 ? ` …(${f.callers.size})` : ""}`);
    console.log(`  locs: ${files.slice(0, 12).join("  ")}${files.length > 12 ? ` …(${files.length})` : ""}`);
  }
}
