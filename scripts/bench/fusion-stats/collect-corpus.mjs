#!/usr/bin/env node
// Step 1: collect Ajisai programs from the repository into corpus.json.
//
//   node scripts/bench/fusion-stats/collect-corpus.mjs <out.json>
//
// kinds (aggregated separately):
//   test     — Rust tests (test files and #[cfg(test)] modules), wasm-tests,
//              TS/JS tests, spec/outcome-witnesses.json, MCP golden cases,
//              tests/formatter-corpus.json
//   example  — README, SKILL.md, MCP quickstarts/README, Reference samples
//              (public/docs, docs/word-reference.md), the `syntax` example of
//              every Word in spec/words.json, MCP evaluation prompts
//              (eval/*.json, traces), Playground Example Words,
//              SPECIFICATION.html
//   bench    — scripts/bench/speed-bench-cases.json (written to exercise
//              interpreter routes)
//   business — the business-calculation and cross-language comparison programs
//              of the earlier reports (business.mjs)
//
// A string counts as an Ajisai program when the reference lexer accepts it and
// every bare name in it is a Core Word, TRUE/FALSE/NIL, or a name the program
// itself defines or binds (or an Example Word); prose and identifiers fail that.
import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, relative, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { lexProgram } from "./lexer.mjs";
import { BUSINESS } from "./business.mjs";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const CORE = new Set(JSON.parse(readFileSync(join(repo, "spec/words.json"), "utf8")).entries.map((e) => e.name));
for (const w of ["TRUE", "FALSE", "NIL"]) CORE.add(w);
const EXAMPLE_WORDS = ["SAY-HELLO", "SAY-WORLD", "SAY-BANG", "GREET"];

function isProgram(src, { minTokens = 1, needWord = true } = {}) {
  if (typeof src !== "string" || !src.trim() || src.length > 200000) return false;
  const lx = lexProgram(src); if (lx.error) return false;
  const t = lx.tokens; if (t.length < minTokens) return false;
  const defined = new Set(EXAMPLE_WORDS);
  for (let i = 0; i + 1 < t.length; i++) if (t[i].kind === "string" && t[i + 1].kind === "symbol" && /^(DEF|BIND)$/i.test(t[i + 1].value)) for (const n of t[i].value.split(/\s+/)) defined.add(n.toUpperCase());
  let words = 0;
  for (const k of t) {
    if (k.kind !== "symbol") continue;
    const u = k.value.toUpperCase();
    if (CORE.has(u)) { words++; continue; }
    if (defined.has(u)) continue;
    return false;
  }
  return !needWord || words > 0;
}

const walk = (dir, pred, acc = []) => { if (!existsSync(dir)) return acc; for (const f of readdirSync(dir)) { const p = join(dir, f); if (f === "node_modules" || f === "target" || f.startsWith(".")) continue; const s = statSync(p); if (s.isDirectory()) walk(p, pred, acc); else if (pred(p)) acc.push(p); } return acc; };

// --- string literal extraction ---------------------------------------------
function rustStrings(text) {
  const out = []; let i = 0;
  while (i < text.length) {
    const c = text[i];
    if (c === "/" && text[i + 1] === "/") { while (i < text.length && text[i] !== "\n") i++; continue; }
    if (c === "/" && text[i + 1] === "*") { const e = text.indexOf("*/", i + 2); i = e < 0 ? text.length : e + 2; continue; }
    if (c === "'" ) { // char literal or lifetime
      const m = /^'(\\.|[^\\'])'/s.exec(text.slice(i, i + 12)); if (m) { i += m[0].length; continue; }
      i++; continue;
    }
    const raw = /^(b?)r(#*)"/.exec(text.slice(i, i + 20));
    if (raw && (i === 0 || !/[A-Za-z0-9_]/.test(text[i - 1]))) {
      const hashes = raw[2]; const start = i + raw[0].length; const end = text.indexOf('"' + hashes, start);
      if (end < 0) break; if (!raw[1]) out.push({ s: text.slice(start, end), at: start }); i = end + 1 + hashes.length; continue;
    }
    if (c === '"' || (c === "b" && text[i + 1] === '"' && !/[A-Za-z0-9_]/.test(text[i - 1] ?? ""))) {
      const byte = c === "b"; let j = i + (byte ? 2 : 1); let s = "";
      while (j < text.length && text[j] !== '"') {
        if (text[j] === "\\") { const n = text[j + 1];
          if (n === "n") s += "\n"; else if (n === "t") s += "\t"; else if (n === "r") s += "\r"; else if (n === "0") s += "\0";
          else if (n === "\n") { j += 2; while (/\s/.test(text[j])) j++; continue; }
          else if (n === "u") { const m = /^\\u\{([0-9a-fA-F]+)\}/.exec(text.slice(j)); if (m) { s += String.fromCodePoint(parseInt(m[1], 16)); j += m[0].length; continue; } }
          else if (n === "x") { s += String.fromCharCode(parseInt(text.slice(j + 2, j + 4), 16)); j += 4; continue; }
          else s += n;
          j += 2; continue; }
        s += text[j]; j++;
      }
      if (!byte) out.push({ s, at: i }); i = j + 1; continue;
    }
    i++;
  }
  return out;
}
function jsStrings(text) {
  const out = []; let i = 0;
  while (i < text.length) {
    const c = text[i];
    if (c === "/" && text[i + 1] === "/") { while (i < text.length && text[i] !== "\n") i++; continue; }
    if (c === "/" && text[i + 1] === "*") { const e = text.indexOf("*/", i + 2); i = e < 0 ? text.length : e + 2; continue; }
    if (c === '"' || c === "'" || c === "`") {
      let j = i + 1; let s = ""; let interp = false;
      while (j < text.length && text[j] !== c) {
        if (text[j] === "\\") { const n = text[j + 1]; s += n === "n" ? "\n" : n === "t" ? "\t" : n; j += 2; continue; }
        if (c === "`" && text[j] === "$" && text[j + 1] === "{") interp = true;
        if (c !== "`" && text[j] === "\n") break;
        s += text[j]; j++;
      }
      if (!interp) out.push({ s, at: i }); i = j + 1; continue;
    }
    i++;
  }
  return out;
}
const lineOf = (text, at) => text.slice(0, at).split("\n").length;
const htmlText = (h) => h.replace(/<br\s*\/?>/g, "\n").replace(/<[^>]+>/g, "").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&#39;|&apos;/g, "'").replace(/&nbsp;/g, " ").replace(/&amp;/g, "&");

const corpus = []; const seen = new Set();
function add(kind, file, origin, program, line) {
  const key = kind + "\u0000" + program; if (seen.has(key)) return; seen.add(key);
  corpus.push({ id: corpus.length, kind, file, origin, line, program, lines: program.split("\n").length });
}
const rel = (p) => relative(repo, p);

// --- tests ------------------------------------------------------------------
for (const f of walk(join(repo, "rust"), (p) => p.endsWith(".rs"))) {
  const text = readFileSync(f, "utf8");
  const isTestFile = /_tests?\.rs$|\/tests\/|wasm-tests|test_support|proptest|laws\.rs$/.test(f);
  const testStart = isTestFile ? 0 : text.indexOf("#[cfg(test)]");
  if (testStart < 0) continue;
  for (const { s, at } of rustStrings(text)) if (at >= testStart && isProgram(s)) add("test", rel(f), "rust string literal", s, lineOf(text, at));
}
for (const f of walk(join(repo, "src"), (p) => /\.test\.ts$/.test(p)).concat(walk(join(repo, "tools/mcp-server"), (p) => /\.test\.js$|selftest\.js$|limit-cases\.js$/.test(p)))) {
  const text = readFileSync(f, "utf8");
  for (const { s, at } of jsStrings(text)) if (isProgram(s, { minTokens: 2 })) add("test", rel(f), "js string literal", s, lineOf(text, at));
}
const jsonSources = (file, kind, keys, origin) => {
  const j = JSON.parse(readFileSync(join(repo, file), "utf8"));
  const visit = (v, path) => { if (Array.isArray(v)) v.forEach((x, i) => visit(x, path + "/" + i)); else if (v && typeof v === "object") for (const [k, x] of Object.entries(v)) { if (keys.includes(k) && typeof x === "string") { if (isProgram(x)) add(kind, file, `${origin} ${path}/${k}`, x); } else visit(x, path + "/" + k); } };
  visit(j, "");
};
jsonSources("spec/outcome-witnesses.json", "test", ["source"], "witness");
jsonSources("tools/mcp-server/golden/cases.json", "test", ["source"], "golden");
jsonSources("tests/formatter-corpus.json", "test", ["input", "expected"], "formatter");

// --- examples ----------------------------------------------------------------
function markdown(file, origin) {
  const text = readFileSync(join(repo, file), "utf8");
  const fence = /```([a-zA-Z-]*)\n([\s\S]*?)```/g; let m;
  while ((m = fence.exec(text))) if (isProgram(m[2])) add("example", file, `${origin} fenced`, m[2].replace(/\n$/, ""), lineOf(text, m.index));
  const stripped = text.replace(/```[\s\S]*?```/g, "");
  const inl = /`([^`\n]+)`/g;
  while ((m = inl.exec(stripped))) if (isProgram(m[1], { minTokens: 3 })) add("example", file, `${origin} inline`, m[1]);
}
markdown("README.md", "README"); markdown("SKILL.md", "SKILL.md");
markdown("tools/mcp-server/assets/quickstart.md", "MCP quickstart"); markdown("tools/mcp-server/mcp-quickstart.md", "MCP quickstart");
markdown("tools/mcp-server/README.md", "MCP README"); markdown("docs/word-reference.md", "Word Reference (md)");
function htmlDoc(file, origin) {
  const text = readFileSync(join(repo, file), "utf8");
  // sample tables: the first <td><code> of each row inside <div class="sample">
  const sample = /<div class="sample">([\s\S]*?)<\/table>/g; let m;
  while ((m = sample.exec(text))) { const rows = m[1].match(/<tr>[\s\S]*?<\/tr>/g) ?? []; for (const r of rows) { const c = /<td><code>([\s\S]*?)<\/code><\/td>/.exec(r); if (c) { const s = htmlText(c[1]); if (isProgram(s)) add("example", file, `${origin} sample`, s, lineOf(text, m.index)); } } }
  const pre = /<pre[^>]*>([\s\S]*?)<\/pre>/g; while ((m = pre.exec(text))) { const s = htmlText(m[1]); if (isProgram(s)) add("example", file, `${origin} pre`, s, lineOf(text, m.index)); }
  const code = /<code>([\s\S]*?)<\/code>/g; while ((m = code.exec(text))) { const s = htmlText(m[1]); if (isProgram(s, { minTokens: 3 })) add("example", file, `${origin} inline`, s, lineOf(text, m.index)); }
}
htmlDoc("public/docs/en/index.html", "Reference (en)"); htmlDoc("public/docs/ja/index.html", "Reference (ja)"); htmlDoc("SPECIFICATION.html", "Specification");
for (const e of JSON.parse(readFileSync(join(repo, "spec/words.json"), "utf8")).entries) {
  const syn = e.documentation?.syntax; if (isProgram(syn)) add("example", "spec/words.json", `Word ${e.name} syntax`, syn);
  for (const m of (e.documentation?.summary ?? "").matchAll(/`([^`]+)`/g)) if (isProgram(m[1], { minTokens: 3 })) add("example", "spec/words.json", `Word ${e.name} summary`, m[1]);
}
for (const f of ["tools/mcp-server/eval/cases.json", "tools/mcp-server/eval/repair-cases.json", "tools/mcp-server/eval/number-baseline.json", "tools/mcp-server/eval/reference-repair-traces.json"]) jsonSources(f, "example", ["source", "repairedSource", "program", "code"], "MCP eval");
for (const f of walk(join(repo, "tools/mcp-server/eval/traces"), (p) => p.endsWith(".json"))) jsonSources(rel(f), "example", ["source", "program", "code"], "MCP trace");
for (const f of walk(join(repo, "tools/lexicon-emergence/tasks"), (p) => p.endsWith(".json"))) jsonSources(rel(f), "example", ["source", "program", "code", "solution", "reference"], "lexicon task");
add("example", "src/gui/interpreter-state-persistence.ts", "Playground Example Words",
  "[ 'Hello' PRINT ] 'SAY-HELLO' DEF\n[ 'World' PRINT ] 'SAY-WORLD' DEF\n[ '!' PRINT ] 'SAY-BANG' DEF\n[ SAY-HELLO SAY-WORLD SAY-BANG ] 'GREET' DEF\nGREET");

// --- bench / business ----------------------------------------------------------
for (const c of JSON.parse(readFileSync(join(repo, "scripts/bench/speed-bench-cases.json"), "utf8")).cases)
  add("bench", "scripts/bench/speed-bench-cases.json", `speed-bench ${c.name}`, `${c.setup}\n${c.source.repeat(c.repeat ?? 1)}`.trim());
for (const b of BUSINESS) add("business", b.file, b.origin, b.program);

writeFileSync(process.argv[2] ?? "corpus.json", JSON.stringify(corpus, null, 1));
const by = {}; for (const c of corpus) { const k = `${c.kind}\t${c.file}`; by[k] = by[k] ?? { n: 0, lines: 0 }; by[k].n++; by[k].lines += c.lines; }
for (const [k, v] of Object.entries(by).sort()) console.log(`${k}\t${v.n} programs\t${v.lines} lines`);
console.log(`total ${corpus.length}`);
