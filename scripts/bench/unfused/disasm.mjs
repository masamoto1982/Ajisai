#!/usr/bin/env node
// Minimal disassembler for reading one anonymous function: node disasm.mjs <wasm> <funcIndex>
import { readFileSync } from "node:fs";
const buf = readFileSync(process.argv[2]); const want = Number(process.argv[3]);
let p = 8; const u8 = () => buf[p++];
const u32 = () => { let r = 0, s = 0, b; do { b = buf[p++]; r |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return r >>> 0; };
const s32 = () => { let r = 0, s = 0, b; do { b = buf[p++]; r |= (b & 0x7f) << s; s += 7; } while (b & 0x80); if (s < 32 && (b & 0x40)) r |= -1 << s; return r | 0; };
const s64 = () => { let r = 0n, s = 0n, b; do { b = buf[p++]; r |= BigInt(b & 0x7f) << s; s += 7n; } while (b & 0x80); if (b & 0x40) r -= 1n << s; return r; };
const N = {0:"unreachable",1:"nop",2:"block",3:"loop",4:"if",5:"else",0xb:"end",0xc:"br",0xd:"br_if",0xe:"br_table",0xf:"return",0x10:"call",0x11:"call_indirect",0x1a:"drop",0x1b:"select",0x20:"local.get",0x21:"local.set",0x22:"local.tee",0x23:"global.get",0x24:"global.set",0x28:"i32.load",0x29:"i64.load",0x2d:"i32.load8_u",0x2c:"i32.load8_s",0x2f:"i32.load16_u",0x35:"i64.load32_u",0x36:"i32.store",0x37:"i64.store",0x3a:"i32.store8",0x3b:"i32.store16",0x3e:"i64.store32",0x41:"i32.const",0x42:"i64.const",0x45:"i32.eqz",0x46:"i32.eq",0x47:"i32.ne",0x48:"i32.lt_s",0x49:"i32.lt_u",0x4b:"i32.gt_u",0x4d:"i32.le_u",0x4f:"i32.ge_u",0x50:"i64.eqz",0x51:"i64.eq",0x52:"i64.ne",0x6a:"i32.add",0x6b:"i32.sub",0x6c:"i32.mul",0x71:"i32.and",0x72:"i32.or",0x74:"i32.shl",0x76:"i32.shr_u",0x7c:"i64.add",0x7d:"i64.sub",0x7e:"i64.mul",0x83:"i64.and",0x84:"i64.or",0xad:"i64.extend_i32_u",0xa7:"i32.wrap_i64"};
let imported = 0;
while (p < buf.length) {
  const id = u8(); const size = u32(); const end = p + size;
  if (id === 2) { const n = u32(); for (let i = 0; i < n; i++) { let l = u32(); p += l; l = u32(); p += l; const k = u8(); if (k === 0) { u32(); imported++; } else if (k === 1) { u8(); const f = u8(); u32(); if (f & 1) u32(); } else if (k === 2) { const f = u8(); u32(); if (f & 1) u32(); } else { u8(); u8(); } } }
  if (id === 10) {
    const n = u32();
    for (let i = 0; i < n; i++) {
      const sz = u32(); const st = p;
      if (imported + i !== want) { p += sz; continue; }
      const nl = u32(); for (let j = 0; j < nl; j++) { u32(); u8(); }
      let depth = 1; const out = [];
      while (p < st + sz) {
        const op = u8(); let s = N[op] ?? `op_${op.toString(16)}`;
        if (op === 2 || op === 3 || op === 4) { const b = buf[p]; if (b === 0x40 || b >= 0x6f) p++; else s32(); }
        else if (op === 0xc || op === 0xd || op === 0x10 || (op >= 0x20 && op <= 0x24)) s += " " + u32();
        else if (op === 0xe) { const k = u32(); const t = []; for (let j = 0; j <= k; j++) t.push(u32()); s += " [" + t.join(",") + "]"; }
        else if (op === 0x11) { u32(); u32(); }
        else if (op >= 0x28 && op <= 0x3e) { u32(); s += " off=" + u32(); }
        else if (op === 0x3f || op === 0x40) u8();
        else if (op === 0x41) s += " " + s32();
        else if (op === 0x42) s += " " + s64();
        else if (op === 0x43) p += 4; else if (op === 0x44) p += 8;
        else if (op === 0xfc) { const sub = u32(); s = "fc." + sub; if (sub === 10) { u8(); u8(); s = "memory.copy"; } else if (sub === 11) { u8(); s = "memory.fill"; } else if (sub === 8) { u32(); u8(); } else if (sub > 8) u32(); }
        if (op === 5 || op === 0xb) depth--;
        out.push("  ".repeat(Math.max(0, depth)) + s);
        if (op === 2 || op === 3 || op === 4 || op === 5) depth++;
      }
      console.log(out.join("\n")); process.exit(0);
    }
  }
  p = end;
}
